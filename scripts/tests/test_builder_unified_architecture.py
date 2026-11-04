"""Contract tests: five unified apps, ten historical Code/NoCode acceptance gates."""
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

    def test_exactly_five_unique_platform_applications(self):
        apps = self.arch["applications"]
        self.assertEqual(len(apps), 5)
        self.assertEqual({a["platform_id"] for a in apps},
                         {"cloud", "windows", "macos", "android", "ios"})
        self.assertEqual(len({a["id"] for a in apps}), 5)
        self.assertEqual(self.arch["architecture"], "five-unified-applications")

    def test_legacy_ten_mode_acceptances_are_partitioned_without_duplication(self):
        products = self.manifest["products"]
        ids = {p["id"] for p in products}
        self.assertEqual(len(products), 10)
        self.assertEqual(len(ids), 10)
        referenced = []
        for app in self.arch["applications"]:
            matching = [p for p in products if p["platform_id"] == app["platform_id"]]
            self.assertEqual({p["mode"] for p in matching}, {"Code", "NoCode"})
            self.assertEqual(set(app["mode_product_ids"]), {p["id"] for p in matching})
            referenced.extend(app["mode_product_ids"])
        self.assertEqual(len(referenced), len(set(referenced)))
        self.assertEqual(set(referenced), ids)

    def test_switches_cloud_and_native_evidence_cannot_be_conflated(self):
        self.assertEqual(self.arch["mode_switch"], ["Code", "NoCode"])
        self.assertEqual(self.arch["workspace_switch"], ["Local", "Cloud"])
        self.assertEqual(self.arch["implementation_status"], "SPECIFIED_NOT_ACCEPTED")
        for app in self.arch["applications"]:
            self.assertEqual(app["native_artifact_required"],
                             app["platform_id"] != "cloud")
            self.assertEqual(app["cloud_workspace"],
                             "native" if app["platform_id"] == "cloud"
                             else "remote-authenticated")
        for product in self.manifest["products"]:
            self.assertFalse(product["production_approved"])


if __name__ == "__main__":
    unittest.main()
