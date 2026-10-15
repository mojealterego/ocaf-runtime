#!/usr/bin/env python3
"""Fail-closed source-traceability validation for OCAF Sovereign Workspace v9.

Checks 40 contiguous NEW main stages 1.55..1.94 and 16 technical tasks
from the user-supplied PDF. Roadmap acceptance is not implementation proof.
"""
import argparse
import json
from pathlib import Path
import re

MAIN = [f"1.{i:02d}" for i in range(55,95)]
TECH = ([f"0.39.{i}" for i in range(26,30)]
        +[f"0.40.{i}" for i in range(11,15)]
        +[f"0.41.{i}" for i in range(11,15)]
        +[f"0.42.{i}" for i in range(1,5)])
IDS = MAIN + TECH
CRATES = ["ocaf-transport-crdt","ocaf-telemetry-ebpf","ocaf-cognition-rqgm",
          "ocaf-economy-gov","ocaf-domain-swarms","ocaf-master-node"]
FIELDS = {"id","name","scope","acceptance","negative","dependencies",
          "pdf_pages","priority","owner","status","evidence","source_file"}
DEP = re.compile(r"(?:0\.\d{2}(?:\.\d+)?|1\.\d{2})\Z")
VALID_OWNER = re.compile(r"[a-z][a-z0-9_-]{1,35}\Z")
SHA = re.compile(r"[a-f0-9]{40}\Z")
SHA256 = re.compile(r"[a-f0-9]{64}\Z")
CI_URL = re.compile(r"https://github\.com/mojealterego/ocaf-runtime/actions/runs/[0-9]+\Z")


class InvalidRoadmap(ValueError):
    pass


def _unique_keys(pairs):
    out = {}
    for key,val in pairs:
        if key in out:
            raise InvalidRoadmap("duplicate JSON field")
        out[key]=val
    return out


def decode(raw):
    if not isinstance(raw,bytes) or not 100<len(raw)<=500000:
        raise InvalidRoadmap("invalid JSON document size")
    try:
        value=json.loads(raw,object_pairs_hook=_unique_keys,
            parse_constant=lambda _:(_ for _ in ()).throw(InvalidRoadmap("nonfinite JSON")))
    except (ValueError,TypeError,UnicodeError) as exc:
        raise InvalidRoadmap("JSON not valid and unique") from exc
    if not isinstance(value,dict):
        raise InvalidRoadmap("object required")
    return value


