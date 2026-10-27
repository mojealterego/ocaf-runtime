import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "ocaf_agent_fleet_supervisor", ROOT / "scripts/ocaf_agent_fleet_supervisor.py"
)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_real_fleet_is_exactly_111_and_33():
    result = MODULE.validate(
        ROOT / "OCAF Agents MOJEALTEREGO/AGENT-FLEET-111",
        ROOT / "OCAF Agents MOJEALTEREGO/SWARM-FLEET-33",
        ROOT / "docs/agents/agent_registry_111.json",
    )
    assert result["agents"] == 111
    assert result["swarms"] == 33
    assert result["active_admission"] == "DENY"
    assert result["inventory_only"] is True


def test_registry_uses_unique_named_agent_packages():
    registry = json.loads((ROOT / "docs/agents/agent_registry_111.json").read_text())
    agent_root = ROOT / "OCAF Agents MOJEALTEREGO/AGENT-FLEET-111"
    paths = [MODULE.agent_package_path(agent_root, entry) for entry in registry["agents"]]
    assert len(set(paths)) == 111
    assert paths[0].name == "OCAF-AGENT-001-asterion"


def test_package_path_escape_is_rejected():
    registry = json.loads((ROOT / "docs/agents/agent_registry_111.json").read_text())
    registry["agents"][0]["package_path"] = "AGENT-FLEET-111/../OCAF-AGENT-001-asterion"
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "registry.json"
        path.write_text(json.dumps(registry))
        try:
            MODULE.validate(
                ROOT / "OCAF Agents MOJEALTEREGO/AGENT-FLEET-111",
                ROOT / "OCAF Agents MOJEALTEREGO/SWARM-FLEET-33",
                path,
            )
        except MODULE.SupervisorError as exc:
            assert "invalid package_path" in str(exc)
        else:
            raise AssertionError("package path escape must be rejected")


def test_declared_active_agents_are_rejected():
    registry = json.loads((ROOT / "docs/agents/agent_registry_111.json").read_text())
    registry["active_runtime_agent_count"] = 1
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "registry.json"
        path.write_text(json.dumps(registry))
        try:
            MODULE.validate(
                ROOT / "OCAF Agents MOJEALTEREGO/AGENT-FLEET-111",
                ROOT / "OCAF Agents MOJEALTEREGO/SWARM-FLEET-33",
                path,
            )
        except MODULE.SupervisorError as exc:
            assert "ACTIVE" in str(exc)
        else:
            raise AssertionError("false ACTIVE declaration must be rejected")


def test_duplicate_agent_is_rejected():
    original = json.loads((ROOT / "docs/agents/agent_registry_111.json").read_text())
    original["agents"][1]["agent_id"] = original["agents"][0]["agent_id"]
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "registry.json"
        path.write_text(json.dumps(original))
        try:
            MODULE.validate(
                ROOT / "OCAF Agents MOJEALTEREGO/AGENT-FLEET-111",
                ROOT / "OCAF Agents MOJEALTEREGO/SWARM-FLEET-33",
                path,
            )
        except MODULE.SupervisorError as exc:
            assert "duplicate" in str(exc)
        else:
            raise AssertionError("duplicate agent must be rejected")


def load_tests(_loader, _tests, _pattern):
    cases = [value for name, value in globals().items()
             if name.startswith("test_") and callable(value)]
    return unittest.TestSuite(unittest.FunctionTestCase(case) for case in cases)
