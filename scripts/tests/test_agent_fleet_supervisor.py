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



def test_noncanonical_agent_numbers_and_unicode_are_rejected():
    original = json.loads((ROOT / "docs/agents/agent_registry_111.json").read_text())
    for forged_id in ("OCAF-AGENT-000", "OCAF-AGENT-112", "OCAF-AGENT-999",
                      "OCAF-AGENT-1", "OCAF-AGENT-١١١"):
        registry = json.loads(json.dumps(original))
        registry["agents"][0]["agent_id"] = forged_id
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "registry.json"
            path.write_text(json.dumps(registry), encoding="utf-8")
            try:
                MODULE.validate(
                    ROOT / "OCAF Agents MOJEALTEREGO/AGENT-FLEET-111",
                    ROOT / "OCAF Agents MOJEALTEREGO/SWARM-FLEET-33",
                    path,
                )
            except MODULE.SupervisorError as exc:
                assert "invalid agent_id" in str(exc), (forged_id, str(exc))
            else:
                raise AssertionError(f"noncanonical agent ID accepted: {forged_id}")


def test_swarm_master_and_member_manifest_must_match():
    root = ROOT / "OCAF Agents MOJEALTEREGO/SWARM-FLEET-33"
    master_path = root / "fleet_manifest.json"
    manifest_path = root / "OCAF-SWARM-01/manifest.json"
    original_master = json.loads(master_path.read_text(encoding="utf-8"))
    original_manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory() as tmp:
        sandbox = Path(tmp) / "swarms"
        sandbox.mkdir()
        (sandbox / "fleet_manifest.json").write_text(
            json.dumps(original_master), encoding="utf-8")
        for item in original_master["swarms"]:
            sid = item["swarm_id"]
            target = sandbox / sid / "manifest.json"
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps(item), encoding="utf-8")
        agent_root = ROOT / "OCAF Agents MOJEALTEREGO/AGENT-FLEET-111"
        registry = ROOT / "docs/agents/agent_registry_111.json"

        # An intact copy must validate identically to the committed fleet.
        assert MODULE.validate(agent_root, sandbox, registry)["swarms"] == 33

        target = sandbox / "OCAF-SWARM-01/manifest.json"
        changed = json.loads(target.read_text(encoding="utf-8"))
        changed["specialization"] = "forged"
        target.write_text(json.dumps(changed), encoding="utf-8")
        try:
            MODULE.validate(agent_root, sandbox, registry)
        except MODULE.SupervisorError as exc:
            assert "differs from master" in str(exc)
        else:
            raise AssertionError("divergent swarm declaration must be rejected")

        target.write_text(json.dumps(original_manifest), encoding="utf-8")
        master = json.loads((sandbox / "fleet_manifest.json").read_text(encoding="utf-8"))
        master["swarms"][0]["swarm_id"] = "OCAF-SWARM-99"
        (sandbox / "fleet_manifest.json").write_text(json.dumps(master), encoding="utf-8")
        try:
            MODULE.validate(agent_root, sandbox, registry)
        except MODULE.SupervisorError as exc:
            assert "invalid or duplicate swarm ID" in str(exc)
        else:
            raise AssertionError("invalid master swarm ID must be rejected")


def test_strict_fleet_json_rejects_duplicates_nonfinite_symlink_and_oversize():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        manifest = root / "manifest.json"
        cases = (
            ('{"a":1,"a":2}', "duplicate JSON key"),
            ('{"amount":NaN}', "non-finite JSON number"),
            ('{"amount":Infinity}', "non-finite JSON number"),
            ('{"a":', "invalid or inaccessible manifest"),
            ('{}{}', "invalid or inaccessible manifest"),
        )
        for content, needle in cases:
            manifest.write_text(content, encoding="utf-8")
            try:
                MODULE.load(manifest)
            except MODULE.SupervisorError as exc:
                assert needle in str(exc), (content, str(exc))
            else:
                raise AssertionError(f"invalid JSON accepted: {content}")
        manifest.write_text('{"ok":true}', encoding="utf-8")
        assert MODULE.load(manifest) == {"ok": True}
        alias = root / "alias.json"
        alias.symlink_to(manifest)
        try:
            MODULE.load(alias)
        except MODULE.SupervisorError:
            pass
        else:
            raise AssertionError("symlink manifest must be rejected")
        with manifest.open("wb") as handle:
            handle.truncate(MODULE.MAX_MANIFEST_BYTES + 1)
        try:
            MODULE.load(manifest)
        except MODULE.SupervisorError as exc:
            assert "oversized" in str(exc)
        else:
            raise AssertionError("oversized manifest must be rejected")

def load_tests(_loader, _tests, _pattern):
    cases = [value for name, value in globals().items()
             if name.startswith("test_") and callable(value)]
    return unittest.TestSuite(unittest.FunctionTestCase(case) for case in cases)
