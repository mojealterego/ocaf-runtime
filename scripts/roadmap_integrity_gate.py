#!/usr/bin/env python3
"""Validate roadmap counts, immutable snapshot hashes and append-only IDs."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--repo", type=Path, default=Path("."))
    p.add_argument("--snapshot", type=Path)
    p.add_argument("--receipt", type=Path)
    args = p.parse_args()
    repo = args.repo
    manifest = json.loads((repo / "docs/roadmap/canonical_v11_inventory.json").read_text())
    detailed = manifest.get("main", [])
    assert manifest.get("main_count") == 333
    assert manifest.get("technical_count") == 64
    assert manifest.get("reference_count") == 33
    assert len(detailed) == 251
    ids = [str(x.get("id")) for x in detailed]
    assert len(ids) == len(set(ids)), "duplicate roadmap IDs"
    assert ids == sorted(ids, key=lambda x: tuple(int(y) if y.isdigit() else y for y in x.split(".")))
    if args.snapshot:
        snap = args.snapshot
        source_hash = snap / "source.sha256"
        assert source_hash.is_file(), source_hash
        expected = {}
        for line in source_hash.read_text().splitlines():
            digest, name = line.split("  ", 1)
            expected[name] = digest
        for name, digest in expected.items():
            current = repo / name
            assert current.is_file(), current
            assert sha256(current) == digest, f"snapshot mismatch: {name}"
    result = {
        "status": "PASS",
        "declared_main": 333,
        "detailed_main": 251,
        "technical": 64,
        "reference": 33,
        "append_only": True,
        "snapshot_checked": bool(args.snapshot),
    }
    if args.receipt:
        args.receipt.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
