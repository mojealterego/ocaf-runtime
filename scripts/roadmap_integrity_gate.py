#!/usr/bin/env python3
"""Fail-closed validation of the canonical roadmap and optional SHA-256 snapshot.

The ID check proves uniqueness and deterministic ordering, not that an earlier
revision was preserved. Historical append-only provenance needs a trusted base.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re


class IntegrityError(ValueError):
    """The input cannot establish the requested integrity evidence."""


COUNTS = {"main_count": 334, "technical_count": 64, "reference_count": 33}
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
STAGE_ID = re.compile(r"(?:0|[1-9][0-9]*)(?:\.[0-9]+)*\Z")
MAX_SNAPSHOT_BYTES = 128 * 1024
MAX_SNAPSHOT_ENTRIES = 4096


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _validate_manifest(repo: Path) -> None:
    path = repo / "docs/roadmap/canonical_v11_inventory.json"
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise IntegrityError("canonical roadmap is not an object")
    for key, required in COUNTS.items():
        value = manifest.get(key)
        if type(value) is not int or value != required:
            raise IntegrityError(f"canonical roadmap count mismatch: {key}")
    entries = manifest.get("main")
    if not isinstance(entries, list) or len(entries) != COUNTS["main_count"]:
        raise IntegrityError("canonical roadmap stage count mismatch")
    ids = []
    for entry in entries:
        identifier = entry.get("id") if isinstance(entry, dict) else None
        if not isinstance(identifier, str) or not STAGE_ID.fullmatch(identifier):
            raise IntegrityError("invalid or noncanonical roadmap stage ID")
        ids.append(tuple(int(component) for component in identifier.split(".")))
    if len(set(ids)) != len(ids):
        raise IntegrityError("duplicate or equivalent roadmap stage IDs")
    if ids != sorted(ids):
        raise IntegrityError("roadmap stages not in numeric order")


def _snapshot_path(repo: Path, relative: str) -> Path:
    if (not relative or "\\" in relative or "\x00" in relative
            or relative.startswith("/") or "//" in relative):
        raise IntegrityError("invalid snapshot file path")
    name = PurePosixPath(relative)
    if any(part in ("", ".", "..") for part in relative.split("/")):
        raise IntegrityError("snapshot path traversal")
    if name.is_absolute():
        raise IntegrityError("absolute snapshot file path")
    path = repo
    for component in name.parts:
        path = path / component
        if path.is_symlink():
            raise IntegrityError("symlink in snapshot file path")
    try:
        resolved = path.resolve(strict=True)
        if not resolved.is_relative_to(repo) or not path.is_file():
            raise IntegrityError("snapshot target is not a regular repo file")
    except (OSError, RuntimeError) as exc:
        raise IntegrityError("snapshot target unavailable") from exc
    return path


def _validate_snapshot(repo: Path, snapshot: Path) -> int:
    listing = snapshot / "source.sha256"
    if listing.is_symlink() or not listing.is_file():
        raise IntegrityError("snapshot checksum manifest missing or symlinked")
    size = listing.stat().st_size
    if not 0 < size <= MAX_SNAPSHOT_BYTES:
        raise IntegrityError("empty or oversized snapshot checksum manifest")
    lines = listing.read_text(encoding="utf-8").splitlines()
    if not 0 < len(lines) <= MAX_SNAPSHOT_ENTRIES:
        raise IntegrityError("invalid snapshot checksum record count")
    seen = set()
    for line in lines:
        if "  " not in line:
            raise IntegrityError("invalid snapshot checksum record")
        digest, relative = line.split("  ", 1)
        if not SHA256.fullmatch(digest) or relative in seen:
            raise IntegrityError("invalid or duplicate snapshot checksum record")
        seen.add(relative)
        target = _snapshot_path(repo, relative)
        if sha256(target) != digest:
            raise IntegrityError(f"snapshot mismatch: {relative}")
    return len(seen)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path("."))
    parser.add_argument("--snapshot", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    try:
        repo = args.repo.resolve(strict=True)
        if not repo.is_dir():
            raise IntegrityError("repository path is not a directory")
        _validate_manifest(repo)
        checked_files = _validate_snapshot(repo, args.snapshot) if args.snapshot else 0
    except (IntegrityError, OSError, UnicodeError, json.JSONDecodeError, RecursionError) as exc:
        print(json.dumps({"status": "FAIL", "reason": str(exc)}, sort_keys=True))
        return 1
    result = {
        "status": "PASS",
        "declared_main": COUNTS["main_count"],
        "detailed_main": COUNTS["main_count"],
        "technical": COUNTS["technical_count"],
        "reference": COUNTS["reference_count"],
        "append_only": True,  # Backward-compatible field: ordered, unique ID structure only.
        "historical_append_only_verified": False,
        "snapshot_checked": bool(args.snapshot),
        "snapshot_file_count": checked_files,
    }
    if args.receipt:
        args.receipt.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
