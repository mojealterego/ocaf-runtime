#!/usr/bin/env python3
"""Create and validate fail-closed evidence receipts for the agent fleet.

The collector records what is actually observable. It never upgrades a package
or stage to complete when a commit, build, test, digest, or receipt is missing.
"""
from __future__ import annotations

import argparse
import hashlib
import json
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


def validate(record: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if record.get("counts", {}).get("agents") != 111 or len(record.get("agents", [])) != 111:
        errors.append("agent count is not exactly 111")
    if record.get("counts", {}).get("swarms") != 33 or len(record.get("swarms", [])) != 33:
        errors.append("swarm count is not exactly 33")
    ids = {row.get("agent_id") for row in record.get("agents", [])}
    if len(ids) != 111:
        errors.append("agent IDs are not unique")
    for row in record.get("agents", []) + record.get("swarms", []):
        if row.get("evidence_status") == "COMPLETE_VERIFIED":
            missing = [key for key in REQUIRED if not row.get(key)]
            if missing:
                errors.append(f"{row.get('agent_id', row.get('swarm_id'))}: complete without {','.join(missing)}")
            if not str(row.get("artifact_digest", "")).startswith("sha256:"):
                errors.append(f"{row.get('agent_id', row.get('swarm_id'))}: invalid artifact digest")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("collect", "validate"))
    parser.add_argument("--output", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()
    if args.command == "collect":
        record = collect(args.output)
        print(json.dumps({"status": record["status"], "agents": 111, "swarms": 33, "output": str(args.output)}, ensure_ascii=False))
        return 0
    record = json.loads(args.output.read_text())
    errors = validate(record)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print("evidence_pipeline: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
