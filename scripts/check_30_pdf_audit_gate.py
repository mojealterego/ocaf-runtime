#!/usr/bin/env python3
"""Fail-closed OCAF PDF audit gate. Standard library only.

This gate validates the canonical roadmap and published audit provenance.
It deliberately does NOT certify source-code compliance or production readiness.
"""
from __future__ import annotations
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
AUDIT = ROOT / "docs/audits/OCAF_30_PDF_FINAL_SOURCE_AUDIT_2026-10-16.md"
CROSSWALK = ROOT / "docs/audits/OCAF_BATCH2_CHAPTER_TO_ROADMAP_MATRIX_2026-10-16.md"
MANIFEST = ROOT / "docs/roadmap/canonical_v11_inventory.json"
ROADMAP = ROOT / "ROADMAP.md"
MIRROR = ROOT / "docs/ROADMAP_FULL_ECOSYSTEM_2026.md"
README = ROOT / "README.md"
MAIN_RE = re.compile(r"^\| \*\*((?:0|1|2)\.\d{2})\*\* \|", re.MULTILINE)

def check() -> None:
    inventory = json.loads(MANIFEST.read_text(encoding="utf-8"))
    main = inventory["main"]
    ids = [stage["id"] for stage in main]
    assert len(ids) == len(set(ids)) == inventory["main_count"] == 207, "main IDs/count mismatch"
    assert len(inventory["technical"]) == inventory["technical_count"] == 64, "technical count mismatch"
    assert inventory["next_new_main_stage"] == "2.07", "next stage changed unexpectedly"
    assert inventory["production_approved"] is False, "unapproved production gate bypass"
    roadmap = ROADMAP.read_text(encoding="utf-8")
    assert roadmap == MIRROR.read_text(encoding="utf-8"), "roadmap mirror diverged"
    assert MAIN_RE.findall(roadmap) == ids, "roadmap stage IDs diverged"
    assert MAIN_RE.findall(README.read_text(encoding="utf-8")) == ids, "README stage IDs diverged"
    assert "281" in roadmap and "207" in roadmap, "stale roadmap counts"
    audit = AUDIT.read_text(encoding="utf-8")
    for token in ("30", "535", "26", "532", "205", "NIEZAMKNIĘTY"):
        assert token in audit, f"missing audit provenance/status: {token}"
    crosswalk = CROSSWALK.read_text(encoding="utf-8")
    rows = re.findall(r"^\|\s*\d+\s*\|.*?KOD NIEPOTWIERDZONY\s*\|$", crosswalk, re.MULTILINE)
    assert len(rows) == 29, f"expected 29 scoped chapter rows, got {len(rows)}"
    stage_set = set(ids)
    for row in rows:
        cells = [c.strip() for c in row.strip("|").split("|")]
        referenced = re.findall(r"\b(?:0|1|2)\.\d{2}\b", cells[3])
        assert referenced and set(referenced) <= stage_set, f"unknown stage in crosswalk: {row}"
    print("PASS: roadmap identity and PDF audit provenance; production remains NOT APPROVED")
    print("NOT TESTED: cargo, security, infrastructure, source-to-requirement implementation")

if __name__ == "__main__":
    try:
        check()
    except (AssertionError, OSError, KeyError, ValueError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
