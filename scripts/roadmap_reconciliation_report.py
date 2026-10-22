#!/usr/bin/env python3
"""Report roadmap count drift without changing stage IDs or declaring completion."""
from __future__ import annotations

import json
from pathlib import Path


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    manifest = json.loads((root / "docs/roadmap/canonical_v11_inventory.json").read_text())
    detail = {str(entry["id"]) for entry in manifest.get("main", [])}
    declared = {str(i) for i in range(321, 334)}
    missing = sorted(declared - detail, key=int)
    result = {
        "status": "RECONCILIATION_REQUIRED" if missing else "NO_DECLARED_STAGE_GAP",
        "declared_main_count": manifest.get("main_count"),
        "detailed_main_count": len(detail),
        "technical_count": manifest.get("technical_count"),
        "reference_count": manifest.get("reference_count"),
        "declared_but_not_detailed": missing,
        "mutation": "none",
    }
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
