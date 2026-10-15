"""0.40.1 — separate GitHub Linux kernel IDs per independently verified PostgreSQL failover lab."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_independent_pg_hosts as hosts


def node(role,boot):
    return {
        "schema_version":1,
        "status":"POSTGRES_3_PROCESSES_STREAMING_FAILOVER_PASSED",
        "kernel_boot_id":boot,
        "nodes_count":3,"distinct_postgres_ports":True,
        "primary_was_writable":True,
        "both_standbys_replayed_pre_crash_wal":True,
        "old_primary_killed_immediate":True,
        "old_primary_unreachable_after_crash":True,
        "standby_b_promoted_writable":True,
        "standby_c_followed_promoted_b":True,
        "standby_c_replayed_post_promotion_wal":True,
        "standby_c_wal_receiver_status":"streaming",
        "all_three_same_kernel":True,
        "three_physically_independent_hosts":False,
        "host_power_loss_tested":False,
        "production_approved":False,
        "stage_040_complete":False,
    }

class IndependentPGHostChecks(unittest.TestCase):
    def setUp(self):
        self.p={
           "a":node("a","11111111-1111-4111-8111-111111111111"),
           "b":node("b","22222222-2222-4222-8222-222222222222"),
           "c":node("c","33333333-3333-4333-8333-333333333333"),
        }

    def test_three_separate_kernel_pg_failover_probes_but_not_real_interhost_ha(self):
        x=hosts.compare_three_hosts(self.p)
        self.assertEqual(x["distinct_kernels_observed"],3)
        self.assertTrue(x["three_real_pg_failovers_on_three_distinct_kernels"])
        self.assertFalse(x["interhost_postgres_streaming_verified"])
        self.assertFalse(x["stage_040_complete"])

    def test_same_kernel_and_missing_host_proofs_fail(self):
        for change in (
            {"b":{**self.p["b"],"kernel_boot_id":self.p["a"]["kernel_boot_id"]}},
            {"c":{**self.p["c"],"old_primary_killed_immediate":False}},
            {"c":{**self.p["c"],"stage_040_complete":True}},
            {"a":{**self.p["a"],"three_physically_independent_hosts":True}},
            {"b":{**self.p["b"],"standby_c_wal_receiver_status":"stopped"}},
        ):
            with self.subTest(change=change),self.assertRaises(hosts.MultiPgHostError):
                hosts.compare_three_hosts({**self.p,**change})
        with self.assertRaises(hosts.MultiPgHostError):
            hosts.compare_three_hosts({"a":self.p["a"],"b":self.p["b"]})

    def test_wrong_claim_of_interhost_replication_not_accepted(self):
        with self.assertRaises(hosts.MultiPgHostError):
            hosts.compare_three_hosts({**self.p,"a":{**self.p["a"],"interhost_postgres_streaming_verified":True}})
