"""Signed guest quarantine frame tests; KVM evidence is workflow gated."""
from __future__ import annotations

import hashlib
from pathlib import Path
import sys
import unittest

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_real_guest_quarantine as q


class GuestQuarantineContract(unittest.TestCase):
    def setUp(self):
        self.frame = (b"OCAF039QC1|tenant-a|" + b"a" * 32 + b"|"
                      b"12345678-1234-1234-1234-123456789abc|42|9001|17|3\n")

    def test_bounded_kernel_identity_and_real_signed_permit(self):
        challenge = q.parse_challenge(self.frame)
        self.assertEqual(challenge["pid"], 42)
        self.assertEqual(challenge["guest_bpf_id"], 17)
        key = Ed25519PrivateKey.generate()
        permit, signature = q.signed_permit(
            challenge, 107, hashlib.sha256(b"two-kernel-evidence").hexdigest(),
            key, 1800000000)
        key.public_key().verify(signature, permit)
        self.assertIn(b"|42|9001|17|107|3|1500|1800000120|", permit)
        with self.assertRaises(Exception):
            key.public_key().verify(signature, permit.replace(b"tenant-a", b"tenant-b"))

    def test_rejects_untrusted_guest_and_host_claims(self):
        for raw in (
            self.frame.replace(b"tenant-a", b"tenant-b"),
            self.frame.replace(b"|17|3\n", b"|17|1\n"),
            self.frame.replace(b"|42|", b"|1|"),
            self.frame + b"junk",
            self.frame[:-1],
            b"x" * 257,
        ):
            with self.subTest(raw=raw[:60]), self.assertRaises(q.QuarantineDenied):
                q.parse_challenge(raw)
        challenge = q.parse_challenge(self.frame)
        with self.assertRaises(q.QuarantineDenied):
            q.signed_permit(challenge, 0, "0" * 64, Ed25519PrivateKey.generate(),
                            1800000000)


if __name__ == "__main__":
    unittest.main()
