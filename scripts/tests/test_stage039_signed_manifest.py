import base64
import json
import sys
import unittest
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_signed_manifest as signed


class SignedManifestTests(unittest.TestCase):
    def setUp(self):
        self.key = Ed25519PrivateKey.generate()
        self.payload = {"schema_version": 1, "tenant_id": "tenant-a", "nonce": "n-1"}

    def raw(self, payload=None):
        payload = self.payload if payload is None else payload
        message = signed.canonical_payload(payload)
        public = self.key.public_key().public_bytes_raw()
        signature = self.key.sign(message)
        return json.dumps({"key_id": "stage039-prod", "public_key": base64.b64encode(public).decode(),
                           "signature": base64.b64encode(signature).decode(), "payload": payload}).encode()

    def test_valid_signature_is_verified(self):
        result = signed.verify(self.raw(), {"stage039-prod": self.key.public_key().public_bytes_raw()})
        self.assertTrue(result["signed"])
        self.assertEqual(result["payload"], self.payload)

    def test_payload_tamper_is_rejected(self):
        data = json.loads(self.raw())
        data["payload"]["tenant_id"] = "tenant-b"
        with self.assertRaises(signed.SignedManifestError):
            signed.verify(json.dumps(data).encode(), {"stage039-prod": self.key.public_key().public_bytes_raw()})

    def test_signature_and_key_are_required(self):
        data = json.loads(self.raw())
        data["signature"] = base64.b64encode(b"bad").decode()
        with self.assertRaises(signed.SignedManifestError):
            signed.verify(json.dumps(data).encode(), {"stage039-prod": self.key.public_key().public_bytes_raw()})

    def test_untrusted_signer_and_duplicate_json_are_rejected(self):
        with self.assertRaisesRegex(signed.SignedManifestError, "not trusted"):
            signed.verify(self.raw(), {"stage039-prod": Ed25519PrivateKey.generate().public_key().public_bytes_raw()})
        with self.assertRaisesRegex(signed.SignedManifestError, "duplicate"):
            signed.verify(self.raw()[:-1] + b',"payload":{}}',
                          {"stage039-prod": self.key.public_key().public_bytes_raw()})


if __name__ == "__main__":
    unittest.main()
