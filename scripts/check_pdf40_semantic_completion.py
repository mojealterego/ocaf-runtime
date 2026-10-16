#!/usr/bin/env python3
"""Fail-closed audit gate for 40-PDF semantic reconciliation.

Input JSON is a reviewed requirement register, not a keyword-matching corpus.
Never interpret a source-page index as semantic acceptance.
"""
import argparse
import json
import pathlib
import sys

REQUIRED = {"document_id", "page", "requirement_id", "source_quote",
            "classification", "decision", "roadmap_stage", "rationale", "reviewer"}
CLASSES = {"NORMATIVE", "BACKGROUND", "EXAMPLE", "DUPLICATE"}
DECISIONS = {"COVERED", "GAP_ADDED", "NOT_APPLICABLE", "SUPERSEDED"}
def check(index_path, reviewed_path):
    pages = [json.loads(line) for line in pathlib.Path(index_path).read_text(encoding="utf-8").splitlines() if line.strip()]
    if len(pages) != 902:
        raise ValueError(f"expected 902 pages, got {len(pages)}")
    keys = [(str(p["document_no"]), int(p["page"])) for p in pages]
    if len(set(keys)) != len(keys) or len(set(d for d, _ in keys)) != 40:
        raise ValueError("duplicate pages or document count != 40")
    data = json.loads(pathlib.Path(reviewed_path).read_text(encoding="utf-8"))
    records = data.get("requirements", [])
    reviewed_pages = {tuple((str(x[0]), int(x[1]))) for x in data.get("reviewed_pages", [])}
    if reviewed_pages != set(keys):
        raise ValueError(f"unreviewed pages: {len(set(keys) - reviewed_pages)}; unexpected: {len(reviewed_pages - set(keys))}")
    if not records:
        raise ValueError("no atomic reviewed requirements")
    ids = set()
    for r in records:
        missing = REQUIRED - set(r)
        if missing:
            raise ValueError(f"missing required fields: {sorted(missing)}")
        rid = r["requirement_id"]
        if rid in ids:
            raise ValueError(f"duplicate requirement_id {rid}")
        ids.add(rid)
        key = (str(r["document_id"]), int(r["page"]))
        if key not in reviewed_pages:
            raise ValueError(f"unknown source page: {rid}")
        if not all(str(r[k]).strip() for k in REQUIRED - {"page"}):
            raise ValueError(f"empty evidence field: {rid}")
        if r["classification"] not in CLASSES or r["decision"] not in DECISIONS:
            raise ValueError(f"unresolved classification or decision: {rid}")
        if r["classification"] == "NORMATIVE" and r["decision"] in {"COVERED", "GAP_ADDED"}:
            if not r.get("test_evidence") or not r.get("acceptance_criteria"):
                raise ValueError(f"missing normative evidence: {rid}")
        if r["decision"] == "GAP_ADDED" and not r.get("roadmap_commit_sha"):
            raise ValueError(f"missing roadmap commit for gap: {rid}")
    return len(records)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--pages", required=True)
    parser.add_argument("--reviewed", required=True)
    args = parser.parse_args()
    try:
        n = check(args.pages, args.reviewed)
    except (ValueError, KeyError, TypeError, OSError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    print(f"PASS: 902 pages, 40 documents, {n} adjudicated atomic requirements")
    return 0

if __name__ == "__main__":
    sys.exit(main())
