"""Regression tests for the protected README roadmap and manifest parity."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import render_readme_roadmap as renderer

ROOT = Path(__file__).resolve().parents[2]


class FullReadmeRoadmapTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inv = json.loads((ROOT / "docs/roadmap/canonical_v11_inventory.json").read_text())
        cls.builders = json.loads((ROOT / "docs/roadmap/agents_builder_products_v11.json").read_text())
        cls.readme = (ROOT / "README.md").read_text()

    def test_complete_manifest_is_pinned(self):
        result = renderer.validate_readme(self.readme, self.inv, self.builders)
        self.assertEqual(result["numbered_items"], 407)
        self.assertEqual(result["main_stage_items"], 333)
        self.assertEqual(result["technical_substage_items"], 64)
        self.assertEqual(result["original_agents_builder_variants"], 10)
        self.assertFalse(result["production_approved"])

    def test_warning_and_no_samowolka_are_first(self):
        head = self.readme.split(renderer.END, 1)[0]
        self.assertTrue(self.readme.startswith(renderer.BEGIN))
        self.assertIn("ZABRONIONA JEST SAMOWOLKA", head[:1500])
        self.assertIn("ZASADA STAŁEJ ROZBUDOWY ROADMAPY", head)

    def test_every_main_and_technical_id_appears_once(self):
        head = self.readme.split(renderer.END, 1)[0]
        for item in self.inv["main"] + self.inv["technical"]:
            with self.subTest(id=item["id"]):
                self.assertEqual(head.count("| **" + item["id"] + "** |"), 1)

    def test_all_original_builder_products(self):
        head = self.readme.split(renderer.END, 1)[0]
        for product in self.builders["products"]:
            with self.subTest(id=product["id"]):
                self.assertEqual(head.count("| **" + product["id"] + "** |"), 1)
                self.assertIn(product["product"], head)

    def test_negative_missing_stage_or_banner(self):
        for altered in (
            self.readme.replace("| **0.39.4** |", "| **MISSING** |", 1),
            self.readme.replace("| **333** |", "| **MISSING** |", 1),
            self.readme.replace("ZABRONIONA JEST SAMOWOLKA", "REMOVED", 1),
            self.readme.replace(renderer.BEGIN, "", 1),
            self.readme.replace(renderer.END, "", 1),
        ):
            with self.subTest(altered=altered[:80]):
                with self.assertRaises(renderer.ReadmeRoadmapError):
                    renderer.validate_readme(altered, self.inv, self.builders)

    def test_manifest_mutation_requires_readme_update(self):
        modified = copy.deepcopy(self.inv)
        modified["main"].append({"id": "334", "title": "New stage"})
        modified["main_count"] += 1
        with self.assertRaises(renderer.ReadmeRoadmapError):
            renderer.validate_readme(self.readme, modified, self.builders)


if __name__ == "__main__":
    unittest.main()
