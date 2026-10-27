"""0.41.6 authenticated backup envelope and recovery denial cases."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage041_encrypted_backup import (
    EncryptedBackupError, seal_snapshot, unseal_snapshot,
)
import stage041_encrypted_backup as backup


class EncryptedBackupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "snapshot.pgcustom"
        self.source.write_bytes(b"PGDMP\x00genuine PostgreSQL test snapshot\x00")
        self.envelope = self.root / "envelope"
        self.envelope.mkdir()
        self.key = b"k" * 32
        self.manifest = seal_snapshot(self.source, self.envelope, key=self.key,
                                      generation=12)

    def recover(self, **kwargs):
        return unseal_snapshot(self.envelope, self.root / "restored.pgcustom",
                               key=kwargs.pop("key", self.key),
                               minimum_generation=kwargs.pop("minimum_generation", 12),
                               revoked_key_ids=kwargs.pop("revoked_key_ids", frozenset()))

    def test_real_bytes_roundtrip_and_no_production_claim(self):
        result = self.recover()
        self.assertEqual((self.root / "restored.pgcustom").read_bytes(),
                         self.source.read_bytes())
        self.assertFalse(result["immutable_offsite_storage_proved"])
        self.assertFalse(result["stage_041_complete"])

    def test_tamper_wrong_key_revoke_rollback_and_duplicate_manifest_denied(self):
        path = self.envelope / "snapshot.aesgcm"
        original = path.read_bytes()
        for changed in (original[:-1] + bytes([original[-1] ^ 1]),
                        original[:20] + bytes([original[20] ^ 1]) + original[21:]):
            path.write_bytes(changed)
            with self.assertRaises(EncryptedBackupError):
                self.recover()
            self.assertFalse((self.root / "restored.pgcustom").exists())
        path.write_bytes(original)
        for policy in ({"key": b"w" * 32},
                       {"minimum_generation": 13},
                       {"revoked_key_ids": frozenset({self.manifest["key_id"]})}):
            with self.subTest(policy=policy), self.assertRaises(EncryptedBackupError):
                self.recover(**policy)
        manifest_path = self.envelope / "manifest.json"
        raw = manifest_path.read_text()
        manifest_path.write_text(raw[:-2] + ',"generation":12}\n')
        with self.assertRaises(EncryptedBackupError):
            self.recover()

    def test_manifest_authentication_rejects_forged_generation(self):
        path = self.envelope / "manifest.json"
        record = json.loads(path.read_text())
        record["generation"] = 13
        path.write_text(json.dumps(record))
        with self.assertRaises(EncryptedBackupError):
            self.recover()

    def test_failed_durable_write_removes_partial_recovery_output(self):
        destination = self.root / "restored.pgcustom"
        with mock.patch.object(backup.os, "fsync", side_effect=OSError("disk failure")):
            with self.assertRaises(OSError):
                self.recover()
        self.assertFalse(destination.exists())

    def test_failed_envelope_write_removes_partial_ciphertext(self):
        fresh = self.root / "fresh"
        fresh.mkdir()
        with mock.patch.object(backup.os, "fsync", side_effect=OSError("disk failure")):
            with self.assertRaises(OSError):
                seal_snapshot(self.source, fresh, key=self.key, generation=13)
        self.assertFalse((fresh / "snapshot.aesgcm").exists())


if __name__ == "__main__":
    unittest.main()
