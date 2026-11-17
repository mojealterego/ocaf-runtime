"""Contract tests for Builder platform targets; not physical build or E2E proof."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
TARGETS = ROOT / "docs/roadmap/agents_builder_target_platforms_v1.json"
HISTORICAL = ROOT / "docs/roadmap/agents_builder_products_v11.json"


class BuilderPlatformTargetsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.matrix = json.loads(TARGETS.read_text(encoding="utf-8"))
        cls.legacy = json.loads(HISTORICAL.read_text(encoding="utf-8"))

    def test_all_eight_platform_targets_are_unique(self):
        ids = [p["platform_id"] for p in self.matrix["targets"]]
        self.assertEqual(self.matrix["platform_target_count"], len(ids))
        self.assertEqual(set(ids), {
            "windows", "macos", "linux", "kali-linux",
            "android", "ios", "harmonyos", "kaios",
        })
        self.assertEqual(len(ids), len(set(ids)))
        self.assertTrue(all(p["status"] != "IMPLEMENTED_VERIFIED"
                            for p in self.matrix["targets"]))

    def test_code_nocode_and_local_cloud_are_switches_not_extra_apps(self):
        self.assertEqual(self.matrix["mode_switch"], ["Code", "NoCode"])
        self.assertEqual(self.matrix["workspace_switch"], ["Local", "Cloud"])
        self.assertFalse(self.matrix["cloud"]["separate_native_application"])
        self.assertNotIn("cloud", {p["platform_id"] for p in self.matrix["targets"]})
        old_cloud = {p["id"] for p in self.legacy["products"]
                     if p["platform_id"] == "cloud"}
        self.assertEqual(set(self.matrix["cloud"]["legacy_acceptance_gates"]), old_cloud)
        self.assertEqual(self.matrix["release_gate"]["status"], "NOT_ACCEPTED")

    def test_kali_reuses_linux_native_host_and_has_real_packaging_source(self):
        platforms = {p["platform_id"]: p for p in self.matrix["targets"]}
        linux, kali = platforms["linux"], platforms["kali-linux"]
        self.assertEqual(linux["native_host"], kali["native_host"])
        self.assertEqual(linux["source_root"], kali["source_root"])
        self.assertEqual(kali["distribution"], "kali-deb-amd64")
        self.assertTrue((ROOT / kali["packager"]).is_file())
        workflow = (ROOT / ".github/workflows/agents-builder-native-linux.yml").read_text()
        self.assertIn("python3 scripts/package_builder_kali.py", workflow)
        self.assertIn("test_package_builder_kali.py", workflow)

    def test_target_sources_are_real_and_not_claimed_deployed(self):
        for target in self.matrix["targets"]:
            with self.subTest(target=target["platform_id"]):
                self.assertTrue((ROOT / target["source_root"]).is_dir())
                self.assertIn("status", target)
                self.assertNotEqual(target["status"], "DEPLOYED")


if __name__ == "__main__":
    unittest.main()
