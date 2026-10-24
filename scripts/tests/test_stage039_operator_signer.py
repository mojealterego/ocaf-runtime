"""Operator signing mode rejects unpinned and mutable trust inputs pre-admission."""
import base64
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_broker_vm_binding as binding
import stage039_signed_manifest as signed


class OperatorSignerTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix="ocaf039-signer-",dir="/dev/shm")
        self.addCleanup(self.temp.cleanup)
        root=Path(self.temp.name)
        key=Ed25519PrivateKey.generate()
        public=key.public_key().public_bytes_raw()
        self.anchor=root/"trusted.key"
        self.anchor.write_bytes(public)
        self.anchor.chmod(0o400)
        self.envelopes={}
        for tenant in ("a","b"):
            payload={"schema_version":1,"tenant_id":"tenant-"+tenant}
            envelope={"key_id":"operator-2026","public_key":base64.b64encode(public).decode(),
                      "signature":base64.b64encode(key.sign(signed.canonical_payload(payload))).decode(),
                      "payload":payload}
            path=root/("signed-"+tenant+".json")
            path.write_text(json.dumps(envelope))
            path.chmod(0o400)
            self.envelopes[tenant]=path
        self.arguments={"signed_manifest_a":self.envelopes["a"],
                        "signed_manifest_b":self.envelopes["b"],
                        "trusted_public_key":self.anchor,
                        "trusted_key_id":"operator-2026",
                        "signer_mode":"operator",
                        "trusted_public_key_sha256":hashlib.sha256(public).hexdigest()}

    def test_accepts_two_externally_signed_envelopes_with_pinned_anchor(self):
        envelopes,keys=binding.load_signer_inputs(**self.arguments)
        self.assertEqual(set(envelopes),{"a","b"})
        self.assertEqual(keys["operator-2026"],self.anchor.read_bytes())

    def test_denies_missing_mismatched_or_lab_trust_anchor(self):
        for changes in ({"trusted_public_key_sha256":None},
                        {"trusted_public_key_sha256":"0"*64},
                        {"trusted_key_id":"stage039-ci"},
                        {"signed_manifest_b":None},
                        {"signer_mode":"unknown"}):
            with self.subTest(changes=changes),self.assertRaises(binding.BindingError):
                binding.load_signer_inputs(**{**self.arguments,**changes})

    def test_denies_changed_signature_symlink_and_mutable_operator_files(self):
        original=self.envelopes["b"].read_bytes()
        changed=json.loads(original)
        changed["payload"]["tenant_id"]="tenant-a"
        self.envelopes["b"].chmod(0o600)
        self.envelopes["b"].write_text(json.dumps(changed))
        self.envelopes["b"].chmod(0o400)
        with self.assertRaises(binding.BindingError):
            binding.load_signer_inputs(**self.arguments)
        self.envelopes["b"].chmod(0o600)
        self.envelopes["b"].write_bytes(original)
        self.envelopes["b"].chmod(0o400)
        link=self.anchor.parent/"key-link"
        link.symlink_to(self.anchor)
        with self.assertRaises(binding.BindingError):
            binding.load_signer_inputs(**{**self.arguments,"trusted_public_key":link})
        self.anchor.chmod(0o666)
        with self.assertRaises(binding.BindingError):
            binding.load_signer_inputs(**self.arguments)
        self.anchor.chmod(0o400)
        self.anchor.parent.chmod(0o777)
        with self.assertRaises(binding.BindingError):
            binding.load_signer_inputs(**self.arguments)
        self.anchor.parent.chmod(0o700)


if __name__=="__main__":
    unittest.main()
