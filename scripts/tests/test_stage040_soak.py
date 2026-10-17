import copy
from datetime import datetime, timedelta, timezone
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_soak import SoakEvidenceError, validate_soak


class SoakEvidenceTests(unittest.TestCase):
    def report(self):
        start = datetime(2026, 10, 17, tzinfo=timezone.utc)
        samples = []
        for i in range(61):
            phase = ("baseline" if i == 0 else "partition" if i < 30 else
                     "failover" if i < 60 else "old_primary_return")
            txids = [str(j) for j in range(1, i + 2)]
            timeline = 1 if i < 30 else 2
            samples.append({
                "sequence": i, "monotonic_seconds": float(i * 60),
                "observed_at_utc": (start + timedelta(seconds=i * 60)).isoformat(),
                "phase": phase, "active_clients": 8,
                "acknowledged_tx_ids": txids, "replayed_tx_ids": list(txids),
                "wal_acks": [{"tx_id": txid, "lsn": f"0/{int(txid) * 16:X}",
                              "timeline": 1 if int(txid) <= 30 else timeline}
                             for txid in txids],
                "replay_lsn": f"0/{(i + 1) * 16:X}", "timeline": timeline,
                "old_primary_write_attempt": None if i == 0 else
                    {"sqlstate": "25006", "rows_affected": 0},
            })
        return {"schema_version": 2, "duration_seconds": 3600.0, "client_count": 8,
                "evidence_source": "operator_supplied", "samples": samples,
                "timeline_history": [{"parent_timeline": 1, "child_timeline": 2,
                                      "fork_lsn": "0/1E0"}]}

    def test_consistent_measured_sequence_is_not_authenticated_observation(self):
        result = validate_soak(self.report())
        self.assertEqual(result["status"], "SOAK_EVIDENCE_CONSISTENT")
        self.assertEqual(result["duration_seconds"], 3600)
        self.assertEqual(result["committed_transactions"], 61)
        self.assertFalse(result["observation_authenticated"])
        self.assertFalse(result["real_soak_verified"])
        self.assertFalse(result["stage_040_complete"])

    def test_boolean_only_report_is_rejected(self):
        with self.assertRaises(SoakEvidenceError):
            validate_soak({"duration_seconds": 3600, "client_count": 8,
                           "partition_injected": True, "old_primary_writable": False,
                           "committed_tx_ids_match": True, "wal_ack_loss": False})

    def test_duration_must_be_measured_finite_and_positive(self):
        for value in (-1, 0, 30, float("inf"), float("nan"), True, "3600"):
            report = self.report(); report["duration_seconds"] = value
            with self.subTest(value=value), self.assertRaises(SoakEvidenceError):
                validate_soak(report)
        for value in (-1, 0, float("inf"), float("nan"), True):
            with self.subTest(minimum=value), self.assertRaises(SoakEvidenceError):
                validate_soak(self.report(), minimum_seconds=value)

    def test_two_timestamps_do_not_substitute_for_measured_sequence(self):
        report = self.report(); report["samples"] = report["samples"][::60]
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_unordered_or_nonfinite_timestamps_are_rejected(self):
        for key, value in (("monotonic_seconds", float("nan")),
                           ("monotonic_seconds", -1), ("monotonic_seconds", 0),
                           ("observed_at_utc", "2026-10-17T00:00:00"),
                           ("observed_at_utc", "not-a-time"),
                           ("observed_at_utc", "2026-10-17T00:00:00+00:00"),
                           ("sequence", True), ("sequence", 0)):
            report = self.report(); report["samples"][1][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(SoakEvidenceError):
                validate_soak(report)

    def test_duration_is_derived_from_sample_span(self):
        report = self.report(); report["duration_seconds"] = 7200
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_replay_must_preserve_exact_acknowledged_txids(self):
        for value in (["1"], [str(i) for i in range(1, 61)] + ["999"], []):
            report = self.report(); report["samples"][-1]["replayed_tx_ids"] = value
            with self.subTest(value=value), self.assertRaises(SoakEvidenceError):
                validate_soak(report)

    def test_acknowledgements_cannot_disappear_or_be_duplicated(self):
        for key in ("acknowledged_tx_ids", "replayed_tx_ids"):
            report = self.report(); report["samples"][-1][key][0] = "2"
            with self.subTest(key=key), self.assertRaises(SoakEvidenceError):
                validate_soak(report)
        report = self.report()
        sample = report["samples"][-1]
        for key in ("acknowledged_tx_ids", "replayed_tx_ids"):
            sample[key] = sample[key][1:]
        sample["wal_acks"] = sample["wal_acks"][1:]
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_txids_require_canonical_unsigned_xid8(self):
        for value in (True, 1, "01", "0", "-1", str(1 << 64), "a"):
            report = self.report(); report["samples"][0]["acknowledged_tx_ids"] = [value]
            with self.subTest(value=value), self.assertRaises(SoakEvidenceError):
                validate_soak(report)

    def test_wal_ack_and_replay_positions_cannot_be_missing_or_lost(self):
        for key, value in (("wal_acks", []), ("replay_lsn", "0/1"),
                           ("replay_lsn", "0/100000000"), ("timeline", 0)):
            report = self.report(); report["samples"][-1][key] = value
            with self.subTest(key=key), self.assertRaises(SoakEvidenceError):
                validate_soak(report)
        report = self.report(); report["samples"][-1]["wal_acks"][0]["lsn"] = "0/1"
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_all_fault_phases_and_workload_progress_are_required(self):
        report = self.report(); report["samples"][-1]["phase"] = "failover"
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)
        report = self.report()
        for sample in report["samples"][1:]:
            sample["acknowledged_tx_ids"] = ["1"]
            sample["replayed_tx_ids"] = ["1"]
            sample["wal_acks"] = copy.deepcopy(report["samples"][0]["wal_acks"])
            sample["replay_lsn"] = "0/10"
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_old_primary_return_requires_observed_write_rejection(self):
        for value in (None, {"sqlstate": "00000", "rows_affected": 1},
                      {"sqlstate": "25006", "rows_affected": True},
                      {"sqlstate": "08006", "rows_affected": 0}):
            report = self.report(); report["samples"][-1]["old_primary_write_attempt"] = value
            with self.subTest(value=value), self.assertRaises(SoakEvidenceError):
                validate_soak(report)

    def test_timeline_numbers_alone_do_not_establish_wal_ancestry(self):
        for history in ([], None,
                        [{"parent_timeline": 3, "child_timeline": 2, "fork_lsn": "0/1E0"}],
                        [{"parent_timeline": 1, "child_timeline": 2, "fork_lsn": "0/1"}],
                        [{"parent_timeline": 1, "child_timeline": 2, "fork_lsn": "0/FFF"}]):
            report = self.report(); report["timeline_history"] = history
            with self.subTest(history=history), self.assertRaises(SoakEvidenceError):
                validate_soak(report)
        report = self.report()
        for sample in report["samples"][30:]:
            sample["timeline"] = 3
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_simultaneous_same_timeline_failover_cannot_claim_promotion(self):
        report = self.report(); report["timeline_history"] = []
        for sample in report["samples"]:
            sample["timeline"] = 1
            for ack in sample["wal_acks"]:
                ack["timeline"] = 1
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_client_count_and_source_must_be_explicit(self):
        for key, value in (("client_count", True), ("client_count", 7),
                           ("evidence_source", None), ("evidence_source", "verified")):
            report = self.report(); report[key] = value
            with self.subTest(key=key), self.assertRaises(SoakEvidenceError):
                validate_soak(report)
        report = self.report(); report["samples"][30]["active_clients"] = 0
        with self.assertRaises(SoakEvidenceError):
            validate_soak(report)

    def test_simulation_cannot_be_promoted_to_real_evidence(self):
        report = self.report(); report["evidence_source"] = "simulation"
        result = validate_soak(report)
        self.assertFalse(result["real_soak_verified"])
        self.assertFalse(result["production_approved"])


if __name__ == "__main__":
    unittest.main()
