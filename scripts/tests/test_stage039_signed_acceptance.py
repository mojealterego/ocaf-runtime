"""Owner-only stage 0.39 dossier signatures cannot self-approve deployment."""
import hashlib
import subprocess
import tempfile
import unittest
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_signed_acceptance as sig


class SignedAcceptance(unittest.TestCase):
    def setUp(self):
        if subprocess.run(["which", "openssl"], capture_output=True).returncode:
            self.skipTest("openssl missing")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.private = self.root / "operator.key"
        self.public = self.root / "operator.pub"
        subprocess.run(["openssl", "genpkey", "-algorithm", "Ed25519",
                        "-out", str(self.private)], check=True, capture_output=True)
        subprocess.run(["openssl", "pkey", "-in", str(self.private),
                        "-pubout", "-out", str(self.public)], check=True,
                       capture_output=True)
        self.pin = hashlib.sha256(self.public.read_bytes()).hexdigest()
        self.manifest = {
            "schema_version": 2, "stage": "0.39", "branch": "main",
            "git_commit": "a" * 40,
            "audit_waiver": "OWNER_WAIVES_INDEPENDENT_AUDIT",
            "operator_decision": "ACCEPT", "evidence": {},
        }
        for name in sig.REQUIRED:
            file = self.root / (name + ".json")
            file.write_text('{"evidence":"synthetic fixture, no runtime authority"}')
            self.manifest["evidence"][name] = {
                "artifact_file": file.name,
                "sha256": hashlib.sha256(file.read_bytes()).hexdigest(),
            }
        payload = sig.canonical_manifest(self.manifest)
        (self.root / "payload.bin").write_bytes(payload)
        self.signature = self.root / "operator.sig"
        subprocess.run(["openssl", "pkeyutl", "-sign", "-inkey", str(self.private),
                        "-rawin", "-in", str(self.root / "payload.bin"),
                        "-out", str(self.signature)], check=True, capture_output=True)

    def verify(self):
        return sig.verify_signed_dossier(
            self.manifest, self.root, operator_pub=self.public,
            operator_pin=self.pin, operator_signature=self.signature)

    def test_owner_signature_and_waiver_verify_integrity_only(self):
        report = self.verify()
        self.assertTrue(report["operator_signature_verified"])
        self.assertTrue(report["independent_audit_waived_by_owner"])
        self.assertFalse(report["production_approved"])
        self.assertFalse(report["stage_039_complete"])

    def test_changed_payload_or_wrong_operator_key_is_denied(self):
        self.manifest["git_commit"] = "b" * 40
        with self.assertRaises(sig.SignedAcceptanceError):
            self.verify()
        self.manifest["git_commit"] = "a" * 40
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.verify_signed_dossier(
                self.manifest, self.root, operator_pub=self.public,
                operator_pin="0" * 64, operator_signature=self.signature)

    def test_missing_artifact_and_old_auditor_schema_are_denied(self):
        (self.root / (sig.REQUIRED[0] + ".json")).unlink()
        with self.assertRaises(sig.SignedAcceptanceError):
            self.verify()
        legacy = dict(self.manifest)
        legacy["schema_version"] = 1
        legacy["audit_result"] = "PASS"
        legacy.pop("audit_waiver")
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.canonical_manifest(legacy)

    def test_duplicate_json_and_unknown_approval_fields_are_denied(self):
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.parse_manifest(b'{"schema_version":2,"schema_version":2}')
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.canonical_manifest({**self.manifest, "production_approved": True})
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.canonical_manifest({**self.manifest, "audit_waiver": ""})


if __name__ == "__main__":
    unittest.main()
