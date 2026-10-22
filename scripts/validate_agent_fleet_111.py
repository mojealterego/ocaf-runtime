#!/usr/bin/env python3
"""Fail-closed structural validator for the 111-agent fleet."""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FLEET = ROOT / "OCAF Agents MOJEALTEREGO" / "AGENT-FLEET-111"
REGISTRY = ROOT / "docs" / "agents" / "agent_registry_111.json"
REQUIRED = (
    "Cargo.toml", "src/main.rs", "src/lib.rs", "config/agent.toml",
    "config/autonomy.toml", "config/tool_policy.toml", "docs/README.md",
    "docs/ARCHITECTURE.md", "docs/USER_MANUAL.md", "tests/security.rs",
    "packaging/install.sh", "packaging/uninstall.sh",
    "packaging/INSTALLER_CONTRACT.md", "packaging/BUILD_EVIDENCE.md",
    "certification/CERTIFICATE_RECORD.md",
)


def main() -> int:
    registry = json.loads(REGISTRY.read_text())
    agents = registry.get("agents", [])
    assert registry.get("declared_agent_count") == 111
    assert registry.get("operational_swarm_count") == 33
    assert len(agents) == 111
    assert len({a["agent_id"] for a in agents}) == 111
    assert registry.get("active_runtime_agent_count") == 0
    for entry in agents:
        agent = FLEET / entry["agent_id"]
        assert agent.is_dir(), agent
        for relative in REQUIRED:
            assert (agent / relative).is_file(), (agent, relative)
        config = (agent / "config/agent.toml").read_text()
        assert f'agent_id = "{entry["agent_id"]}"' in config
        assert 'ostc_enabled = false' in config
        assert 'source_access = "disabled"' in config
        assert 'active_admission = "DENY"' in config
    print("agent_fleet_111: PASS (111 separate packages, 33 swarms, ACTIVE denied)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, KeyError, json.JSONDecodeError) as exc:
        print(f"agent_fleet_111: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
