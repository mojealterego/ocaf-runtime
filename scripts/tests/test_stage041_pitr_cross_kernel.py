"""Stage 0.41.4 external-runner physical basebackup+WAL snapshot transfer tests."""
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage041_pitr_cross_kernel as m

def pitr_report():
    return {
      "schema_version":1,"status":"POSTGRES_REAL_WAL_POINT_IN_TIME_RECOVERY_PASSED",
      "postgres_server_real":True,
      "physical_basebackup_taken_before_wal_transactions":True,
      "postgres_wal_archive_written":True,
      "primary_immediate_crash_executed":True,
      "recovery_signal_and_target_lsn_used":True,
      "restored_postmaster_started":True,
      "expected_rows":35,"actual_recovered_rows":35,
      "post_target_rows":5,"post_target_rows_excluded":True,
      "original_target_checksum":"a"*64,"restored_target_checksum":"a"*64,
      "last_backup_recovery_mode_exited":True,
      "same_kernel":True,"physical_host_crash_tested":False,
      "offsite_wal_archive_tested":False,
      "production_approved":False,"stage_041_complete":False,
      "host_boot_id":"11111111-1111-4111-8111-111111111111",
      "recovery_target_lsn":"0/ABCDEF",
      "target_wal_archived":"000000010000000000000001",
    }

class PhysicalPITRCrossKernelContracts(unittest.TestCase):
    def setUp(self):
        t=tempfile.TemporaryDirectory()
        self.addCleanup(t.cleanup)
        self.root=Path(t.name)
        self.report=self.root/"source.json"
        self.report.write_text(json.dumps(pitr_report()))
        for filename,label,body in (
            ("basebackup.tar.gz","base","16\n".encode()),
            ("archived-wal.tar.gz","wal","WAL".encode()),
        ):
            with tarfile.open(self.root/filename,"w:gz") as tf:
                info=tarfile.TarInfo(label+"/PG_VERSION")
                info.size=len(body)
                tf.addfile(info,io.BytesIO(body))
        self.manifest={
          "schema_version":1,
          "source_boot_id":"11111111-1111-4111-8111-111111111111",
          "source_report_sha256":hashlib.sha256(self.report.read_bytes()).hexdigest(),
          "basebackup_sha256":hashlib.sha256((self.root/"basebackup.tar.gz").read_bytes()).hexdigest(),
          "wal_archive_sha256":hashlib.sha256((self.root/"archived-wal.tar.gz").read_bytes()).hexdigest(),
          "target_lsn":"0/ABCDEF","target_checksum":"a"*64,
          "expected_target_rows":35,"later_committed_rows_excluded":5,
          "run_nonce":"0123456789abcdef","production_approved":False
        }
        (self.root/"manifest.json").write_text(json.dumps(self.manifest))

    def valid(self,boot="22222222-2222-4222-8222-222222222222"):
        return m.inspect_bundle(self.root,boot)

    def test_verified_transfer_requires_distinct_kernel_source_wal_and_digest(self):
        v=self.valid()
        self.assertTrue(v["distinct_linux_kernels"])
        self.assertEqual(v["target_lsn"],"0/ABCDEF")
        self.assertFalse(v["production_approved"])

    def test_archive_byte_mutation_and_same_kernel_rejected(self):
        with self.assertRaises(m.CrossPITRError):
            self.valid("11111111-1111-4111-8111-111111111111")
        wal=self.root/"archived-wal.tar.gz"
        wal.write_bytes(b"tampered")
        with self.assertRaises(m.CrossPITRError):
            self.valid()

    def test_unknown_manifest_and_relabelled_source_rejected(self):
        self.manifest["physical_host_crash_tested"]=True
        (self.root/"manifest.json").write_text(json.dumps(self.manifest))
        with self.assertRaises(m.CrossPITRError):
            self.valid()

    def test_unsafe_tar_member_symlink_escape_rejected(self):
        tar=self.root/"basebackup.tar.gz"
        with tarfile.open(tar,"w:gz") as tf:
            info=tarfile.TarInfo("base/escape")
            info.type=tarfile.SYMTYPE
            info.linkname="/etc/passwd"
            tf.addfile(info)
        manifest=json.loads((self.root/"manifest.json").read_text())
        manifest["basebackup_sha256"]=hashlib.sha256(tar.read_bytes()).hexdigest()
        (self.root/"manifest.json").write_text(json.dumps(manifest))
        with self.assertRaises(m.CrossPITRError):
            m.inspect_bundle(self.root,"22222222-2222-4222-8222-222222222222",
                             validate_tar=True)

    def test_recovered_target_must_exclude_later_transactions(self):
        v=self.valid()
        good=m.validate_restored(v,35,"a"*64,0,245)
        self.assertTrue(good["real_separate_kernel_wal_pitr_proved"])
        self.assertFalse(good["production_approved"])
        for count,checksum,later in ((34,"a"*64,0),(35,"b"*64,0),(35,"a"*64,5)):
            with self.subTest(count=count,checksum=checksum,later=later),self.assertRaises(m.CrossPITRError):
                m.validate_restored(v,count,checksum,later,100)
