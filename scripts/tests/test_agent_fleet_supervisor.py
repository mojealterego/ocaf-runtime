import importlib.util
import json
import tempfile
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
