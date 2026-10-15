"""Negative regression suite for v9 from 13-page user-supplied PDF."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import validate_roadmap_sovereign_v9 as guard

ROOT=Path(__file__).resolve().parents[2]
MANIFEST=ROOT/"docs/roadmap/sovereign_workspace_v9.json"
MARKDOWN=ROOT/"docs/ROADMAP_SOVEREIGN_WORKSPACE_V9_2026-10-15.md"


class SovereignV9Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source=guard.decode(MANIFEST.read_bytes())
        cls.markdown=MARKDOWN.read_text(encoding="utf-8")

    def verify(self,d=None,md=None):
        return guard.validate(copy.deepcopy(self.source) if d is None else d,
                              self.markdown if md is None else md)

    def test_eight_phases_six_crates_and_all_56_items(self):
        p=self.verify()
        self.assertEqual(p["new_main_stages"],40)
        self.assertEqual(p["new_technical_substages"],16)
        self.assertEqual(p["eight_research_phases"],8)
        self.assertEqual(p["proposed_crates"],6)
        self.assertEqual(p["main_ids_in_range"],195)
        self.assertEqual(p["features_implemented_in_this_extension"],0)
        self.assertFalse(p["production_approved"])

    def test_missing_research_step_or_crate_rejected(self):
        for field,action in (
            ("architecture_research_steps",lambda d:d[field].pop()),
            ("proposed_cargo_crates",lambda d:d[field].pop()),
        ):
            d=copy.deepcopy(self.source)
            action(d)
            with self.assertRaises(guard.InvalidRoadmap):
                self.verify(d=d)

    def test_invented_release_claim_and_bad_source_rejected(self):
        for k,v in (
            ("production_approved",True),
            ("source_pdf","untrusted.pdf"),
            ("main_stages",56),
            ("branch","experimental"),
        ):
            d=copy.deepcopy(self.source);d[k]=v
            with self.subTest(k=k),self.assertRaises(guard.InvalidRoadmap):
                self.verify(d=d)

    def test_stage_cannot_advance_without_real_artifact(self):
        for status in ("IN_PROGRESS","COMPLETE"):
            d=copy.deepcopy(self.source)
            d["items"][0]["status"]=status
            with self.subTest(status=status),self.assertRaises(guard.InvalidRoadmap):
                self.verify(d=d)

    def test_invalid_tests_source_page_and_dependency_rejected(self):
        for key,value in (
            ("negative",""),("acceptance","fake"),("pdf_pages",[14]),
            ("dependencies",["1.55"]),("evidence",[{"fabricated":True}]),
        ):
            d=copy.deepcopy(self.source);d["items"][0][key]=value
            with self.subTest(key=key),self.assertRaises(guard.InvalidRoadmap):
                self.verify(d=d)

    def test_missing_stage_and_swapped_table_row_rejected(self):
        d=copy.deepcopy(self.source);d["items"].pop()
        with self.assertRaises(guard.InvalidRoadmap):
            self.verify(d=d)
        with self.assertRaises(guard.InvalidRoadmap):
            self.verify(md=self.markdown.replace("| **1.55** |","| 1.55 |",1))

    def test_duplicate_json_and_nan_fail_closed(self):
        for raw in (b'{"a":1,"a":2}',b'{"rate":NaN}',b''):
            with self.subTest(raw=raw),self.assertRaises(guard.InvalidRoadmap):
                guard.decode(raw)


if __name__=="__main__":
    unittest.main()
