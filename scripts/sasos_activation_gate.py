#!/usr/bin/env python3
"""Stage 334: fail-closed semantic integrity gate for all 555 SASOS declarations.

This validates *claims*, not service execution. No catalog entry is activated
without a separately reviewed backend, integration tests and operator evidence.
"""
from __future__ import annotations

from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "omat/catalog/sasos_tools_001_555.json"
TOOL_ID = re.compile(r"SASOS_TOOL_([0-9]{3})_[A-Z0-9_]+\Z")
REQUIRED = {
    "number", "tool_id", "name", "description", "required_fields",
    "connected_modules", "declared_clearance", "declared_runtime", "status",
    "executable", "imandra_verified", "measured_latency_microseconds",
    "zero_copy_verified",
}


class CatalogIntegrityError(ValueError):
    pass


def audit_catalog(entries: object) -> dict:
    if not isinstance(entries, list) or len(entries) != 555:
        raise CatalogIntegrityError("expected exactly 555 catalog records")
    numbers: set[int] = set()
    identities: set[str] = set()
    runtimes: Counter[str] = Counter()
    for index, entry in enumerate(entries, 1):
        if not isinstance(entry, dict) or set(entry) != REQUIRED:
            raise CatalogIntegrityError(f"record {index}: missing/extra fields")
        number = entry["number"]
        tool_id = entry["tool_id"]
        if type(number) is not int or number != index:
            raise CatalogIntegrityError(f"record {index}: number sequence invalid")
        if not isinstance(tool_id, str) or (match := TOOL_ID.fullmatch(tool_id)) is None:
            raise CatalogIntegrityError(f"record {index}: invalid tool id")
        if int(match.group(1)) != number or number in numbers or tool_id in identities:
            raise CatalogIntegrityError(f"record {index}: identity mismatch or duplicate")
        numbers.add(number)
        identities.add(tool_id)
        for key in ("name", "description", "declared_clearance", "declared_runtime"):
            value = entry[key]
            if not isinstance(value, str) or not value.strip() or len(value) > 8192:
                raise CatalogIntegrityError(f"record {index}: invalid {key}")
        for key in ("required_fields", "connected_modules"):
            value = entry[key]
            if (not isinstance(value, list) or len(value) > 128
                    or any(not isinstance(v, str) or not v or len(v) > 256 for v in value)
                    or len(set(value)) != len(value)):
                raise CatalogIntegrityError(f"record {index}: invalid {key}")
        # An unimplemented adapter must never acquire a fabricated
        # execution receipt, latency measurement or formal proof flag.
        if (entry["status"] != "CATALOG_ONLY"
                or entry["executable"] is not False
                or entry["imandra_verified"] is not False
                or entry["measured_latency_microseconds"] is not None
                or entry["zero_copy_verified"] is not False):
            raise CatalogIntegrityError(
                f"record {index}: unsupported activation/proof/benchmark claim"
            )
        runtimes[entry["declared_runtime"]] += 1
    return {
        "status": "CATALOG_AUDITED_NOT_ACTIVATED",
        "declared_services": len(entries),
        "active_backends_verified": 0,
        "runtime_classes_declared": len(runtimes),
        "declared_runtime_counts": dict(sorted(runtimes.items())),
        "warning": "This gate verifies catalog integrity only; it does not execute services.",
    }


def main() -> int:
    raw = CATALOG.read_bytes()
    report = audit_catalog(json.loads(raw))
    report["catalog_sha256"] = hashlib.sha256(raw).hexdigest()
    print(json.dumps(report, sort_keys=True, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, json.JSONDecodeError, CatalogIntegrityError) as error:
        print(f"SASOS_CATALOG_GATE_FAIL: {error}", file=sys.stderr)
        sys.exit(1)
