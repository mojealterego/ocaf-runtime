import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_soak import SoakEvidenceError, validate_soak


class SoakEvidenceTests(unittest.TestCase):
    def report(self):
        return {"duration_seconds": 3600, "client_count": 8,
                "partition_injected": True, "old_primary_writable": False,
                "committed_tx_ids_match": True, "wal_ack_loss": False}

    def test_valid_measured_soak(self):
        self.assertEqual(validate_soak(self.report())["status"],
                         "POSTGRES_FENCING_SOAK_VERIFIED")

    def test_short_or_unfenced_soak_rejected(self):
        report = self.report(); report["duration_seconds"] = 30
        with self.assertRaises(SoakEvidenceError): validate_soak(report)
        report = self.report(); report["old_primary_writable"] = True
        with self.assertRaises(SoakEvidenceError): validate_soak(report)


if __name__ == "__main__":
    unittest.main()
