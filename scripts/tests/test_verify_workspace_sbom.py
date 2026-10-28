"""Check that the attested root BOM includes the reviewed workspace graph."""
from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from scripts.verify_workspace_sbom import SbomError, verify


def bom(name: str, components: list[dict]) -> dict:
    return {"bomFormat": "CycloneDX", "specVersion": "1.5",
            "metadata": {"component": {"name": name, "version": "0.1.0"}},
            "components": components}


class WorkspaceSbomTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.path = "crates/ocaf-stage-ledger"
        member = self.root / self.path
        member.mkdir(parents=True)
        (self.root / "Cargo.toml").write_text(
            '[workspace]\nmembers = [".", "crates/ocaf-stage-ledger"]\n'
            'default-members = ["."]\n'
            '[package]\nname = "ocaf_runtime"\nversion = "0.26.0"\n'
            '[dependencies]\nocaf_stage_ledger = { version = "0.1.0", path = "crates/ocaf-stage-ledger" }\n',
            encoding="utf-8")
        (member / "Cargo.toml").write_text(
            '[package]\nname = "ocaf_stage_ledger"\nversion = "0.1.0"\n'
            'publish = false\nlicense = "MIT"\n', encoding="utf-8")
        aggregate = bom("ocaf_runtime", [
            {"name": "ocaf_stage_ledger", "version": "0.1.0", "type": "library"},
            {"name": "serde", "version": "1.0.0", "type": "library"},
        ])
        aggregate["metadata"]["component"]["version"] = "0.26.0"
        (self.root / "bom.json").write_text(json.dumps(aggregate), encoding="utf-8")
        (member / "bom.json").write_text(
            json.dumps(bom("ocaf_stage_ledger", [{"name": "serde", "version": "1.0.0"}])),
            encoding="utf-8")

    def add_g01(self) -> Path:
        path = "swarms/g01"
        member = self.root / path
        (member / "src").mkdir(parents=True)
        (member / "src/main.rs").write_text("fn main() {}\n", encoding="utf-8")
        (member / "Cargo.toml").write_text(
            '[package]\nname = "ocaf-g01-swarm"\nversion = "0.1.0"\n'
            'publish = false\nlicense = "MIT"\n'
            '[[bin]]\nname = "ocaf-g01-swarm"\npath = "src/main.rs"\n'
            '[dependencies]\n'
            'ocaf_stage_ledger = { version = "0.1.0", path = "../../crates/ocaf-stage-ledger" }\n',
            encoding="utf-8")
        root_manifest = self.root / "Cargo.toml"
        root_manifest.write_text(root_manifest.read_text(encoding="utf-8").replace(
            'members = [".", "crates/ocaf-stage-ledger"]',
            'members = [".", "crates/ocaf-stage-ledger", "swarms/g01"]'),
            encoding="utf-8")
        aggregate = self.root / "bom.json"
        data = json.loads(aggregate.read_text(encoding="utf-8"))
        data["components"].append({"name": "ocaf-g01-swarm", "version": "0.1.0",
                                   "type": "application"})
        aggregate.write_text(json.dumps(data), encoding="utf-8")
        (member / "bom.json").write_text(json.dumps(bom("ocaf-g01-swarm", [
            {"name": "ocaf_stage_ledger", "version": "0.1.0", "type": "library"}
        ])), encoding="utf-8")
        return member

    def test_accepts_aggregate_and_per_member_sboms(self):
        report = verify(self.root)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["workspace_members"], {"ocaf_stage_ledger": "0.1.0"})

    def test_accepts_both_reviewed_workspace_members(self):
        path = "crates/ocaf-transport-crdt"
        member = self.root / path
        member.mkdir(parents=True)
        (member / "Cargo.toml").write_text(
            '[package]\nname = "ocaf-transport-crdt"\nversion = "0.1.0"\n'
            'publish = false\nlicense = "MIT"\n', encoding="utf-8")
        (member / "bom.json").write_text(json.dumps(bom("ocaf-transport-crdt", [
            {"name": "serde", "version": "1.0.0"}])), encoding="utf-8")
        cargo = self.root / "Cargo.toml"
        cargo.write_text(cargo.read_text().replace(
            'members = [".", "crates/ocaf-stage-ledger"]',
            'members = [".", "crates/ocaf-stage-ledger", "crates/ocaf-transport-crdt"]'
        ) + '\nocaf_transport_crdt = { version = "0.1.0", package = "ocaf-transport-crdt", path = "crates/ocaf-transport-crdt" }\n')
        aggregate = self.root / "bom.json"
        data = json.loads(aggregate.read_text())
        data["components"].append({"name": "ocaf-transport-crdt", "version": "0.1.0"})
        aggregate.write_text(json.dumps(data))
        self.assertEqual(len(verify(self.root)["workspace_members"]), 2)
        (member / "bom.json").unlink()
        with self.assertRaises(SbomError):
            verify(self.root)

    def test_accepts_standalone_g01_with_aggregate_and_own_bom(self):
        self.add_g01()
        report = verify(self.root)
        self.assertEqual(report["workspace_members"], {
            "ocaf_stage_ledger": "0.1.0", "ocaf-g01-swarm": "0.1.0"})

    def test_rejects_aggregate_missing_or_duplicate_g01(self):
        self.add_g01()
        path = self.root / "bom.json"
        original = json.loads(path.read_text(encoding="utf-8"))
        for components in (original["components"][:-1],
                           original["components"] + [dict(original["components"][-1])]):
            with self.subTest(components=components):
                data = dict(original, components=components)
                path.write_text(json.dumps(data), encoding="utf-8")
                with self.assertRaisesRegex(SbomError, "aggregate SBOM omits or duplicates"):
                    verify(self.root)

    def test_rejects_missing_or_mismatched_g01_member_bom(self):
        member = self.add_g01()
        path = member / "bom.json"
        original = path.read_text(encoding="utf-8")
        path.unlink()
        with self.assertRaises(SbomError):
            verify(self.root)
        data = json.loads(original)
        data["metadata"]["component"]["name"] = "forged-swarm"
        path.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(SbomError, "member SBOM package identity mismatch"):
            verify(self.root)

    def test_rejects_g01_member_bom_without_exact_ledger_dependency(self):
        member = self.add_g01()
        path = member / "bom.json"
        original = json.loads(path.read_text(encoding="utf-8"))
        for components in ([],
                           [{"name": "ocaf_stage_ledger", "version": "9.9.9"}],
                           original["components"] + [dict(original["components"][0])]):
            with self.subTest(components=components):
                data = dict(original, components=components)
                path.write_text(json.dumps(data), encoding="utf-8")
                with self.assertRaisesRegex(SbomError, "standalone member SBOM omits or duplicates"):
                    verify(self.root)

    def test_rejects_aggregate_missing_member(self):
        path = self.root / "bom.json"
        data = json.loads(path.read_text())
        data["components"] = data["components"][1:]
        path.write_text(json.dumps(data))
        with self.assertRaises(SbomError):
            verify(self.root)

    def test_rejects_aggregate_duplicate_member(self):
        path = self.root / "bom.json"
        data = json.loads(path.read_text())
        data["components"].append(dict(data["components"][0]))
        path.write_text(json.dumps(data))
        with self.assertRaises(SbomError):
            verify(self.root)

    def test_rejects_missing_per_member_bom(self):
        (self.root / self.path / "bom.json").unlink()
        with self.assertRaises(SbomError):
            verify(self.root)

    def test_rejects_mismatched_per_member_identity(self):
        path = self.root / self.path / "bom.json"
        data = json.loads(path.read_text())
        data["metadata"]["component"]["name"] = "forged"
        path.write_text(json.dumps(data))
        with self.assertRaises(SbomError):
            verify(self.root)

    def test_rejects_symlinked_aggregate(self):
        path = self.root / "bom.json"
        path.rename(self.root / "real-bom.json")
        path.symlink_to(self.root / "real-bom.json")
        with self.assertRaises(SbomError):
            verify(self.root)


if __name__ == "__main__":
    unittest.main()
