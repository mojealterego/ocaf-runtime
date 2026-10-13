#!/usr/bin/env python3
"""Verify the integrity and basic shape of an OCAF CI release bundle.

Cryptographic signing/provenance verification is intentionally delegated to
`gh attestation verify`; local SHA256SUMS alone do not authenticate a signer.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import sys
import tarfile
from pathlib import Path
from typing import Any

ARCHIVE = "ocaf_runtime-linux-x86_64.tar.gz"
SBOM = "ocaf_runtime.cdx.json"
CHECKSUMS = "SHA256SUMS"
FILES = frozenset((ARCHIVE, SBOM))
BINARIES = frozenset((
    "ocaf-coordinator", "ocaf-cluster-health", "ocaf-rpc-node", "ocaf-effect-sink"
))
CHECKSUM_LINE = re.compile(r"([0-9a-f]{64})  ([a-zA-Z0-9_.-]+)")
MAX_ARCHIVE = 256 * 1024 * 1024
MAX_SBOM = 16 * 1024 * 1024
MAX_BINARY = 128 * 1024 * 1024


class BundleError(ValueError):
    """Untrusted, damaged, or incomplete release evidence."""


def _regular_file(path: Path, max_size: int) -> None:
    try:
        info = path.lstat()
    except OSError as error:
        raise BundleError(f"required file missing: {path.name}") from error
    if not stat.S_ISREG(info.st_mode) or not (0 < info.st_size <= max_size):
        raise BundleError(f"file is not a bounded regular file: {path.name}")


def _checksum(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as content:
            for block in iter(lambda: content.read(1024 * 1024), b""):
                digest.update(block)
    except OSError as error:
        raise BundleError(f"cannot hash {path.name}") from error
    return digest.hexdigest()


def verify_bundle(directory: Path) -> dict[str, Any]:
    """Check archive, SBOM, and checksum manifest; never extract files."""
    root = directory.absolute()
    if not root.is_dir():
        raise BundleError("bundle directory does not exist")
    _regular_file(root / CHECKSUMS, 2048)
    _regular_file(root / ARCHIVE, MAX_ARCHIVE)
    _regular_file(root / SBOM, MAX_SBOM)

    try:
        records = (root / CHECKSUMS).read_text(encoding="ascii").splitlines()
    except (OSError, UnicodeError) as error:
        raise BundleError("invalid SHA256SUMS encoding") from error
    expected: dict[str, str] = {}
    for line in records:
        match = CHECKSUM_LINE.fullmatch(line)
        if match is None or match.group(2) in expected:
            raise BundleError("malformed or duplicated checksum entry")
        expected[match.group(2)] = match.group(1)
    if set(expected) != FILES:
        raise BundleError("checksum manifest must list exactly the release archive and SBOM")
    for filename in sorted(FILES):
        if _checksum(root / filename) != expected[filename]:
            raise BundleError(f"checksum mismatch: {filename}")

    try:
        with (root / SBOM).open("r", encoding="utf-8") as content:
            bom = json.load(content)
    except (OSError, ValueError, UnicodeError) as error:
        raise BundleError("unparseable CycloneDX SBOM") from error
    if not isinstance(bom, dict) or bom.get("bomFormat") != "CycloneDX" or bom.get("specVersion") != "1.5":
        raise BundleError("unexpected SBOM format or specification version")
    metadata = bom.get("metadata")
    component = metadata.get("component") if isinstance(metadata, dict) else None
    if not isinstance(component, dict) or component.get("name") != "ocaf_runtime":
        raise BundleError("SBOM describes a different root project")
    components = bom.get("components")
    if not isinstance(components, list) or not components or any(
        not isinstance(item, dict) or not isinstance(item.get("name"), str) or not item["name"]
        for item in components
    ):
        raise BundleError("SBOM has no valid dependency components")

    try:
        with tarfile.open(root / ARCHIVE, mode="r:gz") as archive:
            members = archive.getmembers()
            if len(members) != len(BINARIES):
                raise BundleError("release archive has missing or extra members")
            names: set[str] = set()
            total_size = 0
            for item in members:
                if item.name not in BINARIES or item.name in names or not item.isfile():
                    raise BundleError("unexpected, duplicated, or non-regular archive member")
                if not (0 < item.size <= MAX_BINARY) or item.mode & 0o111 == 0:
                    raise BundleError("non-executable or oversized release binary")
                names.add(item.name)
                total_size += item.size
            if names != BINARIES or total_size > MAX_ARCHIVE:
                raise BundleError("release archive binary inventory mismatch")
    except (OSError, EOFError, tarfile.TarError) as error:
        raise BundleError("invalid compressed release archive") from error

    return {
        "status": "PASS",
        "sha256": dict(sorted(expected.items())),
        "binaries": sorted(BINARIES),
        "sbom_components": len(components),
        "scope": "checksums-and-structure-only; signature-and-provenance-verification-required",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        result = verify_bundle(args.dir)
        status = 0
    except BundleError as error:
        result = {"status": "FAIL", "reason": str(error)}
        status = 1
    print(json.dumps(result, sort_keys=True))
    return status


if __name__ == "__main__":
    sys.exit(main())