def validate(manifest,markdown):
    if (
        not isinstance(manifest,dict)
        or type(manifest.get("schema_version")) is not int or manifest["schema_version"]!=1
        or manifest.get("roadmap_id")!="OCAF_SOVEREIGN_WORKSPACE_V9_2026_10_15"
        or manifest.get("status")!="PROPOSED_NOT_IMPLEMENTED"
        or manifest.get("source_pdf")!="Dokument bez nazwy(3).PDF"
        or type(manifest.get("source_pdf_pages_with_content")) is not int
        or manifest["source_pdf_pages_with_content"]!=13
        or manifest.get("repo")!="mojealterego/ocaf-runtime"
        or manifest.get("branch")!="main"
        or manifest.get("production_approved") is not False
        or manifest.get("main_stage_ids_range")!="1.55-1.94"
        or type(manifest.get("main_stages")) is not int or manifest["main_stages"]!=40
        or type(manifest.get("technical_substages")) is not int
        or manifest["technical_substages"]!=16
        or type(manifest.get("prior_main_stage_identifiers_in_range")) is not int
        or manifest["prior_main_stage_identifiers_in_range"]!=155
        or type(manifest.get("new_main_stage_identifiers_in_range")) is not int
        or manifest["new_main_stage_identifiers_in_range"]!=195
        or manifest.get("proposed_cargo_crates")!=CRATES
        or not isinstance(manifest.get("items"),list)
        or not isinstance(manifest.get("architecture_research_steps"),list)
        or not isinstance(markdown,str)
    ):
        raise InvalidRoadmap("invalid planning source, identity, claims, or six-crate structure")
    phases=manifest["architecture_research_steps"]
    if len(phases)!=8:
        raise InvalidRoadmap("all eight phases required")
    for i,step in enumerate(phases):
        if (not isinstance(step,dict) or set(step)!={"step","slug","title","stage_start","stage_end","source_pdf_pages"}
            or type(step["step"]) is not int or step["step"]!=i+1
            or step["stage_start"]!=MAIN[i*5]
            or step["stage_end"]!=MAIN[i*5+4]
            or not isinstance(step["slug"],str) or not step["slug"]
            or not isinstance(step["title"],str) or len(step["title"])<8
            or not isinstance(step["source_pdf_pages"],list)
            or not step["source_pdf_pages"]
            or any(type(p) is not int or not 1<=p<=13 for p in step["source_pdf_pages"])):
            raise InvalidRoadmap("research methodology phase absent or broken")
    rows=manifest["items"]
    if len(rows)!=56 or [x.get("id") if isinstance(x,dict) else None for x in rows]!=IDS:
        raise InvalidRoadmap("stage IDs are missing, duplicated or not contiguous")
    for r in rows:
        if set(r)!=FIELDS:
            raise InvalidRoadmap("incomplete or unapproved stage item fields")
        key=r["id"]
        if (r["source_file"]!="Dokument bez nazwy(3).PDF"
            or any(not isinstance(r.get(k),str) or len(r[k].strip())<20
                   for k in ("scope","acceptance","negative"))
            or not isinstance(r["name"],str) or len(r["name"].strip())<6
            or r["priority"] not in ("P0","P1","P2")
            or not isinstance(r["owner"],str) or VALID_OWNER.fullmatch(r["owner"]) is None
            or not isinstance(r["dependencies"],list) or not r["dependencies"]
            or any(not isinstance(dep,str) or DEP.fullmatch(dep) is None or dep==key
                   for dep in r["dependencies"])
            or len(set(r["dependencies"]))!=len(r["dependencies"])
            or not isinstance(r["pdf_pages"],list) or not r["pdf_pages"]
            or any(type(p) is not int or not 1<=p<=13 for p in r["pdf_pages"])
            or r["status"] not in ("PLANNED","IN_PROGRESS","COMPLETE")
            or not isinstance(r["evidence"],list)):
            raise InvalidRoadmap(key+": missing genuine scope, dependencies, source or negative test")
        if r["status"]=="PLANNED":
            if r["evidence"]:
                raise InvalidRoadmap(key+": planned task cannot carry fabricated evidence")
        else:
            if not r["evidence"]:
                raise InvalidRoadmap(key+": progress requires real CI run evidence")
            for evidence in r["evidence"]:
                if (not isinstance(evidence,dict)
                    or set(evidence)!={"commit_sha","ci_run_url","artifact_sha256"}
                    or not isinstance(evidence["commit_sha"],str)
                    or SHA.fullmatch(evidence["commit_sha"]) is None
                    or not isinstance(evidence["ci_run_url"],str)
                    or CI_URL.fullmatch(evidence["ci_run_url"]) is None
                    or not isinstance(evidence["artifact_sha256"],str)
                    or SHA256.fullmatch(evidence["artifact_sha256"]) is None):
                    raise InvalidRoadmap(key+": ungrounded completed claim")
        # Validate actual visible table row, not a mere mentioned number in paragraphs.
        needle=r"^\| \*\*"+re.escape(key)+r"\*\* \|"
        if len(re.findall(needle,markdown,re.MULTILINE))!=1 or r["name"] not in markdown:
            raise InvalidRoadmap(key+": markdown table differs from machine manifest")
    for crate in CRATES:
        if f"`{crate}`" not in markdown:
            raise InvalidRoadmap("Cargo crate absent from human-readable roadmap")
    return {"status":"OCAF_V9_EIGHT_RESEARCH_PHASES_56_STAGES_VALID",
            "source_pdf":"Dokument bez nazwy(3).PDF",
            "new_main_stages":40,"new_technical_substages":16,
            "eight_research_phases":8,"proposed_crates":6,
            "main_ids_in_range":195,"features_implemented_in_this_extension":0,
            "production_approved":False}


def main():
    p=argparse.ArgumentParser()
    p.add_argument("--manifest",type=Path,required=True)
    p.add_argument("--markdown",type=Path,required=True)
    a=p.parse_args()
    try:
        d=validate(decode(a.manifest.read_bytes()),a.markdown.read_text(encoding="utf-8"))
    except (OSError,UnicodeError,InvalidRoadmap) as exc:
        p.error(str(exc))
    print(json.dumps(d,sort_keys=True))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
