#!/usr/bin/env python3
"""OCAF v11: FULL root-pinned roadmap + original 10 Agents Builder variants.

Hard-fail on loss of any of the 195 main stages, 64 technical substages,
33 upstreams, or 10 original Code/NoCode products. No unsupported completion
or code-execution claim is made by passing this documentation validator.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import re

import validate_roadmap_canonical as v10

class CanonicalV11Error(ValueError):
    pass

PLATFORMS = {"Cloud": "0.52", "Windows": "0.53", "macOS": "0.53",
             "Android": "0.54", "iOS": "0.54"}
MODES = ("Code", "NoCode")
BUILDER_STAGES = ("0.50", "0.51", "0.52", "0.53", "0.54", "0.55")
PROTECTED = {"main", "technical", "reference_repositories"}
LOST_COUNT = 24


def _assert(ok: bool, msg: str):
    if not ok:
        raise CanonicalV11Error(msg)


def validate(v11: dict, v10_data: dict, root: str, mirror: str, readme: str,
             builders: dict, losses: dict, ledger: str, archive: str) -> dict:
    _assert(root == mirror, "root ROADMAP.md must be identical to docs mirror")
    _assert(readme.startswith(
        "> **📌 [PEŁNA ROADMAPA OCAF OD SAMEGO POCZĄTKU — ROADMAP.md](ROADMAP.md)**"),
        "first line of README must pin the canonical root roadmap")
    _assert(
        isinstance(v11, dict)
        and v11.get("schema_version") == 1
        and v11.get("version") == "v11-original-builder-scope-restored"
        and v11.get("repository") == "mojealterego/ocaf-runtime"
        and v11.get("branch") == "main"
        and v11.get("production_approved") is False
        and v11.get("canonical_roadmap") == "ROADMAP.md"
        and v11.get("docs_mirror") == "docs/ROADMAP_FULL_ECOSYSTEM_2026.md"
        and v11.get("next_new_main_stage") == "1.95",
        "v11 provenance, canonical root path, next milestone or safety claim invalid")
    _assert(
        isinstance(v10_data,dict) and v10_data.get("version")=="v10-restored-full",
        "canonical v10 source must remain untouched")
    for group in PROTECTED:
        old=v10_data.get(group)
        new=v11.get(group)
        _assert(isinstance(old,list) and isinstance(new,list) and len(old)==len(new),
                "history count changed in "+group)
        if group!="main":
            _assert(old==new,"historical substage or reference evidence mutated: "+group)
    _assert(v11.get("main_count")==195 and v11.get("technical_count")==64
            and v11.get("reference_count")==33,
            "roadmap stage count changed despite ONLY restoring old products")
    for old,new in zip(v10_data["main"],v11["main"]):
        _assert(old.get("id")==new.get("id"),"historic stage IDs reordered")
        if old["id"] in BUILDER_STAGES:
            _assert(new.get("previous_title")==old.get("title")
                    and new.get("previous_scope")==old.get("scope"),
                    "historical builder scope lost rather than preserved")
            _assert(new.get("status")==old.get("status")=="PLANNED_OR_UNVERIFIED",
                    "builder stage falsely certified complete")
            _assert(new.get("builder_products_manifest")
                    =="docs/roadmap/agents_builder_products_v11.json",
                    "wrong original builder source mapping")
            _assert(
                "AGENTS BUILDER" in new.get("title","")
                and "Code" in new.get("scope","")
                and "NoCode" in new.get("scope","")
                and len(new.get("negative",""))>40
                and len(new.get("acceptance",""))>40,
                "builder scope still collapsed, untestable or confused with agents")
        else:
            _assert(old==new,"non-builder historical milestone altered: "+old["id"])
    # 10 distinct PRODUCTS, not agents, each with Code and NoCode mode.
    _assert(isinstance(builders,dict)
            and builders.get("schema_version")==1
            and builders.get("category")=="AGENTS_BUILDER_PRODUCTS_NOT_AGENTS"
            and builders.get("branch")=="main"
            and builders.get("new_main_milestones_added")==0
            and builders.get("legacy_main_milestones_preserved")==195
            and builders.get("legacy_technical_substages_preserved")==64
            and builders.get("next_new_main_milestone")=="1.95",
            "ten original product scope/origin/numbering absent")
    products=builders.get("products")
    _assert(isinstance(products,list) and len(products)==10,"exactly 10 original builder products required")
    seen=set()
    for p in products:
        _assert(isinstance(p,dict)
                and p.get("platform") in PLATFORMS
                and p.get("mode") in MODES
                and p.get("canonical_core_stage")=="0.50"
                and p.get("mode_stage")=="0.51"
                and p.get("platform_stage")==PLATFORMS[p["platform"]]
                and p.get("safety_stage")=="0.55",
                "builder platform or original parent stage missing")
        pair=(p["platform"],p["mode"])
        _assert(pair not in seen,"duplicate Agents Builder product mode")
        seen.add(pair)
        label=f"Agents Builder { {'Cloud':'Chmurowy','Windows':'Desktopowy Windows','macOS':'Desktopowy macOS','Android':'Android','iOS':'iOS'}[p['platform']] } {p['mode']}"
        _assert(p.get("product")==label,"incorrect original Agents Builder product title")
        product_id=f"AB-{p['platform'].upper()}-{p['mode'].upper()}"
        _assert(p.get("id")==product_id,"invalid product unique ID")
        _assert(len(p.get("real_acceptance",""))>60 and len(p.get("negative_acceptance",""))>50,
                "must include real platform acceptance and failure tests")
        _assert(p.get("production_approved") is False,
                "production cannot be inferred from product roadmap item")
        if p.get("status")=="NOT_IMPLEMENTED":
            _assert(p.get("implemented_verified") is False
                    and p.get("code_paths_found")==[]
                    and p.get("evidence")==[],
                    "unimplemented builder cannot claim shipped artifact")
        else:
            _assert(p.get("status") in ("PARTIAL","IMPLEMENTED_VERIFIED")
                    and p.get("evidence") and p.get("code_paths_found"),
                    "implementation cannot be credited without code and real evidence")
    _assert(seen==set((platform,mode) for platform in PLATFORMS for mode in MODES),
            "an original platform Code or NoCode variant is missing")
    _assert(all(isinstance(x,dict) and x.get("status")=="NOT_IMPLEMENTED"
                for x in builders.get("shared_requirements",[])),
            "shared Agents Builder source must not falsely claim completion")
    _assert(isinstance(losses,dict)
            and losses.get("version")=="v11-loss-audit"
            and len(losses.get("items",[]))==LOST_COUNT
            and [x.get("id") for x in losses["items"]]==[
                f"INV-{n:03}" for n in range(1,25)]
            and all(x.get("condition")=="MISSING" for x in losses["items"][:13]),
            "missed capability inventory incomplete or silently erased")
    _assert(root.count("## Najważniejsze: pierwotny AGENTS BUILDER")==1
            and root.count("## Obowiązkowy mechanizm utrzymania tej Roadmapy")==1,
            "full roadmap loses original builder scope or stage-end update policy")
    for p in products:
        _assert(root.count("| **"+p["product"]+"** |")==1,
                "missing, duplicated or renamed product in pinned root roadmap: "+p["id"])
    _assert(ledger.startswith("# OCAF — DZIENNIK KOŃCA ETAPU")
            and "RESTORED_EXISTING" in ledger and "0.50–0.55" in ledger
            and "STAGE_VERIFIED" in ledger
            and "NOWYCH" not in ledger[:250],
            "stage-end changelog/history and append-only rule missing")
    # Existing v10 checker also verifies every historical stage, source and
    # artifact row, preserving 0.00–0.10 unknown markers and operator limits.
    v10_result=v10.validate(v10_data,mirror,archive)
    _assert(v10_result["main"]==195 and v10_result["technical"]==64,
            "old milestone inventory failed no-loss regression")
    return {
        "status":"OCAF_V11_ORIGINAL_AGENTS_BUILDER_ALL_TEN_PRODUCTS_RESTORED",
        "canonical_root_roadmap_pinned":True,
        "main_stages":195,
        "technical_substages":64,
        "total_milestone_rows":259,
        "distinct_agents_builder_products":10,
        "builder_platforms":5,
        "code_and_nocode_for_every_platform":True,
        "scope_gaps_audited":24,
        "next_new_main_stage":"1.95",
        "fully_production_approved_builder_products":0,
        "production_approved":False,
    }


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo-root",type=Path,default=Path("."))
    opts=p.parse_args()
    repo=opts.repo_root
    try:
        def j(path):return json.loads((repo/path).read_text(encoding="utf-8"))
        result=validate(
            j("docs/roadmap/canonical_v11_inventory.json"),
            j("docs/roadmap/canonical_v10_inventory.json"),
            (repo/"ROADMAP.md").read_text(encoding="utf-8"),
            (repo/"docs/ROADMAP_FULL_ECOSYSTEM_2026.md").read_text(encoding="utf-8"),
            (repo/"README.md").read_text(encoding="utf-8"),
            j("docs/roadmap/agents_builder_products_v11.json"),
            j("docs/roadmap/missing_capabilities_v11.json"),
            (repo/"docs/roadmap/STAGE_CHANGELOG.md").read_text(encoding="utf-8"),
            (repo/"docs/archive/ROADMAP_FULL_ECOSYSTEM_v9_2026-10-15.md").read_text(encoding="utf-8"),
        )
    except (ValueError,OSError,UnicodeError) as exc:
        p.error(str(exc))
    print(json.dumps(result,sort_keys=True))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
