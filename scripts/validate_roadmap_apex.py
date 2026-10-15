#!/usr/bin/env python3
"""OCAF Roadmap Apex v8 gate: 52 source-traceable *planned*, not delivered, stages.

Enforce contiguous 40 new main IDs, 12 P0/P1 technical IDs, six PDF
architectural pillars through page traceability, explicit negative tests,
dependencies, non-duplicated markdown rows, and fail-closed evidence claims.
This does not verify any proposed technology or benchmark in production.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

EXPECTED_MAIN = tuple(f"1.{i:02d}" for i in range(15, 55))
EXPECTED_INFRA = tuple(
    [f"0.39.{i}" for i in range(22, 26)]
    + [f"0.40.{i}" for i in range(7, 11)]
    + [f"0.41.{i}" for i in range(7, 11)]
)
EXPECTED_IDS = EXPECTED_MAIN + EXPECTED_INFRA
EXPECTED_FIELDS = {
    "id", "title", "deliverable", "acceptance", "negative",
    "dependencies", "pdf_pages", "priority", "owner", "status",
    "evidence", "source_file",
}
DEP_RE = re.compile(r"(?:0\.\d{2}(?:\.\d+)?|1\.\d{2})\Z")
OWNER_RE = re.compile(r"[a-z][a-z0-9_-]{1,35}\Z")
ALLOWED_OWNERS = {
    "transport", "security", "edge", "memory", "observability", "agents",
    "privacy", "evaluation", "research", "code", "crypto", "finops",
    "payments", "runtime", "database", "reliability",
}


class RoadmapValidationError(ValueError):
    pass


def _reject_duplicate_pairs(items):
    out = {}
    for key, value in items:
        if key in out:
            raise RoadmapValidationError("duplicate JSON object key: " + str(key))
        out[key] = value
    return out


def parse_roadmap(raw: bytes) -> dict:
    if not isinstance(raw, bytes) or not 100 < len(raw) < 200000:
        raise RoadmapValidationError("roadmap JSON outside bounded range")
    try:
        value = json.loads(
            raw, object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda _: (_ for _ in ()).throw(
                RoadmapValidationError("nonfinite JSON denied")),
        )
    except (ValueError, TypeError, UnicodeError) as exc:
        raise RoadmapValidationError("invalid roadmap source JSON") from exc
    if not isinstance(value, dict):
        raise RoadmapValidationError("roadmap JSON root must be object")
    return value


def validate(manifest: dict, markdown: str) -> dict:
    if (
        not isinstance(manifest, dict)
        or type(manifest.get("schema_version")) is not int
        or manifest["schema_version"] != 1
        or manifest.get("roadmap_id") != "OCAF_APEX_TIER0_2026_10_15"
        or manifest.get("status") != "PROPOSED_NOT_IMPLEMENTED"
        or manifest.get("repo") != "mojealterego/ocaf-runtime"
        or manifest.get("branch") != "main"
        or manifest.get("source_pdf") != "Dokument bez nazwy(2).PDF"
        or type(manifest.get("source_pages")) is not int
        or manifest["source_pages"] != 15
        or manifest.get("main_stage_ids_range") != "1.15-1.54"
        or type(manifest.get("main_stages")) is not int
        or manifest["main_stages"] != 40
        or type(manifest.get("technical_substages")) is not int
        or manifest["technical_substages"] != 12
        or manifest.get("independent_audit_gate_waived_for_development") is not True
        or manifest.get("production_approved") is not False
        or not isinstance(manifest.get("items"), list)
        or not isinstance(markdown, str)
    ):
        raise RoadmapValidationError("invalid overall v8 scope, source or readiness claim")

    items = manifest["items"]
    if len(items) != 52:
        raise RoadmapValidationError("exactly 40 main and 12 technical tasks required")
    ids = [r.get("id") if isinstance(r, dict) else None for r in items]
    if ids != list(EXPECTED_IDS):
        raise RoadmapValidationError("noncontiguous, missing, extra or reordered stage ID")
    for row in items:
        ident = row["id"]
        if set(row) != EXPECTED_FIELDS:
            raise RoadmapValidationError(ident + ": missing or unapproved roadmap field")
        if not all(
            isinstance(row[key], str) and len(row[key].strip()) >= minimum
            for key, minimum in (
                ("title", 7), ("deliverable", 35),
                ("acceptance", 35), ("negative", 35)
            )
        ):
            raise RoadmapValidationError(ident + ": no actionable acceptance/negative test")
        if (
            row["source_file"] != "Dokument bez nazwy(2).PDF"
            or row["priority"] not in ("P0", "P1", "P2")
            or not isinstance(row["owner"], str)
            or OWNER_RE.fullmatch(row["owner"]) is None
            or row["owner"] not in ALLOWED_OWNERS
            or not isinstance(row["dependencies"], list)
            or not row["dependencies"]
            or len(set(map(str, row["dependencies"]))) != len(row["dependencies"])
            or any(not isinstance(dep, str) or DEP_RE.fullmatch(dep) is None
                   or dep == ident for dep in row["dependencies"])
            or not isinstance(row["pdf_pages"], list)
            or not row["pdf_pages"]
            or any(type(page) is not int or not 1 <= page <= 15
                   for page in row["pdf_pages"])
            or not isinstance(row["evidence"], list)
            or row["status"] not in ("PLANNED", "IN_PROGRESS", "COMPLETE")
        ):
            raise RoadmapValidationError(ident + ": invalid owner, dependency, source or state")
        # A later edit may promote stages only with hard evidence references.
        if row["status"] != "PLANNED":
            if not row["evidence"] or not all(
                isinstance(e, dict)
                and set(e) == {"commit_sha", "ci_run_url", "artifact_sha256"}
                and isinstance(e["commit_sha"], str)
                and re.fullmatch("[a-f0-9]{40}", e["commit_sha"])
                and isinstance(e["ci_run_url"], str)
                and re.fullmatch(
                    r"https://github\.com/mojealterego/ocaf-runtime/actions/runs/[0-9]+",
                    e["ci_run_url"])
                and isinstance(e["artifact_sha256"], str)
                and re.fullmatch("[a-f0-9]{64}", e["artifact_sha256"])
                for e in row["evidence"]
            ):
                raise RoadmapValidationError(ident + ": progress without verifiable CI pointers")
        elif row["evidence"]:
            raise RoadmapValidationError(ident + ": PLANNED stage has manufactured evidence")

        # Only count canonical table rows, never incidental prose mentions.
        marker = re.compile(r"^\| \*\*" + re.escape(ident) + r"\*\* \|", re.MULTILINE)
        if len(marker.findall(markdown)) != 1:
            raise RoadmapValidationError(ident + ": missing or duplicate row in readable roadmap")
        if row["title"] not in markdown:
            raise RoadmapValidationError(ident + ": human-readable roadmap out of sync")

    # Six pillars: Zenoh/CRDT/OTel; CoALA; RQGM; SDNN+PQC; guest Aya/FinOps; L402.
    blocks = (("1.15", "1.21"), ("1.22", "1.28"), ("1.29", "1.35"),
              ("1.36", "1.41"), ("1.42", "1.48"), ("1.49", "1.54"))
    if any(a not in ids or b not in ids for a, b in blocks):
        raise RoadmapValidationError("Apex architecture source pillar missing")
    return {
        "status": "ROADMAP_APEX_V8_TRACEABILITY_VALID",
        "main_stages_added": 40, "technical_substages_added": 12,
        "new_stage_rows": 52, "total_main_stage_identifier_range": "0.00-1.54",
        "main_stage_ids_in_range": 155,
        "architecture_pillars": 6,
        "features_implemented_by_this_roadmap_change": 0,
        "production_approved": False,
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--manifest", type=Path, required=True)
    p.add_argument("--markdown", type=Path, required=True)
    args = p.parse_args()
    try:
        result = validate(
            parse_roadmap(args.manifest.read_bytes()),
            args.markdown.read_text(encoding="utf-8"),
        )
    except (OSError, UnicodeError, RoadmapValidationError) as exc:
        p.error(str(exc))
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
