"""Offline Azure inventory regression tests. No Azure API is contacted."""
import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch

root = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("azure_stage038_inventory", root / "scripts/azure_stage038_inventory.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class AzureTests(unittest.TestCase):
    def test_fails_closed_when_not_authenticated(self):
        with patch.object(mod, "az_json", return_value=None):
            self.assertEqual(mod.collect()["status"], "AZURE_IDENTITY_NOT_AUTHENTICATED")

    def test_only_counts_vm_and_redacts_ids(self):
        account = {"id": "SECRET_SUBSCRIPTION", "tenantId": "SECRET_TENANT", "state": "Enabled"}
        instances = [{"id": "SECRET_VM_ID"}, {"id": "SECRET_VM_ID_2"}]
        with patch.object(mod, "az_json", side_effect=[account, instances]):
            value = mod.collect()
        self.assertEqual(value["status"], "AZURE_READ_ONLY_OBSERVED")
        self.assertEqual(value["existing_vm_count"], 2)
        self.assertNotIn("SECRET_SUBSCRIPTION", str(value))
        self.assertNotIn("SECRET_TENANT", str(value))
        self.assertNotIn("SECRET_VM_ID", str(value))
        self.assertFalse(value["production_approved"])
        self.assertEqual(value["resources_created"], 0)

    def test_denied_vm_listing_no_fabricated_availability(self):
        with patch.object(mod, "az_json", side_effect=[{"state": "Enabled"}, None]):
            value = mod.collect()
        self.assertIsNone(value["existing_vm_count"])
        self.assertFalse(value["vm_inventory_accessible"])

if __name__ == "__main__":
    unittest.main()
