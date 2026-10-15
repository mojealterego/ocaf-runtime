"""Regression: historical implemented OCAF development work must not be erased by blanket TODO."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import render_readme_roadmap as renderer

ROOT=Path(__file__).resolve().parents[2]

class RecoveredImplementationStatusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inv=json.loads((ROOT/"docs/roadmap/canonical_v11_inventory.json").read_text())
        cls.builders=json.loads((ROOT/"docs/roadmap/agents_builder_products_v11.json").read_text())

    def row(self,stage):
        return next(r for r in self.inv["main"] if r["id"]==stage)

    def test_real_historical_code_implementation_not_all_todo(self):
        for stage in ("0.06","0.07","0.08","0.09","0.11","0.12","0.13",
                      "0.14","0.15","0.16","0.17","0.18","0.19","0.20",
                      "0.21","0.22","0.23","0.24","0.25","0.26","0.27",
                      "0.28","0.29","0.30","0.31","0.32","0.33","0.34","0.35"):
            with self.subTest(stage=stage):
                row=self.row(stage)
                self.assertIn("implementation_proof",row)
                p=row["implementation_proof"]
                self.assertEqual(p["status"],"IMPLEMENTED_REPO_SCOPE_VERIFIED")
                self.assertFalse(p["production_approved"])
                self.assertEqual(p["stage_id"],stage)
                self.assertTrue(p["paths"])
                self.assertEqual(renderer.mark(row["status"],"main",row),
                                 "ZROBIONE — ZAKRES IMPLEMENTACJI")

    def test_missing_proof_is_not_pass_and_no_prod_claim(self):
        row=copy.deepcopy(self.row("0.11"))
        row.pop("implementation_proof")
        self.assertEqual(renderer.mark(row["status"],"main",row),"DO ZROBIENIA")
        for stage in ("0.00","0.01","0.02","0.03","0.04","0.05","0.10",
                      "0.38","0.39","0.40","0.41","0.50","0.51","0.52","0.53","0.54","0.55"):
            self.assertNotEqual(renderer.mark(self.row(stage)["status"],"main",self.row(stage)),
                                "ZROBIONE — ZAKRES IMPLEMENTACJI")

    def test_front_pinned_readme_contains_historical_implementation_labels(self):
        section=renderer.render_section(self.inv,self.builders)
        self.assertIn("7. **[ZROBIONE — ZAKRES IMPLEMENTACJI]**",section)
        self.assertIn("12. **[ZROBIONE — ZAKRES IMPLEMENTACJI]**",section)
        self.assertIn("36. **[ZROBIONE — ZAKRES IMPLEMENTACJI]**",section)
        self.assertIn("50. **[DO ZROBIENIA]**",section)
        self.assertEqual(section.count("ZROBIONE — ZAKRES IMPLEMENTACJI]"),29)

if __name__=="__main__":
    unittest.main()
