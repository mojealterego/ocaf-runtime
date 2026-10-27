#!/usr/bin/env python3
"""Fail-closed static Cargo.lock source/checksum inventory; no online audit."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
import stat
import sys
import tomllib
from pathlib import Path
from typing import Any

ALLOWED_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
CHECKSUM = re.compile(r"[0-9a-f]{64}")
MAX_MANIFEST_BYTES = 512 * 1024
MAX_LOCK_BYTES = 8 * 1024 * 1024
MAX_PACKAGES = 10_000
# Local workspace code has no registry checksum. Each path and identity is
# reviewed here so an unrelated local package cannot enter the locked graph.
REVIEWED_WORKSPACE_MEMBERS = {
    "crates/ocaf-stage-ledger": ("ocaf_stage_ledger", "0.1.0"),
    "crates/ocaf-transport-crdt": ("ocaf-transport-crdt", "0.1.0"),
}


class PolicyError(ValueError):
    """Invalid or untrusted dependency inventory."""


def _read_regular(path: Path, limit: int) -> bytes:
    try:
        metadata = path.lstat()
    except OSError as exc:
        raise PolicyError("required source file unavailable") from exc
    if not stat.S_ISREG(metadata.st_mode) or not 0 < metadata.st_size <= limit:
        raise PolicyError("source file must be a nonempty bounded regular file")
    try:
        with path.open("rb") as stream:
            blob = stream.read(limit + 1)
    except OSError as exc:
        raise PolicyError("cannot read source file") from exc
    if not 0 < len(blob) <= limit:
        raise PolicyError("source file exceeds permitted size")
    return blob


def _toml(blob: bytes) -> dict[str, Any]:
    try:
        doc = tomllib.loads(blob.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as exc:
        raise PolicyError("malformed TOML source") from exc
    if not isinstance(doc, dict):
        raise PolicyError("TOML document must be a table")
    return doc


def _workspace_members(root: Path, manifest: dict[str, Any]) -> dict[str, str]:
    """Return reviewed local package versions from an exact workspace layout."""
    workspace = manifest.get("workspace")
    if workspace is None:
        return {}
    if not isinstance(workspace, dict):
        raise PolicyError("invalid Cargo workspace definition")
    paths = workspace.get("members")
    if not isinstance(paths, list) or not paths or paths[0] != "." or \
            len(paths) != len(set(str(path) for path in paths)) or \
            workspace.get("default-members") != ["."]:
        raise PolicyError("unreviewed Cargo workspace members or defaults")
    if any(not isinstance(path, str) or (path != "." and path not in REVIEWED_WORKSPACE_MEMBERS)
           for path in paths):
        raise PolicyError("unreviewed Cargo workspace member")
    dependencies = manifest.get("dependencies", {})
    if not isinstance(dependencies, dict):
        raise PolicyError("invalid root dependency table")
    accepted: dict[str, str] = {}
    for path in paths[1:]:
        name, version = REVIEWED_WORKSPACE_MEMBERS[path]
        member_root = root / path
        for directory in (member_root.parent, member_root):
            try:
                mode = directory.lstat().st_mode
            except OSError as exc:
                raise PolicyError("reviewed workspace path is unavailable") from exc
            if not stat.S_ISDIR(mode):
                raise PolicyError("reviewed workspace path must be a directory")
        member = _toml(_read_regular(member_root / "Cargo.toml", MAX_MANIFEST_BYTES))
        package = member.get("package")
        if not isinstance(package, dict) or \
                (package.get("name"), package.get("version")) != (name, version) or \
                package.get("publish") is not False or package.get("license") != "MIT":
            raise PolicyError("reviewed workspace package identity or license changed")
        bound = [alias for alias, dep in dependencies.items()
                 if isinstance(dep, dict) and dep.get("path") == path and
                 dep.get("package", alias) == name and dep.get("version") == version]
        if len(bound) != 1:
            raise PolicyError("workspace member is not bound to the root package")
        accepted[name] = version
    return accepted


def inspect(repository: Path) -> dict[str, Any]:
    """Check locked identities and sources; not a registry/download verification."""
    root = repository.absolute()
    manifest = _toml(_read_regular(root / "Cargo.toml", MAX_MANIFEST_BYTES))
    package = manifest.get("package")
    if not isinstance(package, dict):
        raise PolicyError("Cargo.toml package metadata missing")
    name, version = package.get("name"), package.get("version")
    if not isinstance(name, str) or not name or not isinstance(version, str) or not version:
        raise PolicyError("invalid Cargo.toml package identity")
    workspace_members = _workspace_members(root, manifest)
    blob = _read_regular(root / "Cargo.lock", MAX_LOCK_BYTES)
    lock = _toml(blob)
    if type(lock.get("version")) is not int or lock["version"] != 4:
        raise PolicyError("unsupported Cargo.lock format")
    packages = lock.get("package")
    if not isinstance(packages, list) or not 2 <= len(packages) <= MAX_PACKAGES:
        raise PolicyError("locked dependency graph is absent or oversized")
    # One explicitly reviewed first-party patch may replace the pinned
    # zenoh-transport crate. No other local/third-party source is allowed.
    patches = manifest.get("patch", {}).get("crates-io", {})
    if not isinstance(patches, dict):
        raise PolicyError("malformed Cargo source patches")
    if set(patches) - {"zenoh-transport", "token-cell", "zenoh-config", "zenoh-link-tls", "zenoh-link-commons"}:
        raise PolicyError("unaudited Cargo source patch")
    patched_transport = patches.get("zenoh-transport")
    if patched_transport is not None:
        if not isinstance(patched_transport, dict) or patched_transport.get("path") != "vendor/zenoh-transport" or set(patched_transport) != {"path"}:
            raise PolicyError("untrusted Zenoh transport patch location")
        vendored = _toml(_read_regular(root / "vendor/zenoh-transport/Cargo.toml", MAX_MANIFEST_BYTES))
        meta = vendored.get("package")
        if not isinstance(meta, dict) or meta.get("name") != "zenoh-transport" or meta.get("version") != "1.10.1":
            raise PolicyError("unrecognized vendored Zenoh package")
        deps = vendored.get("dependencies", {})
        codec = deps.get("lz4_flex") if isinstance(deps, dict) else None
        codec_ver = codec if isinstance(codec, str) else codec.get("version") if isinstance(codec, dict) else None
        if codec_ver != "0.11.6":
            raise PolicyError("Zenoh must depend on patched lz4_flex 0.11.6")
    # Official token-cell >=3 uses maintained pastey already, but Zenoh
    # pins ^2. Pin a reviewable 2.1.1 compatibility backport instead
    # of silently forcing a potentially incompatible major version.
    token_patch = patches.get("token-cell")
    if token_patch is not None:
        if not isinstance(token_patch, dict) or token_patch != {"path": "vendor/token-cell"}:
            raise PolicyError("untrusted token-cell patch location")
        vendored_token = _toml(_read_regular(root / "vendor/token-cell/Cargo.toml", MAX_MANIFEST_BYTES))
        package_meta = vendored_token.get("package")
        if not isinstance(package_meta, dict) or package_meta.get("name") != "token-cell" or package_meta.get("version") != "2.1.1":
            raise PolicyError("unrecognized vendored token-cell package")
        dependencies = vendored_token.get("dependencies")
        alias = dependencies.get("paste") if isinstance(dependencies, dict) else None
        if not isinstance(alias, dict) or alias.get("version") != "0.2.3" or alias.get("package") != "pastey":
            raise PolicyError("token-cell must use the maintained pastey crate")
    hardened = ("zenoh-config", "zenoh-link-tls", "zenoh-link-commons")
    hardened_requested = {name_ for name_ in hardened if name_ in patches}
    if hardened_requested and hardened_requested != set(hardened):
        raise PolicyError("the config and TLS security backports must be applied together")
    for component in hardened_requested:
        override = patches.get(component)
        if override != {"path": f"vendor/{component}"}:
            raise PolicyError("unexpected Zenoh source override")
        vendor_root = root / "vendor" / component
        vendor_manifest = _toml(_read_regular(vendor_root / "Cargo.toml", MAX_MANIFEST_BYTES))
        pkg_info = vendor_manifest.get("package")
        if not isinstance(pkg_info, dict) or pkg_info.get("name") != component or pkg_info.get("version") != "1.10.1":
            raise PolicyError("unverified Zenoh component identity")
        deps = vendor_manifest.get("dependencies")
        if not isinstance(deps, dict):
            raise PolicyError("missing Zenoh component dependencies")
        if component == "zenoh-config":
            if "serde_yaml" in deps:
                raise PolicyError("deprecated YAML parser still required")
            yaml_source = _read_regular(vendor_root / "src/lib.rs", 2 * MAX_MANIFEST_BYTES)
            if b"YAML configuration disabled by OCAF" not in yaml_source or b"serde_yaml::" in yaml_source:
                raise PolicyError("unsafe YAML decoder not disabled")
            include_source = _read_regular(vendor_root / "src/include.rs", 2 * MAX_MANIFEST_BYTES)
            if b"YAML configuration disabled by OCAF" not in include_source or b"serde_yaml::" in include_source:
                raise PolicyError("unsafe YAML include decoder not disabled")
        else:
            if "rustls-pemfile" in deps:
                raise PolicyError("obsolete PEM decoder still required")
            source_path = ("src/utils.rs" if component == "zenoh-link-tls" else "src/quic/utils.rs")
            parser_source = _read_regular(vendor_root / source_path, 2 * MAX_MANIFEST_BYTES)
            if b"rustls_pemfile::" in parser_source or b"pem::PemObject" not in parser_source:
                raise PolicyError("obsolete PEM parser calls not replaced")
            if component == "zenoh-link-commons" and "dep:rustls-pemfile" in vendor_manifest.get("features", {}).get("quic", []):
                raise PolicyError("QUIC feature retains obsolete PEM dependency")
    seen: set[tuple[str, str, str]] = set()
    vendored_seen = 0
    registry_transport = 0
    lz4_safe = 0
    token_vendored = 0
    token_registry = 0
    pastey_safe = 0
    hardened_local = {name_: 0 for name_ in hardened}
    hardened_registry = {name_: 0 for name_ in hardened}
    deprecated_seen = set()
    roots = 0
    member_counts = {member: 0 for member in workspace_members}
    external = 0
    for pkg in packages:
        if not isinstance(pkg, dict):
            raise PolicyError("invalid locked package")
        pname, pversion = pkg.get("name"), pkg.get("version")
        if not isinstance(pname, str) or not pname or not isinstance(pversion, str) or not pversion:
            raise PolicyError("invalid locked package identity")
        source = pkg.get("source")
        if source is None:
            if pkg.get("checksum") is not None:
                raise PolicyError("local package may not claim a registry checksum")
            if (pname, pversion) == (name, version):
                roots += 1
                locked_dependencies = pkg.get("dependencies", [])
                if not isinstance(locked_dependencies, list) or any(
                    member not in locked_dependencies for member in workspace_members
                ):
                    raise PolicyError("root lockfile does not depend on workspace members")
            elif pname in workspace_members and workspace_members[pname] == pversion:
                member_counts[pname] += 1
            elif patched_transport is not None and (pname, pversion) == ("zenoh-transport", "1.10.1"):
                vendored_seen += 1
            elif token_patch is not None and (pname, pversion) == ("token-cell", "2.1.1"):
                token_vendored += 1
            elif pname in hardened_requested and pversion == "1.10.1":
                hardened_local[pname] += 1
            else:
                raise PolicyError("unexpected unverified local package")
            source = ""
        else:
            if source != ALLOWED_SOURCE:
                raise PolicyError("non-allowlisted registry or Git dependency")
            checksum = pkg.get("checksum")
            if not isinstance(checksum, str) or CHECKSUM.fullmatch(checksum) is None:
                raise PolicyError("invalid or missing registry checksum")
            if pname == "zenoh-transport":
                registry_transport += 1
            if pname == "token-cell":
                token_registry += 1
            if pname in hardened:
                hardened_registry[pname] += 1
            if pname in {"rustls-pemfile", "serde_yaml", "unsafe-libyaml"}:
                deprecated_seen.add(pname)
            if pname == "pastey" and pversion == "0.2.3":
                pastey_safe += 1
            if pname == "paste" and token_patch is not None:
                raise PolicyError("outdated paste dependency still in locked graph")
            if pname == "lz4_flex" and pversion == "0.11.6":
                lz4_safe += 1
            if pname == "lz4_flex" and (pversion.startswith("0.10.") or pversion in ("0.11.0", "0.11.1", "0.11.2", "0.11.3", "0.11.4", "0.11.5", "0.12.0")):
                if patched_transport is not None:
                    raise PolicyError("security-backport graph still includes a vulnerable LZ4 version")
            external += 1
        identity = (pname, pversion, source)
        if identity in seen:
            raise PolicyError("duplicate package identity in Cargo.lock")
        seen.add(identity)
    if roots != 1 or external == 0:
        raise PolicyError("expected root and locked crates.io dependencies not found")
    if any(count != 1 for count in member_counts.values()):
        raise PolicyError("reviewed workspace member is missing from Cargo.lock")
    if patched_transport is not None and (vendored_seen != 1 or registry_transport or lz4_safe != 1):
        raise PolicyError("patched Zenoh transport or safe LZ4 dependency is not exclusively locked")
    if token_patch is not None and (token_vendored != 1 or token_registry or pastey_safe != 1):
        raise PolicyError("token-cell is not exclusively patched to maintained pastey")
    if hardened_requested:
        if any(hardened_local[k] != 1 or hardened_registry[k] for k in hardened):
            raise PolicyError("an outdated Zenoh TLS/configuration component remains in the graph")
        if deprecated_seen:
            raise PolicyError("deprecated TLS/YAML dependencies remain in the graph")
    return {
        "status": "PASS",
        "lock_format": 4,
        "cargo_lock_sha256": hashlib.sha256(blob).hexdigest(),
        "registry_packages": external,
        "workspace_members": dict(sorted(workspace_members.items())),
        "vendored_zenoh_transport": vendored_seen == 1,
        "vendored_token_cell": token_vendored == 1,
        "migrated_zenoh_tls_config": bool(hardened_requested),
        "package_count": len(packages),
        "allowed_source": ALLOWED_SOURCE,
        "scope": "static-lockfile-integrity-not-a-vulnerability-or-license-audit",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        report = inspect(args.repo)
        code = 0
    except PolicyError as exc:
        report = {"status": "FAIL", "reason": str(exc)}
        code = 1
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return code


if __name__ == "__main__":
    sys.exit(main())
