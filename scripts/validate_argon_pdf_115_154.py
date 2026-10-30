#!/usr/bin/env python3
"""Validate all 40 Argon source requirements against the unchanged OCAF v11 stage IDs."""
from __future__ import annotations
import json
from pathlib import Path
import re
import sys


def validate(repo: Path) -> dict[str, object]:
    manifest = json.loads((repo / "docs/roadmap/argon_pdf_115_154_manifest.json").read_text(encoding="utf-8"))
    canonical = json.loads((repo / "docs/roadmap/canonical_v11_inventory.json").read_text(encoding="utf-8"))
    expected = [f"PDF-1.{i:02d}" for i in range(15, 55)]
    rows = manifest["requirements"]
    assert [row["source_id"] for row in rows] == expected, "PDF source IDs missing, duplicated or reordered"
    assert len(rows) == manifest["source_requirements_count"] == 40
    assert manifest["production_approved"] is False and manifest["canonical_stage_ids_unchanged"] is True
    assert canonical["main_count"] == 334 and canonical["technical_count"] == 64
    assert canonical["argon_pdf_115_154_reconciliation"]["source_requirements_count"] == 40
    stages = {item["id"] for item in canonical["main"]}
    for row in rows:
        assert row["status"] in {"PARTIAL_EXISTING", "PREFLIGHT_ONLY", "IMPLEMENTED_CI_PENDING", "OPEN"}
        assert row["complete"] is False and row["production_approved"] is False
        assert row["verified_ci"] == [], "Claimed CI must be separately checked and bound to commit SHA"
        assert set(row["canonical_candidates"]).issubset(stages)
        assert row["mapping"] == ("SEMANTIC_PROPOSED" if row["canonical_candidates"] else "UNMAPPED")
        for code in row["code_paths"]:
            assert (repo / code).is_file(), f"Missing claimed file: {code}"
    for roadmap_file in ["ROADMAP.md", "docs/ROADMAP_FULL_ECOSYSTEM_2026.md"]:
        content = (repo / roadmap_file).read_text(encoding="utf-8")
        split = content.split("<!-- ARGON_PDF_115_154_SOURCE_MATRIX -->")
        assert len(split) == 2, f"Missing source matrix: {roadmap_file}"
        present = re.findall(r"^\| (PDF-1\.\d{2}) \|", split[1], re.MULTILINE)
        assert present == expected, f"Wrong source IDs in {roadmap_file}"
        assert "production_approved=false" in split[1], "A source map cannot imply production GO"
    readme = (repo / "README.md").read_text(encoding="utf-8")
    assert "docs/roadmap/argon_pdf_115_154_manifest.json" in readme
    audit = (repo / "docs/audits/ARGON_115_154_RECONCILIATION_2026-10-30.md").read_text(encoding="utf-8")
    for source_id in expected:
        assert f"| {source_id} |" in audit
    return {"source_requirements": len(rows), "production_approved": False, "status": "SOURCE_REQUIREMENTS_RECONCILED"}


if __name__ == "__main__":
    print(json.dumps(validate(Path(sys.argv[1] if len(sys.argv) > 1 else ".")), sort_keys=True))
