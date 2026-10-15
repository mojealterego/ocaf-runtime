"""Guard against losing any canonical OCAF stage, substage or source reference."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import validate_roadmap_canonical as check

ROOT=Path(__file__).resolve().parents[2]

class CanonicalRoadmapTests(unittest.TestCase):
    def setUp(self):
        self.inventory=json.loads((ROOT/"docs/roadmap/canonical_v10_inventory.json").read_text())
        self.md=(ROOT/"docs/ROADMAP_FULL_ECOSYSTEM_2026.md").read_text()
        self.archive=(ROOT/"docs/archive/ROADMAP_FULL_ECOSYSTEM_v9_2026-10-15.md").read_text()

    def test_all_195_stages_64_substeps_and_33_upstreams_restored(self):
        summary=check.validate(self.inventory,self.md,self.archive)
        self.assertEqual(summary["main"],195)
        self.assertEqual(summary["technical"],64)
        self.assertEqual(summary["upstream"],33)
        self.assertEqual(summary["unspecified_early"],11)
        self.assertFalse(summary["production_approved"])

    def test_missing_any_stage_id_is_failure(self):
        bad=copy.deepcopy(self.inventory)
        bad["main"].pop(45)
        with self.assertRaises(check.CanonicalRoadmapError):
            check.validate(bad,self.md,self.archive)

    def test_duplicate_substage_and_missing_upstream_are_failures(self):
        for list_name in ("technical","reference_repositories"):
            bad=copy.deepcopy(self.inventory)
            bad[list_name].append(bad[list_name][0])
            with self.subTest(list_name=list_name),self.assertRaises(check.CanonicalRoadmapError):
                check.validate(bad,self.md,self.archive)

    def test_missing_markdown_row_is_failure(self):
        with self.assertRaises(check.CanonicalRoadmapError):
            check.validate(self.inventory,self.md.replace("| **1.94** |","| 1.94 |"),self.archive)

    def test_cannot_make_operator_milestone_a_production_signoff(self):
        bad=copy.deepcopy(self.inventory)
        bad["production_approved"]=True
        with self.assertRaises(check.CanonicalRoadmapError):
            check.validate(bad,self.md,self.archive)
        bad=copy.deepcopy(self.inventory)
        bad["main"][38]["status"]="COMPLETE_PRODUCTION"
        with self.assertRaises(check.CanonicalRoadmapError):
            check.validate(bad,self.md,self.archive)

    def test_original_legacy_matrix_must_be_preserved(self):
        with self.assertRaises(check.CanonicalRoadmapError):
            check.validate(self.inventory,self.md,self.archive.replace("## Complete execution matrix","## Lost source"))

    def test_historical_unknown_early_items_must_not_be_fabricated(self):
        bad=copy.deepcopy(self.inventory)
        bad["main"][2]["status"]="COMPLETE"
        with self.assertRaises(check.CanonicalRoadmapError):
            check.validate(bad,self.md,self.archive)
        bad=copy.deepcopy(self.inventory)
        bad["main"][2]["title"]="Verified agent execution"
        with self.assertRaises(check.CanonicalRoadmapError):
            check.validate(bad,self.md,self.archive)
