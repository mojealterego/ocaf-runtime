"""Pre-upload gate must reject plaintext even with Python optimization enabled."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage041_encrypted_artifact_gate import ArtifactGateError, verify_encrypted_only
from stage041_encrypted_backup import seal_snapshot


class GateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "bundle"
        envelope = self.root / "envelope"
        envelope.mkdir(parents=True)
        snapshot = Path(self.temp.name) / "real.pgcustom"
        snapshot.write_bytes(b"PGDMP real plaintext")
        self.key = b"k" * 32
        snapshot_hash = hashlib.sha256(snapshot.read_bytes()).hexdigest()
        source = {
            "schema_version": 1,
            "status": "POSTGRES_REAL_BACKUP_RESTORE_IDENTICAL_RECORDS_PASSED",
            "original_rows": 25, "restored_rows": 25,
            "original_checksum": "a" * 64, "restored_checksum": "a" * 64,
            "backup_file": "snapshot.pgcustom", "backup_sha256": snapshot_hash,
            "backup_size_bytes": snapshot.stat().st_size,
            "tampered_backup_rejected": True, "postgres_server_real": True,
            "backup_was_custom_pg_dump": True, "restore_was_pg_restore": True,
            "same_kernel_source_and_restore": True,
            "cross_host_restore_tested": False, "stage_041_complete": False,
            "production_approved": False,
        }
        (self.root / "source.json").write_text(json.dumps(source) + "\n")
        (self.root / "manifest.json").write_text(json.dumps({
            "schema_version": 1,
            "source_boot_id": "11111111-1111-4111-8111-111111111111",
            "source_report_sha256": hashlib.sha256((self.root / "source.json").read_bytes()).hexdigest(),
            "snapshot_sha256": snapshot_hash,
            "snapshot_size_bytes": snapshot.stat().st_size,
            "run_nonce": "0123456789abcdef",
            "production_approved": False,
        }) + "\n")
        # The envelope is bound to the transfer manifest, which pins the
        # source report and the PostgreSQL snapshot.
        seal_snapshot(snapshot, envelope, key=self.key, generation=1,
                      transfer_manifest_sha256=hashlib.sha256(
                          (self.root / "manifest.json").read_bytes()).hexdigest())

    def test_complete_encrypted_only_bundle_passes(self):
        verify_encrypted_only(self.root, key=self.key)

    def test_plaintext_file_is_rejected_with_python_optimize(self):
        (self.root / "snapshot.pgcustom").write_bytes(b"PGDMP plaintext")
        script = Path(__file__).resolve().parents[1] / "stage041_encrypted_artifact_gate.py"
        result = subprocess.run([sys.executable, "-O", str(script), "--bundle", str(self.root)],
                                capture_output=True, text=True, check=False,
                                env={**os.environ, "STAGE041_KEY_HEX": self.key.hex()})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("plaintext", result.stderr)

    def test_ciphertext_mutation_and_symlink_are_rejected(self):
        target = self.root / "envelope/snapshot.aesgcm"
        target.write_bytes(target.read_bytes() + b"x")
        with self.assertRaises(ArtifactGateError):
            verify_encrypted_only(self.root, key=self.key)
        target.unlink()
        target.symlink_to(self.root / "source.json")
        with self.assertRaises(ArtifactGateError):
            verify_encrypted_only(self.root, key=self.key)

    def test_plaintext_renamed_as_ciphertext_with_forged_sha_is_rejected(self):
        ciphertext = self.root / "envelope/snapshot.aesgcm"
        ciphertext.write_bytes(b"PGDMP real plaintext")
        manifest_path = self.root / "envelope/manifest.json"
        manifest = json.loads(manifest_path.read_text())
        manifest["ciphertext_sha256"] = hashlib.sha256(ciphertext.read_bytes()).hexdigest()
        manifest_path.write_text(json.dumps(manifest))
        with self.assertRaises(ArtifactGateError):
            verify_encrypted_only(self.root, key=self.key)

    def test_source_report_mutation_is_rejected_before_upload(self):
        source = self.root / "source.json"
        source.write_text(source.read_text().replace('"original_rows": 25', '"original_rows": 24'))
        with self.assertRaises(ArtifactGateError):
            verify_encrypted_only(self.root, key=self.key)

    def test_manifest_extra_field_and_false_size_are_rejected(self):
        manifest = self.root / "manifest.json"
        for mutation in ({"claimed_host": "physical"}, {"snapshot_size_bytes": 1}):
            with self.subTest(mutation=mutation):
                original = json.loads(manifest.read_text())
                manifest.write_text(json.dumps({**original, **mutation}))
                with self.assertRaises(ArtifactGateError):
                    verify_encrypted_only(self.root, key=self.key)
                manifest.write_text(json.dumps(original))

    def test_rewritten_source_and_transfer_manifest_cannot_rebind_ciphertext(self):
        source = self.root / "source.json"
        report = json.loads(source.read_text())
        report["original_checksum"] = "b" * 64
        report["restored_checksum"] = "b" * 64
        source.write_text(json.dumps(report))
        manifest = self.root / "manifest.json"
        transfer = json.loads(manifest.read_text())
        transfer["source_report_sha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
        manifest.write_text(json.dumps(transfer))
        with self.assertRaises(ArtifactGateError):
            verify_encrypted_only(self.root, key=self.key)

    def test_preupload_rejects_invalid_authenticated_scope_fields(self):
        path = self.root / "envelope/manifest.json"
        original = json.loads(path.read_text())
        for mutation in ({"schema_version": True}, {"generation": 0},
                         {"generation": True}, {"generation": 2**63},
                         {"immutable_offsite_storage_proved": True}):
            with self.subTest(mutation=mutation):
                path.write_text(json.dumps({**original, **mutation}))
                with self.assertRaises(ArtifactGateError):
                    verify_encrypted_only(self.root, key=self.key)
        path.write_text(json.dumps(original))


if __name__ == "__main__":
    unittest.main()
