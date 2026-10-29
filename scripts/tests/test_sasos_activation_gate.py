"""Regression tests for the Stage 334 catalog gate."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from sasos_activation_gate import CATALOG, CatalogIntegrityError, audit_catalog


class SasosActivationGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = json.loads(CATALOG.read_text(encoding="utf-8"))

    def test_all_entries_remain_unverified(self):
        report = audit_catalog(self.original)
        self.assertEqual(report["declared_services"], 555)
        self.assertEqual(report["active_backends_verified"], 0)
        self.assertEqual(sum(report["declared_runtime_counts"].values()), 555)

    def test_rejects_missing_duplicate_or_invalid_identity(self):
        entries = copy.deepcopy(self.original)
        entries.pop()
        with self.assertRaises(CatalogIntegrityError):
            audit_catalog(entries)
        entries = copy.deepcopy(self.original)
        entries[1] = copy.deepcopy(entries[0])
        with self.assertRaises(CatalogIntegrityError):
            audit_catalog(entries)
        entries = copy.deepcopy(self.original)
        entries[0]["number"] = True
        with self.assertRaises(CatalogIntegrityError):
            audit_catalog(entries)

    def test_rejects_unverified_claims(self):
        for field, value in [
            ("status", "INTEGRATED"),
            ("executable", True),
            ("imandra_verified", True),
            ("measured_latency_microseconds", 0),
            ("zero_copy_verified", True),
        ]:
            entries = copy.deepcopy(self.original)
            entries[0][field] = value
            with self.subTest(field=field), self.assertRaises(CatalogIntegrityError):
                audit_catalog(entries)

    def test_rejects_extra_fields_and_duplicate_required_fields(self):
        entries = copy.deepcopy(self.original)
        entries[0]["unexpected"] = "value"
        with self.assertRaises(CatalogIntegrityError):
            audit_catalog(entries)
        entries = copy.deepcopy(self.original)
        entries[0]["required_fields"] = ["prompt", "prompt"]
        with self.assertRaises(CatalogIntegrityError):
            audit_catalog(entries)


if __name__ == "__main__":
    unittest.main()
