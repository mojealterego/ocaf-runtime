"""0.40 actual PostgreSQL primary + two standby lab evidence, fail closed."""
import sys
import unittest
import subprocess
from pathlib import Path
from unittest.mock import patch
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
    def test_cleanup_attempts_remaining_postmasters_after_a_timeout(self):
        calls=[]
        def command(argv,**kwargs):
            node=Path(argv[2]).name
            calls.append((node,argv[-1]))
            if node=="c" and argv[-1]=="stop":
                raise ha.PostgresHAError("timeout")
            return subprocess.CompletedProcess(argv,3 if argv[-1]=="status" else 0)
        with patch.object(ha,"_run",side_effect=command):
            with self.assertRaises(ha.PostgresHAError):
                ha.stop_postmasters("/bin/pg_ctl",{n:Path("/tmp")/n for n in "abc"},set("abc"))
        self.assertIn(("a","stop"),calls)
        self.assertIn(("b","stop"),calls)

    def test_cleanup_does_not_accept_status_errors_as_a_stopped_server(self):
        for code in (0,1,4):
            with self.subTest(code=code),patch.object(ha,"_run",return_value=subprocess.CompletedProcess([],code)):
                with self.assertRaises(ha.PostgresHAError):
                    ha.stop_postmasters("/bin/pg_ctl",{"a":Path("/tmp/a")},{"a"})

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

    def test_witness_rewind_receipt_requires_active_slot_and_replayed_commits(self):
        witness={"authority":"single_local_sqlite_witness",
                 "majority_consensus_tested":False,
                 "former_a_rejoined_by_pg_rewind":True,
                 "physical_replication_slot_active_on_c":True,
                 "former_a_wal_receiver_status":"streaming",
                 "final_committed_event_ids":"1,2,3,4,5",
                 "database_server_side_fencing_tested":False}
        valid={**record(),"witness_lab":witness}
        self.assertEqual(ha.validate_report(valid),valid)
        for change in ({"former_a_rejoined_by_pg_rewind":False},
                       {"physical_replication_slot_active_on_c":False},
                       {"former_a_wal_receiver_status":"stopped"},
                       {"final_committed_event_ids":"1,2,3,4"},
                       {"database_server_side_fencing_tested":True}):
            with self.subTest(change=change),self.assertRaises(ha.PostgresHAError):
                ha.validate_report({**valid,"witness_lab":{**witness,**change}})

    def test_port_assignment_and_path_bounds(self):
        ha.validate_local_pg_ports((25101,25102,25103))
        for invalid in ((25101,25101,25103),(0,25102,25103),
                        (25101,True,25103),(25101,25102,70000)):
            with self.subTest(invalid=invalid),self.assertRaises(ha.PostgresHAError):
                ha.validate_local_pg_ports(invalid)
