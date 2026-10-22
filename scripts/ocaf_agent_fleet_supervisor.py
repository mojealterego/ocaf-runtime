#!/usr/bin/env python3
"""Fail-closed local supervisor for 111 agents and 33 swarms.

This process validates individual manifests and swarm membership, writes an
append-only audit log, and emits health/registration decisions. It never starts
an LLM, enables OSTc, grants source access, or reports ACTIVE from a declaration.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_AGENT_ROOT = ROOT / "OCAF Agents MOJEALTEREGO" / "AGENT-FLEET-111"
DEFAULT_SWARM_ROOT = ROOT / "OCAF Agents MOJEALTEREGO" / "SWARM-FLEET-33"
DEFAULT_REGISTRY = ROOT / "docs" / "agents" / "agent_registry_111.json"
DEFAULT_LOG = Path("/tmp/ocaf-agent-fleet-supervisor.jsonl")
REQUIRED_AGENT_FILES = (
    "Cargo.toml", "src/main.rs", "src/lib.rs", "config/agent.toml",
    "config/autonomy.toml", "config/tool_policy.toml", "docs/README.md",
    "docs/ARCHITECTURE.md", "docs/USER_MANUAL.md", "tests/security.rs",
    "packaging/install.sh", "packaging/uninstall.sh",
    "packaging/INSTALLER_CONTRACT.md", "packaging/BUILD_EVIDENCE.md",
    "certification/CERTIFICATE_RECORD.md",
)


class SupervisorError(RuntimeError):
    pass


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise SupervisorError(f"missing manifest: {path}") from exc
    except json.JSONDecodeError as exc:
        raise SupervisorError(f"invalid JSON: {path}: {exc}") from exc


def validate(agent_root: Path, swarm_root: Path, registry_path: Path) -> dict[str, Any]:
    registry = load(registry_path)
    agents = registry.get("agents")
    if registry.get("declared_agent_count") != 111 or not isinstance(agents, list) or len(agents) != 111:
        raise SupervisorError("registry must contain exactly 111 agents")
    if registry.get("operational_swarm_count") != 33:
        raise SupervisorError("registry must declare exactly 33 swarms")
    ids = set()
    agent_groups: dict[str, str] = {}
    for entry in agents:
        aid = entry.get("agent_id")
        group_id = entry.get("group_id")
        if not isinstance(aid, str) or not re.fullmatch(r"OCAF-AGENT-\d{3}", aid) or aid in ids:
            raise SupervisorError(f"duplicate or invalid agent_id: {aid}")
        if not isinstance(group_id, str) or not re.fullmatch(r"G(?:0[1-9]|[12]\d|3[0-3])", group_id):
            raise SupervisorError(f"{aid}: invalid group_id: {group_id}")
        ids.add(aid)
        agent_groups[aid] = group_id
        if entry.get("ostc_enabled") is not False or entry.get("source_access") != "disabled" or entry.get("active_admission") != "DENY":
            raise SupervisorError(f"unsafe gate in {aid}")
        package = agent_root / aid
        for relative in REQUIRED_AGENT_FILES:
            if not (package / relative).is_file():
                raise SupervisorError(f"{aid}: missing {relative}")
        config = (package / "config/agent.toml").read_text(encoding="utf-8")
        if f'agent_id = "{aid}"' not in config:
            raise SupervisorError(f"{aid}: config identity mismatch")
        if f'group_id = "{group_id}"' not in config:
            raise SupervisorError(f"{aid}: config group mismatch")

    package_dirs = {
        path.name for path in agent_root.glob("OCAF-AGENT-*") if path.is_dir()
    }
    if package_dirs != ids:
        raise SupervisorError("agent package directories do not match registry exactly")

    fleet = load(swarm_root / "fleet_manifest.json")
    if fleet.get("declared_swarm_count") != 33:
        raise SupervisorError("swarm manifest must declare 33 swarms")
    swarm_ids = set()
    members = []
    for path in sorted(swarm_root.glob("OCAF-SWARM-*/manifest.json")):
        manifest = load(path)
        sid = manifest.get("swarm_id")
        if not isinstance(sid, str) or sid in swarm_ids:
            raise SupervisorError(f"duplicate or invalid swarm_id: {sid}")
        swarm_ids.add(sid)
        listed = manifest.get("members")
        if not isinstance(listed, list) or manifest.get("member_count") != len(listed):
            raise SupervisorError(f"{sid}: invalid member list")
        if manifest.get("health", {}).get("active_admission") != "DENY":
            raise SupervisorError(f"{sid}: ACTIVE gate is not DENY")
        if manifest.get("health", {}).get("ostc_enabled") is not False:
            raise SupervisorError(f"{sid}: OSTc gate is not disabled")
        expected_group = manifest.get("group_id")
        if not isinstance(expected_group, str):
            raise SupervisorError(f"{sid}: missing group_id")
        for member in listed:
            if agent_groups.get(member) != expected_group:
                raise SupervisorError(f"{sid}: member {member} is outside its group")
        members.extend(listed)
    if len(swarm_ids) != 33 or len(members) != 111 or set(members) != ids or len(set(members)) != 111:
        raise SupervisorError("swarm membership must cover exactly the 111 registered agents")
    return {"agents": 111, "swarms": 33, "active_agents": 0, "active_swarms": 0, "ostc_enabled": False, "source_access": "disabled", "active_admission": "DENY"}


def write_event(log: Path, event: str, detail: dict[str, Any]) -> None:
    log.parent.mkdir(parents=True, exist_ok=True)
    record = {"ts_ms": int(time.time() * 1000), "event": event, **detail}
    with log.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent-root", type=Path, default=DEFAULT_AGENT_ROOT)
    parser.add_argument("--swarm-root", type=Path, default=DEFAULT_SWARM_ROOT)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    parser.add_argument("--log", type=Path, default=DEFAULT_LOG)
    args = parser.parse_args()
    try:
        summary = validate(args.agent_root, args.swarm_root, args.registry)
        write_event(args.log, "FLEET_VALIDATED", summary)
        print(json.dumps({"status": "PASS", **summary}, sort_keys=True))
        return 0
    except SupervisorError as exc:
        write_event(args.log, "FLEET_REJECTED", {"status": "FAIL", "reason": str(exc), "active_admission": "DENY"})
        print(f"fleet supervisor: FAIL: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
