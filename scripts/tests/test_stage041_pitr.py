"""Stage 0.41.3 actual PostgreSQL WAL point-in-time recovery contracts."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage041_pitr as pitr

def evidence():
    return {
      "schema_version":1,
      "status":"POSTGRES_REAL_WAL_POINT_IN_TIME_RECOVERY_PASSED",
      "postgres_server_real":True,
      "physical_basebackup_taken_before_wal_transactions":True,
      "postgres_wal_archive_written":True,
      "primary_immediate_crash_executed":True,
      "recovery_signal_and_target_lsn_used":True,
      "restored_postmaster_started":True,
      "expected_rows":35,
      "actual_recovered_rows":35,
      "post_target_rows":5,
      "post_target_rows_excluded":True,
      "original_target_checksum":"a"*64,
      "restored_target_checksum":"a"*64,
      "last_backup_recovery_mode_exited":True,
      "same_kernel":True,
      "physical_host_crash_tested":False,
      "offsite_wal_archive_tested":False,
      "production_approved":False,
      "stage_041_complete":False,
    }

class PITR(unittest.TestCase):
    def test_accepts_only_exact_wal_pitr_target_and_excludes_later_writes(self):
        result=pitr.validate_pitr(evidence())
        self.assertTrue(result["pitr_exact_target_restored"])
        self.assertFalse(result["production_approved"])

    def test_fail_closed_on_missing_target_or_claim_of_physical_dr(self):
        for field in ("postgres_server_real",
                      "physical_basebackup_taken_before_wal_transactions",
                      "postgres_wal_archive_written",
                      "primary_immediate_crash_executed",
                      "recovery_signal_and_target_lsn_used",
                      "restored_postmaster_started",
                      "post_target_rows_excluded",
                      "last_backup_recovery_mode_exited"):
            with self.subTest(field=field), self.assertRaises(pitr.PITRError):
                pitr.validate_pitr({**evidence(),field:False})
        for values in (
            {"expected_rows":35,"actual_recovered_rows":34},
            {"post_target_rows":0},
            {"original_target_checksum":"invalid"},
            {"restored_target_checksum":"b"*64},
            {"same_kernel":False},
            {"physical_host_crash_tested":True},
            {"offsite_wal_archive_tested":True},
            {"production_approved":True},
            {"stage_041_complete":True},
        ):
            with self.subTest(values=values), self.assertRaises(pitr.PITRError):
                pitr.validate_pitr({**evidence(),**values})

    def test_validate_untrusted_config_fields(self):
        pitr.safe_wal_name("000000010000000000000002")
        for name in ("../etc/passwd","x", "000000010000000000000002;rm", "-rm"):
            with self.subTest(name=name),self.assertRaises(pitr.PITRError):
                pitr.safe_wal_name(name)
