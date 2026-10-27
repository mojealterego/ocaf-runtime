import dataclasses
import math
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_quorum_certificate import (
    CertificateError, QuorumCertificateGate, issue_vote, issue_vote_ed25519,
)


class QuorumCertificateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "quorum.sqlite3"
        self.now = [100.0]
        self.keys = {name: (name.encode() * 32)[:32] for name in ("w1", "w2", "w3")}
        self.gate = self.open_gate()
        self.addCleanup(self.gate.close)

    def open_gate(self, **kwargs):
        return QuorumCertificateGate(
            self.path, cluster_id="cluster-a", witness_keys=self.keys, allow_shared_secret_lab=True,
            clock=lambda: self.now[0], clock_domain="test-clock", **kwargs)

    def votes(self, candidate="node-b", epoch=1, lease_id="lease-1",
              witnesses=("w1", "w2"), issued=100.0, expires=130.0):
        return [issue_vote(
            witness_id=name, key=self.keys[name], cluster_id="cluster-a",
            candidate_id=candidate, epoch=epoch, lease_id=lease_id,
            issued_at=issued, expires_at=expires) for name in witnesses]

    def test_strict_majority_certificate_is_admitted_and_idempotent(self):
        receipt = self.gate.admit(self.votes())
        self.assertEqual(receipt.witness_ids, ("w1", "w2"))
        self.assertEqual(receipt.epoch, 1)
        self.assertTrue(receipt.certificate_sha256.startswith("sha256:"))
        self.assertEqual(self.gate.admit(self.votes()), receipt)

    def test_one_vote_duplicate_and_unknown_witness_are_denied(self):
        with self.assertRaises(CertificateError):
            self.gate.admit(self.votes(witnesses=("w1",)))
        vote = self.votes(witnesses=("w1",))[0]
        with self.assertRaises(CertificateError):
            self.gate.admit([vote, vote])
        foreign = issue_vote(
            witness_id="foreign", key=b"f" * 32, cluster_id="cluster-a",
            candidate_id="node-b", epoch=1, lease_id="lease-1",
            issued_at=100, expires_at=130)
        with self.assertRaises(CertificateError):
            self.gate.admit([vote, foreign])

    def test_signature_tamper_and_mixed_lease_are_denied(self):
        votes = self.votes()
        votes[0] = dataclasses.replace(votes[0], candidate_id="node-c")
        with self.assertRaises(CertificateError):
            self.gate.admit(votes)
        votes = self.votes()
        votes[1] = self.votes(witnesses=("w2",), lease_id="another")[0]
        with self.assertRaises(CertificateError):
            self.gate.admit(votes)

    def test_expired_future_nonfinite_and_oversized_leases_are_denied(self):
        for issued, expires in ((90, 100), (101, 120), (100, 401),
                                (math.nan, 120), (100, math.inf)):
            with self.subTest(issued=issued, expires=expires), self.assertRaises(CertificateError):
                self.gate.admit(self.votes(issued=issued, expires=expires))

    def test_higher_epoch_fences_old_and_conflicting_same_epoch(self):
        old = self.votes()
        self.gate.admit(old)
        new = self.votes(candidate="node-c", epoch=2, lease_id="lease-2")
        self.gate.admit(new)
        with self.assertRaises(CertificateError):
            self.gate.admit(old)
        conflicting = self.votes(candidate="node-b", epoch=2, lease_id="conflict")
        with self.assertRaises(CertificateError):
            self.gate.admit(conflicting)

    def test_restart_preserves_epoch_and_certificate_identity(self):
        accepted = self.votes(candidate="node-b", epoch=7, lease_id="lease-7")
        receipt = self.gate.admit(accepted)
        self.gate.close()
        with self.open_gate() as reopened:
            self.assertEqual(reopened.admit(accepted), receipt)
            with self.assertRaises(CertificateError):
                reopened.admit(self.votes(epoch=6, lease_id="lease-6"))

    def test_denial_persists_clock_high_water_and_prevents_time_rollback(self):
        self.now[0] = 130
        with self.assertRaises(CertificateError):
            self.gate.admit(self.votes())
        self.gate.close()
        self.now[0] = 129
        with self.open_gate() as reopened, self.assertRaises(CertificateError):
            reopened.admit(self.votes(epoch=2, lease_id="lease-2", expires=140))

    def test_membership_and_clock_domain_change_fail_closed(self):
        self.gate.admit(self.votes())
        self.gate.close()
        changed = dict(self.keys)
        changed["w4"] = changed.pop("w3")
        with self.assertRaises(CertificateError):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=changed, allow_shared_secret_lab=True,
                clock=lambda: 100, clock_domain="test-clock")
        with self.assertRaises(CertificateError):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=self.keys, allow_shared_secret_lab=True,
                clock=lambda: 100, clock_domain="another-clock")

    def test_same_id_key_rotation_and_threshold_change_fail_closed(self):
        self.gate.admit(self.votes())
        self.gate.close()
        rotated = dict(self.keys)
        rotated["w2"] = b"rotated-witness-key".ljust(32, b"!")
        with self.assertRaises(CertificateError):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=rotated, allow_shared_secret_lab=True,
                clock=lambda: 100, clock_domain="test-clock")
        with self.assertRaises(CertificateError):
            self.open_gate(threshold=3)

    def test_corrupt_durable_state_never_reinitializes(self):
        self.gate.admit(self.votes())
        self.gate.close()
        with sqlite3.connect(self.path) as db:
            db.execute("UPDATE quorum_state SET certificate_sha256=NULL")
        with self.assertRaises(CertificateError):
            self.open_gate()

    def test_configuration_requires_three_witnesses_and_strict_majority(self):
        self.gate.close()
        self.path.unlink()
        for keys, threshold in ((dict(list(self.keys.items())[:2]), None),
                                (self.keys, 1), (self.keys, 4)):
            with self.subTest(size=len(keys), threshold=threshold), self.assertRaises(CertificateError):
                QuorumCertificateGate(
                    self.path, cluster_id="cluster-a", witness_keys=keys, allow_shared_secret_lab=True,
                    threshold=threshold, clock=lambda: 100, clock_domain="test-clock")

    def test_two_witness_names_cannot_share_one_signing_key(self):
        self.gate.close()
        self.path.unlink()
        duplicated = dict(self.keys)
        duplicated["w2"] = duplicated["w1"]
        with self.assertRaisesRegex(CertificateError, "keys must be distinct"):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=duplicated, allow_shared_secret_lab=True,
                clock=lambda: 100, clock_domain="test-clock")

    def test_shared_secret_mode_is_denied_without_lab_opt_in(self):
        with self.assertRaisesRegex(CertificateError, "explicit lab opt-in"):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=self.keys,
                clock=lambda: 100, clock_domain="test-clock")

    def test_preexisting_symlink_or_hardlinked_wal_is_denied(self):
        self.gate.close()
        for suffix in ("-wal", "-shm"):
            sibling = Path(str(self.path) + suffix)
            target = Path(self.tmp.name) / ("target" + suffix)
            target.write_bytes(b"external authority")
            sibling.symlink_to(target)
            with self.assertRaises(CertificateError):
                self.open_gate()
            sibling.unlink()
            os.link(target, sibling)
            with self.assertRaises(CertificateError):
                self.open_gate()
            sibling.unlink()

    def test_symlinked_parent_directory_is_denied(self):
        self.gate.close()
        alias = Path(self.tmp.name) / "alias"
        alias.symlink_to(self.tmp.name, target_is_directory=True)
        with self.assertRaises(CertificateError):
            QuorumCertificateGate(
                alias / "quorum.sqlite3", cluster_id="cluster-a",
                witness_keys=self.keys, allow_shared_secret_lab=True,
                clock=lambda: 100, clock_domain="test-clock")


class PublicKeyQuorumCertificateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "public-quorum.sqlite3"
        self.private = {name: Ed25519PrivateKey.generate()
                        for name in ("w1", "w2", "w3")}
        self.public = {name: key.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw)
            for name, key in self.private.items()}
        self.now = [100.0]
        self.gate = self.open_gate()
        self.addCleanup(self.gate.close)

    def open_gate(self, **overrides):
        options = dict(cluster_id="cluster-a", witness_public_keys=self.public,
                       clock=lambda: self.now[0], clock_domain="test-clock")
        options.update(overrides)
        return QuorumCertificateGate(self.path, **options)

    def votes(self, *, candidate="node-a", epoch=1, lease="lease-1",
              names=("w1", "w2")):
        return [issue_vote_ed25519(
            witness_id=name, private_key=self.private[name],
            cluster_id="cluster-a", candidate_id=candidate, epoch=epoch,
            lease_id=lease, issued_at=100.0, expires_at=130.0)
            for name in names]

    def test_public_key_majority_survives_restart_and_fences_stale_epoch(self):
        old = self.votes()
        receipt = self.gate.admit(old)
        self.assertEqual(receipt.witness_ids, ("w1", "w2"))
        self.gate.close()
        with self.open_gate() as reopened:
            self.assertEqual(reopened.admit(old), receipt)
            reopened.admit(self.votes(candidate="node-b", epoch=2, lease="lease-2"))
            with self.assertRaisesRegex(CertificateError, "stale"):
                reopened.admit(old)

    def test_public_key_verifier_rejects_forgery_and_hmac_vote(self):
        forged = self.votes()
        forged[1] = dataclasses.replace(forged[1], candidate_id="node-b")
        with self.assertRaisesRegex(CertificateError, "invalid witness signature"):
            self.gate.admit(forged)
        foreign_key = Ed25519PrivateKey.generate()
        forged[1] = issue_vote_ed25519(
            witness_id="w2", private_key=foreign_key, cluster_id="cluster-a",
            candidate_id="node-a", epoch=1, lease_id="lease-1",
            issued_at=100.0, expires_at=130.0)
        with self.assertRaisesRegex(CertificateError, "invalid witness signature"):
            self.gate.admit(forged)
        hmac_vote = issue_vote(
            witness_id="w2", key=b"x" * 32, cluster_id="cluster-a",
            candidate_id="node-a", epoch=1, lease_id="lease-1",
            issued_at=100.0, expires_at=130.0)
        with self.assertRaisesRegex(CertificateError, "malformed signature"):
            self.gate.admit([self.votes(names=("w1",))[0], hmac_vote])

    def test_public_key_scheme_and_membership_cannot_change_on_restart(self):
        self.gate.admit(self.votes())
        self.gate.close()
        rotated = dict(self.public)
        rotated["w2"] = Ed25519PrivateKey.generate().public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw)
        with self.assertRaises(CertificateError):
            self.open_gate(witness_public_keys=rotated)
        with self.assertRaises(CertificateError):
            self.open_gate(witness_keys={name: b"x" * 31 + bytes([n])
                                         for n, name in enumerate(self.public)},
                           witness_public_keys=None)
        with self.assertRaises(CertificateError):
            self.open_gate(threshold=3)

    def test_public_key_configuration_rejects_duplicate_and_mixed_keys(self):
        self.gate.close()
        duplicate = dict(self.public)
        duplicate["w2"] = duplicate["w1"]
        with self.assertRaises(CertificateError):
            self.open_gate(witness_public_keys=duplicate)
        with self.assertRaises(CertificateError):
            self.open_gate(witness_keys={"w1": b"x" * 32})
        with self.assertRaises(CertificateError):
            self.open_gate(allow_shared_secret_lab=True)


if __name__ == "__main__":
    unittest.main()
