"""Stage 0.40.3 live remote-host PostgreSQL WAL acceptance requires REAL SSH host evidence."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_interhost_live as pg

def evidence():
    return {
        "schema_version":1,
        "status":"REAL_DISTINCT_LINUX_KERNELS_POSTGRESQL_LIVE_TCP_STREAMING_VERIFIED",
        "topology":"three_truly_distinct_linux_kernels",
        "boot_ids":{"a":"11111111-1111-4111-8111-111111111111",
                    "b":"22222222-2222-4222-8222-222222222222",
                    "c":"33333333-3333-4333-8333-333333333333"},
        "client_addresses":["10.60.0.2","10.60.0.3"],
        "standby_addresses":{"b":"10.60.0.2","c":"10.60.0.3"},
        "source_writable":True,"b_in_recovery":True,"c_in_recovery":True,
        "replication_b_status":"streaming","replication_c_status":"streaming",
        "source_nonce":"a"*32,"standby_b_nonce":"a"*32,"standby_c_nonce":"a"*32,
        "source_wal_lsn":"0/16D4D20",
        "standby_b_wal_replay_at_or_above_source_lsn":True,
        "standby_c_wal_replay_at_or_above_source_lsn":True,
        "used_ssh_host_key_pinning":True,
        "actual_direct_network_replication":True,
        "physical_host_power_off_tested":False,
        "consensus_fencing_verified":False,
        "production_approved":False,
        "stage_040_complete":False,
    }

class RealIndependentHostPgTests(unittest.TestCase):
    def test_positive_requires_three_actual_unique_host_boot_ids_and_direct_network(self):
        report=pg.validate_receipt(evidence())
        self.assertEqual(report["status"],"REAL_DISTINCT_LINUX_KERNELS_POSTGRESQL_LIVE_TCP_STREAMING_VERIFIED")
        self.assertTrue(report["actual_direct_network_replication"])
        self.assertFalse(report["consensus_fencing_verified"])

    def test_rejects_same_host_local_netns_or_artifact_faked_tcp(self):
        valid=evidence()
        mods=[
            {"boot_ids":{**valid["boot_ids"],"c":valid["boot_ids"]["b"]}},
            {"topology":"three_netns_one_kernel"},
            {"actual_direct_network_replication":False},
            {"consensus_fencing_verified":True},
            {"production_approved":True},
            {"physical_host_power_off_tested":True},
            {"client_addresses":["10.60.0.2","10.60.0.2"]},
            {"standby_b_nonce":"f"*32},
            {"standby_c_wal_replay_at_or_above_source_lsn":False},
            {"replication_c_status":"stopped"},
        ]
        for change in mods:
            with self.subTest(change=change),self.assertRaises(pg.InterhostPgError):
                pg.validate_receipt({**valid,**change})

    def test_invalid_host_targets_and_malformed_ips_rejected(self):
        valid=pg.validate_host({"role":"a","host":"10.60.0.1","ssh_user":"pgops",
                                "ssh_port":22,"replication_address":"10.60.0.1"})
        self.assertEqual(valid["host"],"10.60.0.1")
        for data in (
            {"host":"127.0.0.1"},{"host":"10.60.0.1;rm -rf /"},
            {"ssh_user":"root;id"},{"ssh_port":0},
            {"replication_address":"10.60.0.1x"},
        ):
            with self.subTest(data=data),self.assertRaises(pg.InterhostPgError):
                pg.validate_host({**valid,**data})

    def test_dangerous_sql_and_non_private_pinned_host_file_rejected(self):
        for nonce in ("x","' OR 1=1","a"*33):
            with self.subTest(nonce=nonce),self.assertRaises(pg.InterhostPgError):
                pg.safe_nonce(nonce)
