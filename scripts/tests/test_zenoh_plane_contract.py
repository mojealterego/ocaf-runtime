import importlib.util
import json
from pathlib import Path
import unittest

MODULE = Path(__file__).resolve().parents[1] / "zenoh_plane_contract.py"
spec = importlib.util.spec_from_file_location("zenoh_plane_contract", MODULE)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)
CONTRACT = Path(__file__).resolve().parents[2] / "deploy/zenoh-plane-contract.json"

class PlaneContractTests(unittest.TestCase):
    def setUp(self):
        self.document = json.loads(CONTRACT.read_text())

    def test_canonical_plane_passes(self):
        report = checker.validate(self.document)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["active_admission"], "DENY")

    def test_direct_ocaf_omat_bypass_is_rejected(self):
        self.document["allowed_links"].append({"from": "ocaf", "to": "omat"})
        with self.assertRaises(checker.ContractError): checker.validate(self.document)

    def test_ostc_cannot_be_enabled_in_current_contract(self):
        self.document["health"]["ostc_enabled"] = True
        with self.assertRaises(checker.ContractError): checker.validate(self.document)

    def test_missing_replay_store_is_rejected(self):
        self.document["health"]["required"].remove("replay-store")
        with self.assertRaises(checker.ContractError): checker.validate(self.document)

if __name__ == "__main__":
    unittest.main()
