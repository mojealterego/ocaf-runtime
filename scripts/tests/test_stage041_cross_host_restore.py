"""Stage 0.41.2: cross-kernel recovery is real only with pinned source bytes and new PG."""
from pathlib import Path
import hashlib
import json
import tempfile
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage041_cross_host_restore as recovery


def source_report(snapshot):
    return {
        "schema_version": 1, "status": "POSTGRES_REAL_BACKUP_RESTORE_IDENTICAL_RECORDS_PASSED",
        "observed_at_utc": "2026-10-15T12:00:00+00:00",
        "original_rows": 25, "restored_rows": 25,
        "original_checksum": "b" * 64, "restored_checksum": "b" * 64,
        "backup_file": "snapshot.pgcustom", "backup_sha256": hashlib.sha256(snapshot).hexdigest(),
        "backup_size_bytes": len(snapshot),
        "tampered_backup_rejected": True, "postgres_server_real": True,
        "backup_was_custom_pg_dump": True, "restore_was_pg_restore": True,
        "same_kernel_source_and_restore": True, "cross_host_restore_tested": False,
        "production_approved": False, "stage_041_complete": False
    }


class TestCrossHostRecovery(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.snapshot = self.root / "snapshot.pgcustom"
        self.snapshot.write_bytes(b"PGDMP" + bytes(range(1, 50)))
        self.source = self.root / "source.json"
        self.source.write_text(json.dumps(source_report(self.snapshot.read_bytes()), sort_keys=True))
        self.manifest = self.root / "manifest.json"
        self.manifest.write_text(json.dumps({
            "schema_version": 1,
            "source_boot_id": "11111111-1111-4111-8111-111111111111",
            "source_report_sha256": hashlib.sha256(self.source.read_bytes()).hexdigest(),
            "snapshot_sha256": hashlib.sha256(self.snapshot.read_bytes()).hexdigest(),
            "snapshot_size_bytes": self.snapshot.stat().st_size,
            "run_nonce": "0123456789abcdef",
            "production_approved": False
        }, sort_keys=True))

    def check(self, boot="22222222-2222-4222-8222-222222222222"):
        return recovery.validate_transfer(self.snapshot, self.source, self.manifest, boot)

    def test_manifest_ties_actual_snapshot_and_source_report_to_distinct_kernel(self):
        v=self.check()
        self.assertEqual(v["original_rows"], 25)
        self.assertTrue(v["distinct_linux_kernels"])
        self.assertFalse(v["production_approved"])

    def test_cross_kernel_result_is_not_durable_remote_dr(self):
        v=self.check()
        report=recovery.validate_recovered_rows(v, 25, "b"*64, 381)
        self.assertTrue(report["actual_postgres_restore_confirmed"])
        self.assertEqual(report["rpo_at_backup_boundary_rows_lost"], 0)
        self.assertFalse(report["physical_host_disaster_recovery_tested"])
        self.assertFalse(report["stage_041_complete"])
        for rows, sha in ((24,"b"*64),(25,"c"*64),(True,"b"*64)):
            with self.subTest(rows=rows), self.assertRaises(recovery.CrossHostRestoreError):
                recovery.validate_recovered_rows(v,rows,sha,10)

    def test_tampered_snapshot_and_wrong_boot_fail_closed(self):
        self.snapshot.write_bytes(b"PGDMPcorrupted")
        with self.assertRaises(recovery.CrossHostRestoreError):
            self.check()
        self.snapshot.write_bytes(b"PGDMP"+bytes(range(1,50)))
        with self.assertRaises(recovery.CrossHostRestoreError):
            self.check("11111111-1111-4111-8111-111111111111")

    def test_report_mutation_and_manifest_tampering_rejected(self):
        self.source.write_text(self.source.read_text().replace('"original_rows": 25','"original_rows": 26'))
        with self.assertRaises(recovery.CrossHostRestoreError):
            self.check()

    def test_symlink_and_duplicates_rejected(self):
        link=self.root/"snapshot-link"
        link.symlink_to(self.snapshot)
        with self.assertRaises(recovery.CrossHostRestoreError):
            recovery.validate_transfer(link,self.source,self.manifest,"22222222-2222-4222-8222-222222222222")
        self.manifest.write_text('{"schema_version":1,"schema_version":1}')
        with self.assertRaises(recovery.CrossHostRestoreError):
            self.check()

    def test_unknown_or_synthetic_approval_fields_rejected(self):
        d=json.loads(self.manifest.read_text())
        d["production_approved"]=True
        self.manifest.write_text(json.dumps(d))
        with self.assertRaises(recovery.CrossHostRestoreError):
            self.check()
