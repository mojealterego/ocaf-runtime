#!/usr/bin/env python3
"""OCAF canonical Roadmap v10: lossless full-scope fail-closed validator.

CI verifies all stage identifiers, every unique documented substage,
33 reference repositories, rendered markdown parity, preservation of the
previous 0.36–0.99 source and explicit unknown early-stage requirements.
This tool DOES NOT certify implementations, live services or production.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import re


class CanonicalRoadmapError(ValueError):
    pass


def _ids():
    return tuple(f"0.{i:02d}" for i in range(100)) + tuple(
        f"1.{i:02d}" for i in range(95))


def _stage_rows(content: str) -> list[str]:
    # Only the canonical execution matrix defines stages. Later PDF source
    # mappings may repeat an ID as a cross-reference (for example 1.98).
    matrix = content.split("## 3. Pełne 207 etapów głównych od początku", 1)
    if len(matrix) != 2:
        raise CanonicalRoadmapError("canonical execution matrix missing")
    matrix = matrix[1].split("## 5. Rejestr 33 repozytoriów", 1)[0]
    return re.findall(r"^\| \*\*((?:0|1)\.\d{2}(?:\.\d+)?)\*\* \|",
                      matrix, flags=re.MULTILINE)


def validate(inventory: dict, markdown: str, archive: str) -> dict:
    if (
        not isinstance(inventory,dict)
        or type(inventory.get("schema_version")) is not int
        or inventory["schema_version"]!=1
        or inventory.get("version")!="v10-restored-full"
        or inventory.get("repository")!="mojealterego/ocaf-runtime"
        or inventory.get("branch")!="main"
        or inventory.get("production_approved") is not False
        or type(inventory.get("main_count")) is not int
        or inventory["main_count"]!=195
        or type(inventory.get("technical_count")) is not int
        or inventory["technical_count"]!=64
        or type(inventory.get("reference_count")) is not int
        or inventory["reference_count"]!=33
        or inventory.get("legacy_archive")!="docs/archive/ROADMAP_FULL_ECOSYSTEM_v9_2026-10-15.md"
        or not isinstance(inventory.get("main"),list)
        or not isinstance(inventory.get("technical"),list)
        or not isinstance(inventory.get("reference_repositories"),list)
        or not isinstance(markdown,str) or not isinstance(archive,str)
    ):
        raise CanonicalRoadmapError("invalid canonical version, scope or production claim")
    main=inventory["main"]
    technical=inventory["technical"]
    refs=inventory["reference_repositories"]
    if (len(main)!=195 or tuple(r.get("id") for r in main)!=_ids()
        or len(technical)!=64 or len(refs)!=33):
        raise CanonicalRoadmapError("missing/reordered/unexpected main, substep or upstream")
    tids=[r.get("id") for r in technical]
    if len(set(tids))!=64 or not all(re.fullmatch(
        r"0\.(?:3[8-9]|4[0-2])\.\d{1,2}",str(t)) for t in tids):
        raise CanonicalRoadmapError("duplicated/invalid technical stage identifier")
    # Only known, cited substeps, not synthetic gaps in the numbering.
    for item in main+technical:
        if (not isinstance(item,dict)
            or not isinstance(item.get("id"),str)
            or not isinstance(item.get("title"),str)
            or not item["title"]
            or not isinstance(item.get("scope"),str)
            or not isinstance(item.get("acceptance"),str)
            or not isinstance(item.get("negative"),str)
            or not isinstance(item.get("status"),str)
            or not isinstance(item.get("source"),str)
            or not isinstance(item.get("dependencies"),list)):
            raise CanonicalRoadmapError("incomplete traceability, status or negative gate")
        if (item["id"].startswith("1.")
            and (item["status"]!="PLANNED" or not item["source"])):
            raise CanonicalRoadmapError("v7/v8/v9 source requirements cannot claim completion")
        if "PRODUCTION" in item["status"] and item["id"] not in ("0.38",):
            raise CanonicalRoadmapError("manufactured production pass")
    for i in range(11):
        item=main[i]
        if (item.get("status")!="UNSPECIFIED_IN_CURRENT_REPO"
            or item.get("source")!=""
            or item.get("title")!="Historyczny etap bez zachowanej definicji"):
            raise CanonicalRoadmapError("historical 0.00–0.10 description invented")
    if main[38].get("status")!="CLOSED_BY_OPERATOR_TECHNICAL_OPEN":
        raise CanonicalRoadmapError("administrative milestone must retain technical NO-GO")
    if main[39].get("status")!="CLOSED_BY_OPERATOR_SIMULATION_TECHNICAL_OPEN":
        raise CanonicalRoadmapError("0.39 administrative simulation closure must retain technical NO-GO")
    if any(main[i].get("status")!="ACTIVE_TECHNICAL_OPEN" for i in (40,41)):
        raise CanonicalRoadmapError("multihost PG/DR production gates still open")
    keys=[(r.get("name"),r.get("group")) for r in refs]
    if (len(set(keys))!=33
        or not all(isinstance(name,str) and name and
                   isinstance(group,str) and re.fullmatch(r"P(?:24|09)-\d{2}",group)
                   for name,group in keys)
        or sum(k[1].startswith("P24") for k in keys)!=24
        or sum(k[1].startswith("P09") for k in keys)!=9):
        raise CanonicalRoadmapError("24+9 references not fully preserved")
    rows=_stage_rows(markdown)
    expected=list(_ids())+tids
    # v10 is a protected historical subset of the current v11 matrix.
    # Additional, uniquely numbered v11 stages must not invalidate it.
    if (len(rows)!=len(set(rows)) or
        any(rows.count(stage_id)!=1 for stage_id in expected)):
        raise CanonicalRoadmapError("rendered roadmap drops, duplicates or renumbers stages")
    # Require every repository to appear as its own physical table row.
    rrows=re.findall(r"^\| \d+ \| `([^`]+)` \|",markdown,re.MULTILINE)
    if len(rrows)!=33 or set(rrows)!={k[0] for k in keys}:
        raise CanonicalRoadmapError("one or more source repositories missing in markdown")
    if (not archive.startswith("# OCAF FULL ECOSYSTEM ROADMAP")
        or "## Complete execution matrix" not in archive
        or "| 0.36 |" not in archive or "| 0.99 |" not in archive
        or "| 0.39.11 |" not in archive):
        raise CanonicalRoadmapError("original v9 full Roadmap execution history missing")
    required=(
        "0.39 — broker", "0.40 — inter-host PostgreSQL HA",
        "0.41 — odporność", "0.42 — release",
        "archive/ROADMAP_FULL_ECOSYSTEM_v9_2026-10-15.md",
        "ROADMAP_TWO_PDF_RUST_MODULES_2026-10-15.md",
        "ROADMAP_APEX_TIER0_2026-10-15.md",
        "ROADMAP_SOVEREIGN_WORKSPACE_V9_2026-10-15.md",
        "ROADMAP_33_REPOSITORY_ADAPTATIONS_2026-10-15.md",
    )
    if any(t not in markdown for t in required):
        raise CanonicalRoadmapError("critical dependency, gate or original scope disappeared")
    return {
        "status":"OCAF_ROADMAP_V10_FULL_SCOPE_RESTORED_AND_TRACEABLE",
        "main":195,"technical":64,"upstream":33,
        "total_milestones_and_substeps":259,
        "unspecified_early":11,
        "historical_archive_preserved":True,
        "production_approved":False,
    }


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--inventory",type=Path,required=True)
    p.add_argument("--roadmap",type=Path,required=True)
    p.add_argument("--legacy-archive",type=Path,required=True)
    a=p.parse_args()
    try:
        r=validate(json.loads(a.inventory.read_text(encoding="utf-8")),
                   a.roadmap.read_text(encoding="utf-8"),
                   a.legacy_archive.read_text(encoding="utf-8"))
    except (OSError,ValueError,UnicodeError) as exc:
        p.error(str(exc))
    print(json.dumps(r,sort_keys=True))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
