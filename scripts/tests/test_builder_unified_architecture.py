"""Contract tests: four native Builder applications and two Cloud service gates."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "docs/roadmap/agents_builder_products_v11.json"


class UnifiedBuilderArchitectureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cls.arch = cls.manifest["application_architecture"]

    def test_exactly_four_native_applications(self):
        apps = self.arch["applications"]
        self.assertEqual(len(apps), 4)
        self.assertEqual({a["platform_id"] for a in apps},
                         {"windows", "macos", "android", "ios"})
        self.assertEqual(len({a["id"] for a in apps}), 4)
        self.assertEqual(self.arch["architecture"], "four-native-applications-cloud-service")
        self.assertTrue(all(a["native_artifact_required"] for a in apps))

    def test_eight_native_mode_gates_and_two_cloud_service_gates(self):
        products = self.manifest["products"]
        ids = {p["id"] for p in products}
        self.assertEqual(len(products), 10)
        self.assertEqual(len(ids), 10)
        native = [p for p in products if p["platform_id"] != "cloud"]
        cloud = [p for p in products if p["platform_id"] == "cloud"]
        self.assertEqual(len(native), 8)
        self.assertEqual({p["mode"] for p in cloud}, {"Code", "NoCode"})
        referenced = []
        for app in self.arch["applications"]:
            matching = [p for p in native if p["platform_id"] == app["platform_id"]]
            self.assertEqual({p["mode"] for p in matching}, {"Code", "NoCode"})
            self.assertEqual(set(app["mode_product_ids"]), {p["id"] for p in matching})
            referenced.extend(app["mode_product_ids"])
        self.assertEqual(len(referenced), len(set(referenced)))
        self.assertEqual(set(referenced), {p["id"] for p in native})
        self.assertEqual(set(self.arch["cloud_service"]["mode_gate_ids"]),
                         {p["id"] for p in cloud})
        self.assertFalse(self.arch["cloud_service"]["native_artifact_required"])

    def test_switches_and_production_evidence_remain_separate(self):
        self.assertEqual(self.arch["mode_switch"], ["Code", "NoCode"])
        self.assertEqual(self.arch["workspace_switch"], ["Local", "Cloud"])
        self.assertEqual(self.arch["implementation_status"], "SPECIFIED_NOT_ACCEPTED")
        for app in self.arch["applications"]:
            self.assertEqual(app["cloud_workspace"], "remote-authenticated")
        for product in self.manifest["products"]:
            self.assertFalse(product["production_approved"])


if __name__ == "__main__":
    unittest.main()
