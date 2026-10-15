"""Stage 0.41 real PostgreSQL backup/restore, tampering and RPO proof contract."""
import hashlib
import tempfile
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage041_backup_restore as backup

class BackupVerification(unittest.TestCase):
    def test_backup_inode_sha256_is_pinned_and_tamper_detected(self):
        with tempfile.TemporaryDirectory() as td:
            path=Path(td)/"snapshot.dump"
            path.write_bytes(b"test fixture for backup integrity\x00")
            digest=backup.hash_backup(path)
            self.assertEqual(digest,hashlib.sha256(path.read_bytes()).hexdigest())
            backup.require_backup_digest(path,digest)
            path.write_bytes(b"tampered\x00")
            with self.assertRaises(backup.BackupRestoreError):
                backup.require_backup_digest(path,digest)

    def test_actual_restore_requires_matching_keys_and_data_checksum(self):
        a={"backup_size_bytes":1024,"restored_rows":25,
           "original_rows":25,"original_checksum":"a"*64,
           "restored_checksum":"a"*64,"tampered_backup_rejected":True,
           "backup_was_custom_pg_dump":True,"restore_was_pg_restore":True,
           "postgres_server_real":True,"production_approved":False}
        self.assertTrue(backup.verify_restore(a)["restore_exact_match"])
        for key,v in [("restored_rows",24),("restored_checksum","b"*64),
                      ("tampered_backup_rejected",False),("postgres_server_real",False),
                      ("production_approved",True)]:
            with self.subTest(key=key),self.assertRaises(backup.BackupRestoreError):
                backup.verify_restore({**a,key:v})
