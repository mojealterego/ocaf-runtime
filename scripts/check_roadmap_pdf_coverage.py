#!/usr/bin/env python3
"""Fail-closed static consistency check for the 20-PDF OCAF roadmap reconciliation.

Does not claim that planned features are implemented or that every PDF assertion is true.
Run: python3 scripts/check_roadmap_pdf_coverage.py
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs/roadmap/canonical_v11_inventory.json"
ROADMAP = ROOT / "ROADMAP.md"
MIRROR = ROOT / "docs/ROADMAP_FULL_ECOSYSTEM_2026.md"
README = ROOT / "README.md"
AUDIT = ROOT / "docs/audits/OCAF_20_PDF_DOCUMENTATION_AUDIT_2026-10-15.md"

SOURCE_PDFS = [
    "Analiza AGI I Ludzkich AI (1).PDF",
    "Analiza Architektury Systemu OCAF (1).PDF",
    "Analiza Architektury Systemu OCAF(1).PDF",
    "Analiza Architektury Systemu OCAF(2).PDF",
    "Analiza Architektury Systemu OCAF(3).PDF",
    "Analiza Architektury Systemu OCAF(4).PDF",
    "Analiza Architektury Systemu OCAF(5).PDF",
    "Analiza Architektury Systemu OCAF.PDF",
    "Architektura OCAF w Języku Rust (1).PDF",
    "Architektura OCAF w Języku Rust(1).PDF",
    "Architektura OCAF w Języku Rust.PDF",
    "Architektura OCAF w Języku RustPDF_261012_042941.pdf",
    "Architektura OmniContext Agent Framework (OCAF) w .. (1).PDF",
    "Dokument bez nazwy (2).PDF",
    "Dokument bez nazwy (6).PDF",
    "Kompleksowa Roadmapa Wdrożenia Ekosystemu OCAF w Rust.PDF",
    "Projekt i Mapa Drogowa Rewolucyjnej Architektury S.. (1).PDF",
    "Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(1) (1).PDF",
    "Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(2) (1).PDF",
    "Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(3).PDF"
]

REQUIRED = {
    "1.95": "Hyperon",
    "1.96": "ACT-R",
    "1.97": "MemGPT",
    "1.98": "DSPy",
    "1.99": "SWE-bench",
    "2.00": "Taskweaving",
    "2.01": "AI Scientist",
    "2.02": "OpenHands",
    "2.03": "Theory-of-Mind",
}

def fail(message: str) -> None:
    raise AssertionError(message)

def main() -> None:
    inventory = json.loads(MANIFEST.read_text(encoding="utf-8"))
    main_stages = inventory["main"]
    technical = inventory["technical"]
    ids = [item["id"] for item in main_stages]
    if len(ids) != len(set(ids)):
        fail("duplicate main-stage IDs")
    if len(ids) != inventory["main_count"] or len(technical) != inventory["technical_count"]:
        fail("manifest counts mismatch")
    if inventory.get("production_approved") is not False:
        fail("production_approved cannot be true without independent evidence")
    if ids[-1] != "2.03" or inventory.get("next_new_main_stage") != "2.04":
        fail("unexpected last/next main-stage ID")
    for sid, keyword in REQUIRED.items():
        stage = next((s for s in main_stages if s["id"] == sid), None)
        if stage is None:
            fail(f"missing stage {sid}")
        combined = " ".join(str(stage.get(k, "")) for k in ("title", "scope", "acceptance"))
        if keyword.casefold() not in combined.casefold():
            fail(f"missing {keyword} in {sid}")
        if stage.get("status") != "PLANNED":
            fail(f"stage {sid} must remain PLANNED until real acceptance evidence exists")
        if not stage.get("pdf_file") or not stage.get("pdf_pages"):
            fail(f"stage {sid} lacks source PDF provenance")
        if stage.get("evidence"):
            fail(f"stage {sid} has unverified implementation evidence")
    canonical = ROADMAP.read_text(encoding="utf-8")
    mirror = MIRROR.read_text(encoding="utf-8")
    if canonical != mirror:
        fail("roadmap mirror differs from canonical file")
    readme = README.read_text(encoding="utf-8")
    row_re = re.compile(r"^[|] [*][*]((?:0|1|2)[.][0-9]{2})[*][*] [|]", re.M)
    for name, content in (("ROADMAP", canonical), ("README", readme)):
        rows = row_re.findall(content)
        if len(rows) != len(main_stages) or rows != ids:
            fail(f"{name}: stage rows missing, duplicated or reordered")
        for sid in REQUIRED:
            if sid not in rows:
                fail(f"{name}: missing PDF-recovered stage {sid}")
    if "278" not in readme or "204" not in readme:
        fail("README total does not reflect 204+64+10=278")
    audit = AUDIT.read_text(encoding="utf-8")
    if len(SOURCE_PDFS) != 20 or len(set(SOURCE_PDFS)) != 20:
        fail("source PDF inventory must contain 20 distinct filenames")
    for name in SOURCE_PDFS:
        if name not in audit:
            fail(f"missing source PDF in audit: {name}")
    if "307" not in audit:
        fail("source audit does not record 307 PDF pages")
    for sid in REQUIRED:
        if sid not in audit:
            fail(f"audit missing trace for {sid}")
    print("PASS: 204 main stages, 64 technical, 10 builder products; 9 PDF-recovered stages traced")
    print("NOTE: static roadmap consistency only; not code/CI/production certification")

if __name__ == "__main__":
    try:
        main()
    except (AssertionError, KeyError, OSError, ValueError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        sys.exit(1)
