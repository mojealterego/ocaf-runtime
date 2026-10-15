"""Stage 0.41 reliability: real simultaneous TCP WAL + 8-client pgbench network chaos.

Unit tests gate summary metrics only; a separate privileged Linux-netns CI
runs physical PostgreSQL processes and measures database results.
"""
import sys
from pathlib import Path
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_postgres_netns as pg


class NetnsPgbenchChaos(unittest.TestCase):
    def test_true_live_pgbench_wal_partition_and_replica_replay(self):
        p=pg.validate_pgbench_partition(
            clients=8,seconds=20,actual_transactions=2456,
            before=0,primary=2456,standby_b_isolated=0,
            standby_c_healthy=2456,standby_b_healed=2456,
            caught_up_c_after_promote=2456)
        self.assertTrue(p["both_replica_tcp_streams_replayed_commits"])
        self.assertEqual(p["actual_committed_transactions"],2456)
        self.assertFalse(p["inter_kernel_streaming_proved"])

    def test_fail_closed_if_partition_does_not_stop_wal_or_standby_stale(self):
        valid={"clients":8,"seconds":20,"actual_transactions":2456,
               "before":0,"primary":2456,"standby_b_isolated":0,
               "standby_c_healthy":2456,"standby_b_healed":2456,
               "caught_up_c_after_promote":2456}
        failures=[
            {"clients":1},{"seconds":1},{"actual_transactions":0},
            {"before":20},{"primary":2455},{"standby_b_isolated":2456},
            {"standby_c_healthy":0},{"standby_b_healed":0},
            {"caught_up_c_after_promote":0},{"actual_transactions":True},
        ]
        for change in failures:
            with self.subTest(change=change),self.assertRaises(pg.NetnsPGError):
                pg.validate_pgbench_partition(**{**valid,**change})

