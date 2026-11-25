#!/usr/bin/env python3
"""Fail-closed source consistency gate for OCAF's 525-record roadmap matrix.

This verifies source traceability, NOT implementation, tests, or production GO.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
from typing import Any

SOURCES = {
    "canonical": "docs/roadmap/canonical_v11_inventory.json",
    "evolution": "docs/roadmap/agents_builder_om_evolution_v1.json",
    "products": "docs/roadmap/agents_builder_products_v11.json",
    "gaps": "docs/roadmap/missing_capabilities_v11.json",
}
MATRIX = "docs/roadmap/unfinished_requirement_matrix_2026_11_26.json"
COUNTS = {
    "main_stage": 334,
    "technical_stage": 64,
    "reference_adaptation": 33,
    "om_evolution_requirement": 20,
    "agents_builder_variant": 10,
    "missing_capability": 64,
}
SHA1 = re.compile(r"[0-9a-f]{40}\Z")
MAX_JSON_BYTES = 4 * 1024 * 1024


class ScopeError(ValueError):
    """Source material cannot substantiate this traceability record."""


def _unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ScopeError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read(root: Path, name: str) -> tuple[dict[str, Any], bytes]:
    path = root / name
    try:
        if path.is_symlink() or not path.is_file():
            raise ScopeError(f"missing or symlinked input: {name}")
        size = path.stat().st_size
        if size <= 0 or size > MAX_JSON_BYTES:
            raise ScopeError(f"unsafe JSON size: {name}")
        raw = path.read_bytes()
        if not 0 < len(raw) <= MAX_JSON_BYTES:
            raise ScopeError(f"unsafe JSON read: {name}")
        result = json.loads(raw, object_pairs_hook=_unique)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ScopeError(f"cannot read valid JSON: {name}") from exc
    if not isinstance(result, dict):
        raise ScopeError(f"not a JSON object: {name}")
    return result, raw


def _git_blob_sha(raw: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(raw)).encode("ascii") + b"\0" + raw).hexdigest()


def _expected(data: dict[str, Any]) -> list[tuple[str, str, str, dict[str, Any], str]]:
    c, e, p, g = (data[k] for k in SOURCES)
    result: list[tuple[str, str, str, dict[str, Any], str]] = []
    def append(kind: str, rows: Any, manifest: str, id_field: str, title_field: str) -> None:
        if not isinstance(rows, list) or len(rows) != COUNTS[kind]:
            raise ScopeError(f"invalid source count: {kind}")
        for index, row in enumerate(rows, 1):
            if not isinstance(row, dict):
                raise ScopeError(f"invalid source record: {kind}")
            rid, title = row.get(id_field), row.get(title_field)
            if not isinstance(rid, str) or not rid or not isinstance(title, str) or not title:
                raise ScopeError(f"missing source identity: {kind}")
            key = (f"reference_adaptation:{index:03d}:{rid}" if kind == "reference_adaptation"
                   else f"{kind}:{rid}")
            result.append((key, rid, title, row, manifest))
    append("main_stage", c.get("main"), SOURCES["canonical"], "id", "title")
    append("technical_stage", c.get("technical"), SOURCES["canonical"], "id", "title")
    append("reference_adaptation", c.get("reference_repositories"), SOURCES["canonical"], "name", "name")
    append("om_evolution_requirement", e.get("requirements"), SOURCES["evolution"], "id", "title")
    append("agents_builder_variant", p.get("products"), SOURCES["products"], "id", "product")
    append("missing_capability", g.get("items"), SOURCES["gaps"], "id", "component")
    return result


def validate(root: Path) -> dict[str, Any]:
    root = root.resolve(strict=True)
    matrix, _ = _read(root, MATRIX)
    if (matrix.get("schema_version") != 1 or matrix.get("repository") != "mojealterego/ocaf-runtime"
            or matrix.get("branch") != "main" or matrix.get("production_approved") is not False
            or matrix.get("source_overlaps_expected") is not True
            or matrix.get("unique_engineering_work_items_not_claimed") is not True):
        raise ScopeError("invalid matrix identity or unsafe acceptance flags")
    if (not isinstance(matrix.get("source_commit"), str)
            or not SHA1.fullmatch(matrix["source_commit"])):
        raise ScopeError("invalid source commit SHA")
    pinned = matrix.get("source_manifest_blobs")
    if not isinstance(pinned, dict) or set(pinned) != set(SOURCES.values()):
        raise ScopeError("source manifest set drift")
    source: dict[str, Any] = {}
    for key, path in SOURCES.items():
        raw_data, raw_bytes = _read(root, path)
        digest = pinned[path]
        if not isinstance(digest, str) or not SHA1.fullmatch(digest) or _git_blob_sha(raw_bytes) != digest:
            raise ScopeError(f"source Git blob drift: {path}")
        source[key] = raw_data
    expected = _expected(source)
    records = matrix.get("records")
    if (not isinstance(records, list) or len(records) != len(expected)
            or matrix.get("total_source_records") != len(expected)
            or matrix.get("counts") != COUNTS):
        raise ScopeError("matrix count or classification drift")
    seen: set[str] = set()
    classifications: Counter[str] = Counter()
    for row, (key, rid, title, raw, path) in zip(records, expected, strict=True):
        if not isinstance(row, dict) or key in seen:
            raise ScopeError("invalid or duplicate traceability record")
        seen.add(key)
        kind = key.split(":", 1)[0]
        classifications[kind] += 1
        if (row.get("record_key") != key or row.get("kind") != kind
                or row.get("id") != rid or row.get("title") != title
                or row.get("source_manifest") != path
                or row.get("declared_status") != (raw.get("status") or None)
                or row.get("acceptance_source_text") != (raw.get("acceptance") or raw.get("real_acceptance") or raw.get("test_scope") or None)
                or row.get("scope_source_text") != (raw.get("scope") or raw.get("implementation_targets") or None)
                or row.get("dependencies") != (raw.get("dependencies") or [])
                or row.get("implementation_verified_by_this_matrix") is not False
                or row.get("tests_executed_by_this_matrix") is not False
                or row.get("release_approved_by_this_matrix") is not False):
            raise ScopeError(f"source traceability mismatch: {key}")
    if dict(classifications) != COUNTS:
        raise ScopeError("traceability classification mismatch")
    return {"status": "PASS_SOURCE_TRACEABILITY_ONLY", "source_commit": matrix["source_commit"],
            "records_verified": len(records), "counts": COUNTS,
            "production_approved": False, "implementation_tested": False}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path("."))
    args = parser.parse_args()
    try:
        result = validate(args.repo)
    except (OSError, ScopeError, RecursionError, ValueError) as exc:
        print(json.dumps({"status": "FAIL", "reason": str(exc), "production_approved": False}, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
