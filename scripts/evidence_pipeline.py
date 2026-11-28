#!/usr/bin/env python3
"""Create and validate fail-closed evidence receipts for the agent fleet.

The collector records what is actually observable. It never upgrades a package
or stage to complete when a commit, build, test, digest, or receipt is missing.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
AGENTS = ROOT / "docs/agents/agent_registry_111.json"
SWARMS = ROOT / "docs/agents/swarm_registry_33.json"
DEFAULT_OUT = ROOT / "docs/evidence/FLEET_EVIDENCE_BASELINE_2026-10-21.json"
REQUIRED = ("source_commit", "build_command", "build_result", "test_command", "test_result", "artifact_digest", "receipt_id")


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for block in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(block)
    return f"sha256:{h.hexdigest()}"


def git_commit() -> tuple[str | None, str | None]:
    try:
        p = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=True)
        return p.stdout.strip(), None
    except (OSError, subprocess.CalledProcessError) as exc:
        return None, str(exc)


def load_registry(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def collect(out: Path) -> dict[str, Any]:
    agents = load_registry(AGENTS)
    swarms = load_registry(SWARMS)
    commit, commit_error = git_commit()
    agent_rows = []
    for item in agents["agents"]:
        agent_rows.append({
            "agent_id": item["agent_id"],
            "swarm_id": item["group_id"],
            "lifecycle": item.get("lifecycle", "DRAFT"),
            "deployment_status": item.get("deployment_status", "NOT_DEPLOYED"),
            "evidence_status": "PENDING_EVIDENCE",
            "source_commit": commit,
            "build_command": "NOT_RUN",
            "build_result": "NOT_RUN",
            "test_command": "NOT_RUN",
            "test_result": "NOT_RUN",
            "artifact_digest": None,
            "receipt_id": None,
            "blockers": ["agent build not executed", "agent tests not executed", "artifact digest missing", "independent receipt missing"],
        })
    swarm_rows = []
    for item in swarms["swarms"]:
        swarm_rows.append({
            "swarm_id": item["swarm_id"],
            "members": item["members"],
            "lifecycle": item.get("lifecycle", "DRAFT"),
            "evidence_status": "PENDING_EVIDENCE",
            "source_commit": commit,
            "build_command": "NOT_RUN",
            "build_result": "NOT_RUN",
            "test_command": "NOT_RUN",
            "test_result": "NOT_RUN",
            "artifact_digest": None,
            "receipt_id": None,
            "blockers": ["swarm health/build receipt missing"],
        })
    record = {
        "schema_version": 1,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "status": "PENDING_EVIDENCE",
        "truth_rule": "COMPLETE_VERIFIED requires source_commit, build, test, artifact_digest and receipt_id",
        "git": {"commit": commit, "error": commit_error},
        "counts": {"agents": len(agent_rows), "swarms": len(swarm_rows), "active_runtime_agents": agents.get("active_runtime_agent_count", 0), "active_runtime_swarms": swarms.get("active_runtime_swarm_count", 0)},
        "agents": agent_rows,
        "swarms": swarm_rows,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n")
    return record


def _present_text(value: Any) -> bool:
    return (isinstance(value, str) and bool(value.strip())
            and value.strip().upper() not in {"NOT_RUN", "N/A", "NONE", "UNKNOWN", "PENDING", "BLOCKED", "FAIL"})


def validate(record: dict[str, Any]) -> list[str]:
    """Validate evidence structure only; never authenticate receipts or execute commands."""
    if not isinstance(record, dict):
        return ["record must be an object"]
    errors: list[str] = []
    if type(record.get("schema_version")) is not int or record["schema_version"] != 1:
        errors.append("unsupported evidence schema")
    status = record.get("status")
    if status not in ("PENDING_EVIDENCE", "COMPLETE_VERIFIED"):
        errors.append("invalid record evidence status")
    counts = record.get("counts")
    if not isinstance(counts, dict):
        return errors + ["counts must be an object"]
    for kind, identity, prefix, width, count in (
        ("agents", "agent_id", "OCAF-AGENT-", 3, 111),
        ("swarms", "swarm_id", "OCAF-SWARM-", 2, 33),
    ):
        rows = record.get(kind)
        if (type(counts.get(kind)) is not int or counts[kind] != count
                or not isinstance(rows, list) or len(rows) != count):
            errors.append(f"{kind} count is not exactly {count}")
            continue
        expected = {f"{prefix}{index:0{width}d}" for index in range(1, count + 1)}
        seen: set[str] = set()
        for index, row in enumerate(rows):
            if not isinstance(row, dict):
                errors.append(f"{kind}[{index}]: expected object")
                continue
            rid = row.get(identity)
            if not isinstance(rid, str) or rid not in expected or rid in seen:
                errors.append(f"{kind}[{index}]: unknown or duplicate identity")
                continue
            seen.add(rid)
            row_status = row.get("evidence_status")
            if row_status not in ("PENDING_EVIDENCE", "COMPLETE_VERIFIED"):
                errors.append(f"{rid}: invalid evidence status")
            if status == "COMPLETE_VERIFIED" and row_status != "COMPLETE_VERIFIED":
                errors.append(f"{rid}: summary claims completion without completed row")
            if row_status != "COMPLETE_VERIFIED":
                continue
            for key in REQUIRED:
                if not _present_text(row.get(key)):
                    errors.append(f"{rid}: missing or invalid {key}")
            for key in ("build_result", "test_result"):
                if row.get(key) != "PASS":
                    errors.append(f"{rid}: {key} must be PASS")
            for key, pattern in (("source_commit", r"[0-9a-f]{40}"),
                                 ("artifact_digest", r"sha256:[0-9a-f]{64}")):
                value = row.get(key)
                if not isinstance(value, str) or re.fullmatch(pattern, value) is None:
                    errors.append(f"{rid}: invalid {key}")
            if row.get("blockers") != []:
                errors.append(f"{rid}: completion requires empty blockers list")
        if seen != expected:
            errors.append(f"{kind}: canonical identity set mismatch")
    return errors


def _unique_json(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key")
        result[key] = value
    return result


def _reject_constant(value: str) -> Any:
    raise ValueError("non-finite JSON constant")


def read_evidence(path: Path) -> Any:
    limit = 4 * 1024 * 1024
    with path.open("rb") as stream:
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise ValueError("evidence exceeds size limit")
    return json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_json,
                      parse_constant=_reject_constant)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("collect", "validate"))
    parser.add_argument("--output", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()
    if args.command == "collect":
        record = collect(args.output)
        print(json.dumps({"status": record["status"], "agents": 111, "swarms": 33, "output": str(args.output)}, ensure_ascii=False))
        return 0
    try:
        record = read_evidence(args.output)
        errors = validate(record)
    except (OSError, ValueError, RecursionError) as exc:
        print(f"evidence_pipeline: invalid input: {exc}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print("evidence_pipeline: PASS_STRUCTURE_ONLY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
