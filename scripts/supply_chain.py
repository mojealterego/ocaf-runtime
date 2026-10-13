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
