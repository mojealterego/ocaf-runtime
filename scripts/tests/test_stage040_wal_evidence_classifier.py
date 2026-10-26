"""Artifact handoff must not satisfy a direct interhost WAL gate."""
import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage040_wal_evidence_classifier as gate


A = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
B = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
C = "cccccccc-cccc-4ccc-8ccc-cccccccccccc"


def artifact():
    return {
        "schema_version": 1, "status": gate.ARTIFACT,
        "boot_ids": {"a": A, "b": B, "c": C},
        "distinct_kernels_observed": 3,
        "three_real_pg_failovers_on_three_distinct_kernels": True,
        "real_local_wal_primary_crash_and_reparent_each_kernel": True,
        "interhost_postgres_streaming_verified": False,
        "host_power_loss_tested": False, "split_brain_fencing_tested": False,
        "stage_040_complete": False, "production_approved": False,
    }


def live():
    receiver = {"status": "streaming", "sender_host": "10.60.0.1",
                "sender_port": 5432}
    return {
        "schema_version": 1, "status": gate.TWO_HOST,
        "boot_ids_before": {"a": A, "b": B},
        "boot_ids_after": {"a": A, "b": B},
        "primary_replication_address": "10.60.0.1",
        "standby_replication_address": "10.60.0.2",
        "standby_receiver_before": receiver.copy(),
        "standby_receiver_after": receiver.copy(),
        "fresh_nonce": "a" * 32,
        "primary_wal_lsn": "0/100",
        "standby_replay_lsn": "0/101",
        "transport_health": {
            "primary_challenge_lsn": "0/100",
            "standby_receive_lsn": "0/101",
            "standby_replay_lsn": "0/101",
            "primary_to_receive_lag_bytes": 0,
            "primary_to_replay_lag_bytes": 0,
            "received_not_before_challenge": True,
            "replay_not_ahead_of_received": True,
            "distinct_kernel_boot_ids": True,
        },
        "direct_tcp_stream_observed": True,
        "third_host_witness_tested": False, "fencing_tested": False,
        "stage_040_complete": False, "production_approved": False,
    }


class WalEvidenceClassifierTests(unittest.TestCase):
    def test_three_kernel_artifact_handoff_remains_non_live(self):
        result = gate.classify(artifact())
        self.assertEqual(result["status"], "ARTIFACT_MEDIATED_LOCAL_FAILOVER_ONLY")
        self.assertFalse(result["direct_tcp_wal_claim_in_validated_receipt"])
        self.assertFalse(result["stage_040_complete"])

    def test_artifact_cannot_be_promoted_by_flag_or_status(self):
        forged = artifact()
        forged["interhost_postgres_streaming_verified"] = True
        with self.assertRaises(gate.InterhostPgError):
            gate.classify(forged)
        forged = artifact()
        forged["status"] = gate.THREE_HOST
        with self.assertRaises(gate.InterhostPgError):
            gate.classify(forged)

    def test_live_two_host_receipt_contract_is_distinct_from_ha(self):
        result = gate.classify(live())
        self.assertEqual(result["status"], "TWO_KERNEL_LIVE_TCP_WAL_RECEIPT_CONTRACT")
        self.assertTrue(result["direct_tcp_wal_claim_in_validated_receipt"])
        self.assertTrue(result["offline_receipt_contract_only"])
        self.assertFalse(result["stage_040_complete"])

    def test_live_claim_requires_stable_kernels_receiver_and_wal_order(self):
        for mutation in (
            lambda x: x["boot_ids_after"].update(b=A),
            lambda x: x["standby_receiver_after"].update(status="stopped"),
            lambda x: x["transport_health"].update(standby_receive_lsn="0/FF"),
            lambda x: x.update(standby_replay_lsn="0/FF"),
        ):
            with self.subTest(mutation=mutation):
                forged = copy.deepcopy(live())
                mutation(forged)
                with self.assertRaises(gate.InterhostPgError):
                    gate.classify(forged)


if __name__ == "__main__":
    unittest.main()
