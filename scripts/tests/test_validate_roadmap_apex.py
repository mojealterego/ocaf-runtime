"""Regression: Apex Tier-0 Roadmap must remain 52 traceable and falsifiable TODOs."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import validate_roadmap_apex as guard

REPO = Path(__file__).resolve().parents[2]
MANIFEST = REPO / "docs/roadmap/apex_tier0_extension.json"
MARKDOWN = REPO / "docs/ROADMAP_APEX_TIER0_2026-10-15.md"


class RoadmapApexGuardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = guard.parse_roadmap(MANIFEST.read_bytes())
        cls.prose = MARKDOWN.read_text(encoding="utf-8")

    def verify(self, data=None, markdown=None):
        return guard.validate(
            copy.deepcopy(self.source) if data is None else data,
            self.prose if markdown is None else markdown,
        )

    def test_real_source_manifest_and_exact_stage_boundaries(self):
        result = self.verify()
        self.assertEqual(result["new_stage_rows"], 52)
        self.assertEqual(result["main_stages_added"], 40)
        self.assertEqual(result["technical_substages_added"], 12)
        self.assertEqual(result["architecture_pillars"], 6)
        self.assertFalse(result["production_approved"])
        self.assertEqual(result["features_implemented_by_this_roadmap_change"], 0)

    def test_cannot_rewrite_source_or_claim_production_signoff(self):
        for key, val in (
            ("source_pdf", "other.pdf"), ("production_approved", True),
            ("main_stages", 41), ("technical_substages", 11),
            ("branch", "feature"),
        ):
            obj = copy.deepcopy(self.source)
            obj[key] = val
            with self.subTest(key=key), self.assertRaises(guard.RoadmapValidationError):
                self.verify(data=obj)

    def test_no_fake_completed_status_without_real_evidence(self):
        for status in ("IN_PROGRESS", "COMPLETE"):
            obj = copy.deepcopy(self.source)
            obj["items"][0]["status"] = status
            with self.subTest(status=status), self.assertRaises(guard.RoadmapValidationError):
                self.verify(data=obj)

    def test_omitted_negative_test_wrong_dependency_and_new_unknown_field(self):
        for mutation in (
            lambda d: d["items"][0].__setitem__("negative", ""),
            lambda d: d["items"][0].__setitem__("dependencies", ["1.15"]),
            lambda d: d["items"][0].__setitem__("pdf_pages", [16]),
            lambda d: d["items"][0].__setitem__("extra", "unapproved"),
        ):
            obj = copy.deepcopy(self.source)
            mutation(obj)
            with self.assertRaises(guard.RoadmapValidationError):
                self.verify(data=obj)

    def test_rejects_missing_stage_or_markdown_out_of_sync(self):
        obj = copy.deepcopy(self.source)
        obj["items"].pop()
        with self.assertRaises(guard.RoadmapValidationError):
            self.verify(data=obj)
        prose = self.prose.replace("| **1.15** |", "| 1.15 |", 1)
        with self.assertRaises(guard.RoadmapValidationError):
            self.verify(markdown=prose)

    def test_duplicate_json_and_nan_values_fail_closed(self):
        for raw in (b'{"schema_version":1,"schema_version":1}',
                    b'{"score":NaN}', b'', b'not-json'):
            with self.subTest(raw=raw), self.assertRaises(guard.RoadmapValidationError):
                guard.parse_roadmap(raw)


if __name__ == "__main__":
    unittest.main()
