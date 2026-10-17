import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_wal import WalEvidenceError, _lsn, validate_stream


class WalEvidenceTests(unittest.TestCase):
    def kwargs(self):
        return dict(
            primary_boot_id="d38c90e0-38da-4db2-9d90-1734e6a19a70",
            standby_boot_id="3759c720-cd60-4733-9d93-c2fda01ddf4c",
            primary_host_id="a" * 32, standby_host_id="b" * 32,
            primary_timeline=1, standby_timeline=1,
            evidence_source="operator_supplied", wal_receiver_status="streaming",
            primary_lsn="0/20", standby_lsn="0/20",
            checksum_primary="c" * 64, checksum_standby="c" * 64)

    def test_consistent_supplied_evidence_is_not_authenticated_observation(self):
        result = validate_stream(**self.kwargs())
        self.assertEqual(result["status"], "WAL_STREAM_EVIDENCE_CONSISTENT")
        self.assertFalse(result["observation_authenticated"])
        self.assertFalse(result["interhost_streaming_verified"])
        self.assertFalse(result["stage_040_complete"])
        self.assertEqual(result["evidence_source"], "operator_supplied")

    def test_malformed_and_shared_identities_are_rejected(self):
        cases = [
            ("primary_boot_id", "a" * 32),
            ("primary_boot_id", "00000000-0000-0000-0000-000000000000"),
            ("primary_boot_id", "d38c90e038da4db29d901734e6a19a70"),
            ("primary_boot_id", "D38C90E0-38DA-4DB2-9D90-1734E6A19A70"),
            ("primary_boot_id", "d38c90e0-38da-1db2-9d90-1734e6a19a70"),
            ("standby_boot_id", self.kwargs()["primary_boot_id"]),
            ("standby_host_id", self.kwargs()["primary_host_id"]),
            ("primary_host_id", "0" * 32), ("primary_host_id", "unknown"),
        ]
        for field, value in cases:
            with self.subTest(field=field, value=value):
                args = self.kwargs(); args[field] = value
                with self.assertRaises(WalEvidenceError):
                    validate_stream(**args)

    def test_lagging_or_ahead_standby_is_not_caught_up(self):
        for lsn in ("0/18", "0/21"):
            args = self.kwargs(); args["standby_lsn"] = lsn
            with self.assertRaises(WalEvidenceError):
                validate_stream(**args)

    def test_lsn_halves_are_unsigned_32_bit(self):
        self.assertEqual(_lsn("FFFFFFFF/FFFFFFFF"), (1 << 64) - 1)
        self.assertEqual(_lsn("0/0"), 0)
        for value in ("100000000/0", "0/100000000", "0/000000000", "-1/0",
                      "0/-1", "0/0\n", True, None):
            with self.subTest(value=value):
                with self.assertRaises(WalEvidenceError):
                    _lsn(value)

    def test_timeline_must_be_equal_and_bounded(self):
        for value in (0, -1, True, 1.0, "1", 2, 1 << 32, None):
            args = self.kwargs(); args["standby_timeline"] = value
            with self.subTest(value=value):
                with self.assertRaises(WalEvidenceError):
                    validate_stream(**args)

    def test_checksums_are_nonempty_sha256_and_equal(self):
        for value in ("", "hash", "g" * 64, "c" * 63, "d" * 64, None):
            args = self.kwargs(); args["checksum_standby"] = value
            with self.subTest(value=value):
                with self.assertRaises(WalEvidenceError):
                    validate_stream(**args)

    def test_receiver_and_explicit_source_are_required(self):
        for field, value in (("wal_receiver_status", "catchup"),
                             ("evidence_source", None), ("evidence_source", "verified")):
            args = self.kwargs(); args[field] = value
            with self.assertRaises(WalEvidenceError):
                validate_stream(**args)

    def test_simulation_never_receives_real_verified_status(self):
        args = self.kwargs(); args["evidence_source"] = "simulation"
        result = validate_stream(**args)
        self.assertFalse(result["interhost_streaming_verified"])
        self.assertFalse(result["production_approved"])


if __name__ == "__main__":
    unittest.main()
