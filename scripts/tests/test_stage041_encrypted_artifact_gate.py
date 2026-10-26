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
        sealed = seal_snapshot(snapshot, envelope, key=self.key, generation=1)
        snapshot_hash = sealed["snapshot_sha256"]
        (self.root / "source.json").write_text("{}\n")
        (self.root / "manifest.json").write_text(json.dumps({
            "snapshot_sha256": snapshot_hash}) + "\n")

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


if __name__ == "__main__":
    unittest.main()
