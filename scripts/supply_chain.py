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
    seen: set[tuple[str, str, str]] = set()
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
            if pkg.get("checksum") is not None or (pname, pversion) != (name, version):
                raise PolicyError("unexpected unverified local package")
            roots += 1
            source = ""
        else:
            if source != ALLOWED_SOURCE:
                raise PolicyError("non-allowlisted registry or Git dependency")
            checksum = pkg.get("checksum")
            if not isinstance(checksum, str) or CHECKSUM.fullmatch(checksum) is None:
                raise PolicyError("invalid or missing registry checksum")
            external += 1
        identity = (pname, pversion, source)
        if identity in seen:
            raise PolicyError("duplicate package identity in Cargo.lock")
        seen.add(identity)
    if roots != 1 or external == 0:
        raise PolicyError("expected root and locked crates.io dependencies not found")
    return {
        "status": "PASS",
        "lock_format": 4,
        "cargo_lock_sha256": hashlib.sha256(blob).hexdigest(),
        "registry_packages": external,
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
