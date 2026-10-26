"""Negative cases for the measured local PostgreSQL WAL failover chain."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_failover_trace import FailoverTraceError, validate_trace


def receipt():
    return {
        "schema_version": 1, "scope": "ONE_KERNEL_THREE_POSTMASTERS",
        "a_timeline_before_crash": "00000001",
        "b_timeline_after_promotion": "00000002",
        "c_timeline_after_promotion": "00000003",
        "a_commit_lsn": "0/100", "b_replay_lsn_before_promotion": "0/100",
        "c_replay_lsn_before_promotion": "0/100",
        "b_commit_lsn": "0/200", "c_replay_lsn_after_b_commit": "0/200",
        "c_commit_lsn": "0/300", "a_replay_lsn_after_c_commit": "0/300",
        "committed_markers_replayed": True, "all_client_rpo_zero_proved": False,
        "external_fencing_proved": False, "stage_040_complete": False,
    }


class TraceTests(unittest.TestCase):
    def test_accepts_measured_chain_without_production_claim(self):
        self.assertEqual(validate_trace(receipt()), receipt())

    def test_rejects_missing_replay_regressed_timeline_and_false_claims(self):
        for change in (
            {"b_replay_lsn_before_promotion": "0/FF"},
            {"c_replay_lsn_after_b_commit": "0/1FF"},
            {"a_replay_lsn_after_c_commit": "0/2FF"},
            {"b_timeline_after_promotion": "00000001"},
            {"c_timeline_after_promotion": "00000002"},
            {"a_commit_lsn": "bad"},
            {"all_client_rpo_zero_proved": True},
            {"external_fencing_proved": True},
            {"stage_040_complete": True},
        ):
            with self.subTest(change=change), self.assertRaises(FailoverTraceError):
                validate_trace({**receipt(), **change})


if __name__ == "__main__":
    unittest.main()
