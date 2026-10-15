"""0.40 actual PostgreSQL primary + two standby lab evidence, fail closed."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_postgres_ha as ha

def record():
    return {
        "schema_version":1,
        "status":"POSTGRES_3_PROCESSES_STREAMING_FAILOVER_PASSED",
        "nodes_count":3,"distinct_postgres_ports":True,
        "primary_was_writable":True,
        "both_standbys_replayed_pre_crash_wal":True,
        "old_primary_killed_immediate":True,
        "old_primary_unreachable_after_crash":True,
        "standby_b_promoted_writable":True,
        "standby_c_followed_promoted_b":True,
        "standby_c_replayed_post_promotion_wal":True,
        "all_three_same_kernel":True,
        "host_power_loss_tested":False,
        "production_approved":False,
        "stage_040_complete":False,
    }

class HAProofContracts(unittest.TestCase):
    def test_admits_actual_scope_and_fencing_only(self):
        r=ha.validate_report(record())
        self.assertTrue(r["standby_c_followed_promoted_b"])
        self.assertFalse(r["stage_040_complete"])

    def test_denies_gaps_and_false_approval(self):
        ref=record()
        for key in ("distinct_postgres_ports","primary_was_writable",
                    "both_standbys_replayed_pre_crash_wal",
                    "old_primary_killed_immediate",
                    "old_primary_unreachable_after_crash",
                    "standby_b_promoted_writable",
                    "standby_c_followed_promoted_b",
                    "standby_c_replayed_post_promotion_wal"):
            with self.subTest(key=key),self.assertRaises(ha.PostgresHAError):
                ha.validate_report({**ref,key:False})
        for change in ({"nodes_count":2},{"production_approved":True},
                       {"stage_040_complete":True},{"all_three_same_kernel":False},
                       {"host_power_loss_tested":True},
                       {"status":"fabricated"}):
            with self.subTest(change=change),self.assertRaises(ha.PostgresHAError):
                ha.validate_report({**ref,**change})

    def test_port_assignment_and_path_bounds(self):
        ha.validate_local_pg_ports((25101,25102,25103))
        for invalid in ((25101,25101,25103),(0,25102,25103),
                        (25101,True,25103),(25101,25102,70000)):
            with self.subTest(invalid=invalid),self.assertRaises(ha.PostgresHAError):
                ha.validate_local_pg_ports(invalid)
