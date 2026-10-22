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
RELEASE_MANIFEST = "RELEASE-MANIFEST.json"
FILES = frozenset((ARCHIVE, SBOM, RELEASE_MANIFEST))
BINARIES = frozenset((
    "ocaf-coordinator", "ocaf-cluster-health", "ocaf-rpc-node", "ocaf-effect-sink"
))
CHECKSUM_LINE = re.compile(r"([0-9a-f]{64})  ([a-zA-Z0-9_.-]+)")
MAX_ARCHIVE = 256 * 1024 * 1024
MAX_SBOM = 16 * 1024 * 1024
MAX_BINARY = 128 * 1024 * 1024
MAX_RELEASE_MANIFEST = 16 * 1024
GIT_SHA = re.compile(r"[0-9a-f]{40}\Z")
REPOSITORY = re.compile(r"[A-Za-z0-9_.-]{1,100}/[A-Za-z0-9_.-]{1,100}\Z")


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


def _unique_fields(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise BundleError("duplicate JSON field in release metadata")
        result[key] = value
    return result


def _source_binding(root: Path, expected_sha: str, expected_repository: str,
                    checksums: dict[str, str]) -> dict[str, Any]:
    if GIT_SHA.fullmatch(expected_sha) is None:
        raise BundleError("expected source SHA is invalid")
    if REPOSITORY.fullmatch(expected_repository) is None:
        raise BundleError("expected repository identity is invalid")
    path = root / RELEASE_MANIFEST
    _regular_file(path, MAX_RELEASE_MANIFEST)
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_unique_fields)
    except (OSError, ValueError, UnicodeError) as error:
        raise BundleError("invalid release source manifest") from error
    if not isinstance(manifest, dict) or set(manifest) != {"schema_version", "source", "subjects"}:
        raise BundleError("invalid release source manifest schema")
    if type(manifest["schema_version"]) is not int or manifest["schema_version"] != 1:
        raise BundleError("unsupported release source manifest version")
    source = manifest["source"]
    required_source = {
        "git_sha": expected_sha,
        "ref": "refs/heads/main",
        "repository": expected_repository,
        "workflow": ".github/workflows/supply-chain.yml",
    }
    if not isinstance(source, dict) or source != required_source:
        raise BundleError("release source identity mismatch")
    subjects = manifest["subjects"]
    expected_subjects = {
        name: {"sha256": checksums[name]} for name in (ARCHIVE, SBOM)
    }
    if not isinstance(subjects, dict) or subjects != expected_subjects:
        raise BundleError("release subject binding mismatch")
    return source


def verify_bundle(directory: Path, *, expected_sha: str,
                  expected_repository: str) -> dict[str, Any]:
    """Check archive, SBOM, and checksum manifest; never extract files."""
    root = directory.absolute()
    if not root.is_dir():
        raise BundleError("bundle directory does not exist")
    _regular_file(root / CHECKSUMS, 2048)
    _regular_file(root / ARCHIVE, MAX_ARCHIVE)
    _regular_file(root / SBOM, MAX_SBOM)
    _regular_file(root / RELEASE_MANIFEST, MAX_RELEASE_MANIFEST)

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

    source = _source_binding(root, expected_sha, expected_repository, expected)

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
        "source": source,
        "scope": "checksums-and-structure-only; signature-and-provenance-verification-required",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", type=Path, required=True)
    parser.add_argument("--expected-sha", required=True)
    parser.add_argument("--expected-repository", required=True)
    args = parser.parse_args(argv)
    try:
        result = verify_bundle(
            args.dir,
            expected_sha=args.expected_sha,
            expected_repository=args.expected_repository,
        )
        status = 0
    except BundleError as error:
        result = {"status": "FAIL", "reason": str(error)}
        status = 1
    print(json.dumps(result, sort_keys=True))
    return status


if __name__ == "__main__":
    sys.exit(main())
