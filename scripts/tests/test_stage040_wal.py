import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_wal import WalEvidenceError, validate_stream


class WalEvidenceTests(unittest.TestCase):
    def kwargs(self):
        return dict(primary_boot_id="a" * 32, standby_boot_id="b" * 32,
                    wal_receiver_status="streaming", primary_lsn="0/20",
                    standby_lsn="0/18", checksum_primary="c" * 64,
                    checksum_standby="c" * 64)

    def test_valid_distinct_kernel_stream(self):
        self.assertEqual(validate_stream(**self.kwargs())["status"],
                         "DIRECT_INTERHOST_WAL_STREAM_VERIFIED")

    def test_shared_kernel_is_rejected(self):
        args = self.kwargs(); args["standby_boot_id"] = args["primary_boot_id"]
        with self.assertRaises(WalEvidenceError): validate_stream(**args)

    def test_non_streaming_or_checksum_mismatch_is_rejected(self):
        args = self.kwargs(); args["wal_receiver_status"] = "catchup"
        with self.assertRaises(WalEvidenceError): validate_stream(**args)
        args = self.kwargs(); args["checksum_standby"] = "d" * 64
        with self.assertRaises(WalEvidenceError): validate_stream(**args)


if __name__ == "__main__":
    unittest.main()
