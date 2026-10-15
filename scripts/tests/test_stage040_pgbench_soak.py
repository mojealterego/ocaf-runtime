"""0.40.2: real multi-client PostgreSQL pgbench WAL soak/replica parity receipt."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_postgres_ha as h

class PgBenchReliabilityContracts(unittest.TestCase):
    def test_pg_basebackup_must_force_checkpoint_before_heavy_wal_soak(self):
        argv=h.pg_basebackup_command("/usr/lib/postgresql/16/bin/pg_basebackup",15432,Path("/tmp/ocaf40-b"))
        self.assertIn("--checkpoint=fast",argv)
        self.assertIn("--wal-method=stream",argv)
        self.assertEqual(argv[-1],"--write-recovery-conf")
        for args in ((15432,Path("../unsafe")),(0,Path("/tmp/ocaf40-b")),(True,Path("/tmp/ocaf40-b"))):
            with self.subTest(args=args),self.assertRaises(h.PostgresHAError):
                h.pg_basebackup_command("/usr/lib/postgresql/16/bin/pg_basebackup",args[0],args[1])

    def test_pgbench_parses_actual_committed_load(self):
        raw="""pgbench (16.10)\ntransaction type: <builtin: TPC-B (sort of)>\nscaling factor: 1\nnumber of clients: 8\nnumber of threads: 2\nduration: 30 s\nnumber of transactions actually processed: 12345\nlatency average = 5.6 ms\ntps = 411.12 (without initial connection time)\n"""
        r=h.validate_pgbench_load(raw,30,8)
        self.assertEqual(r["transactions"],12345)
        self.assertGreater(r["tps"],0)
        self.assertTrue(r["actual_pgbench_completed"])

    def test_invalid_output_short_benchmark_and_zero_wal_are_denied(self):
        good="number of clients: 8\nduration: 30 s\nnumber of transactions actually processed: 2345\ntps = 80.1 (without initial connection time)"
        for s,seconds,clients in (
            ("",30,8),
            ("number of transactions actually processed: 0",30,8),
            (good,1,8),
            (good,30,4),
            (good.replace("2345","0"),30,8),
            (good.replace("80.1","0.0"),30,8),
        ):
            with self.subTest(seconds=seconds,clients=clients,s=s),self.assertRaises(h.PostgresHAError):
                h.validate_pgbench_load(s,seconds,clients)

    def test_hash_must_match_both_standbys(self):
        r=h.validate_soak_replay("123:456:789","123:456:789","123:456:789")
        self.assertTrue(r["both_streaming_replicas_replayed_pgbench_dataset"])
        for bad in (("1","2","1"),("a","a","b"),("","","")):
            with self.subTest(bad=bad),self.assertRaises(h.PostgresHAError):
                h.validate_soak_replay(*bad)
