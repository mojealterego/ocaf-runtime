import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "engineering_factory_runner.py"
SPEC = importlib.util.spec_from_file_location("engineering_factory_runner", SCRIPT)
factory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(factory)


def git(repo, *args):
    return subprocess.check_output(["git", *args], cwd=repo, text=True).strip()


class EngineeringFactoryRunnerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name) / "repo"
        self.repo.mkdir()
        subprocess.check_call(["git", "init", "-q", str(self.repo)])
        git(self.repo, "config", "user.email", "factory@example.invalid")
        git(self.repo, "config", "user.name", "Factory Test")
        (self.repo / "README.md").write_text("base\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", "base")
        self.mission = {
            "mission_id": "mission-1", "base_sha": git(self.repo, "rev-parse", "HEAD"),
            "workers": [
                {"id": "a", "role": "rust", "instructions": "create a", "write_set": ["a.txt"]},
                {"id": "b", "role": "test", "instructions": "create b", "write_set": ["b.txt"]},
            ],
            "verify_commands": [["cargo", "test", "--locked", "--no-default-features"]],
        }

    def test_parallel_workers_integration_review_and_credential_boundary(self):
        mission_file = Path(self.tmp.name) / "mission.json"
        mission_file.write_text(json.dumps(self.mission))
        output = Path(self.tmp.name) / "output"
        actual_run = factory.run
        calls = []

        def fake_run(argv, **kwargs):
            if argv[0] == "mock-codex":
                tree = Path(argv[argv.index("-C") + 1])
                calls.append(("codex", tree.name))
                self.assertIn("shell_environment_policy.inherit=none", argv)
                self.assertNotIn("GH_TOKEN", kwargs["env"])
                if "--output-schema" in argv:
                    Path(argv[argv.index("-o") + 1]).write_text(json.dumps({"decision": "approved", "findings": []}))
                else:
                    (tree / ("a.txt" if tree.name == "worker-0" else "b.txt")).write_text("worker change\n")
                    time.sleep(0.15)
                return subprocess.CompletedProcess(argv, 0, "", "")
            if argv[:2] == ["cargo", "test"]:
                calls.append(("test", ""))
                self.assertNotIn("OPENAI_API_KEY", kwargs["env"])
                self.assertNotIn("GH_TOKEN", kwargs["env"])
                return subprocess.CompletedProcess(argv, 0, "", "")
            return actual_run(argv, **kwargs)

        with patch.dict(os.environ, {"OPENAI_API_KEY": "mock-secret", "GH_TOKEN": "mock-token"}), \
             patch.object(factory, "run", side_effect=fake_run):
            receipt = factory.execute(mission_file, output, self.repo, "mock-codex")
        self.assertGreater(receipt["concurrency"]["overlap_ms"], 0)
        self.assertEqual(len(receipt["workers"]), 2)
        self.assertNotIn(receipt["review"]["instance_id"], [w["instance_id"] for w in receipt["workers"]])
        self.assertEqual(receipt["review"]["decision"], "approved")
        self.assertTrue(receipt["tests"]["passed"])
        self.assertEqual(sum(kind == "codex" for kind, _ in calls), 3)
        self.assertIn("a.txt", (output / "integrated.patch").read_text())
        self.assertIn("b.txt", (output / "integrated.patch").read_text())
        self.assertNotIn("mock-secret", (output / "receipt.json").read_text())

    def test_rejects_protected_ancestor_scope_and_bad_base(self):
        self.mission["workers"][0]["write_set"] = ["scripts"]
        with self.assertRaises(factory.FactoryFailure):
            factory.validate_mission(self.mission)
        self.mission["workers"][0]["write_set"] = ["a.txt"]
        self.mission["base_sha"] = "0" * 40
        mission_file = Path(self.tmp.name) / "mission.json"
        mission_file.write_text(json.dumps(self.mission))
        with self.assertRaises(factory.FactoryFailure):
            factory.execute(mission_file, Path(self.tmp.name) / "output", self.repo, "mock-codex")

    def test_rejects_worker_edit_outside_declared_scope(self):
        (self.repo / "wrong.txt").write_text("unauthorized\n")
        with self.assertRaisesRegex(factory.FactoryFailure, "outside write_set"):
            factory.check_patch(self.repo, ["a.txt"])


if __name__ == "__main__":
    unittest.main()
