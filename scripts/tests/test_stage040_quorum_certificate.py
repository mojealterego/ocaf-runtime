import dataclasses
import math
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_quorum_certificate import (
    CertificateError, QuorumCertificateGate, issue_vote,
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
            self.path, cluster_id="cluster-a", witness_keys=self.keys,
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
                self.path, cluster_id="cluster-a", witness_keys=changed,
                clock=lambda: 100, clock_domain="test-clock")
        with self.assertRaises(CertificateError):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=self.keys,
                clock=lambda: 100, clock_domain="another-clock")

    def test_same_id_key_rotation_and_threshold_change_fail_closed(self):
        self.gate.admit(self.votes())
        self.gate.close()
        rotated = dict(self.keys)
        rotated["w2"] = b"rotated-witness-key".ljust(32, b"!")
        with self.assertRaises(CertificateError):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=rotated,
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
                    self.path, cluster_id="cluster-a", witness_keys=keys,
                    threshold=threshold, clock=lambda: 100, clock_domain="test-clock")

    def test_two_witness_names_cannot_share_one_signing_key(self):
        self.gate.close()
        self.path.unlink()
        duplicated = dict(self.keys)
        duplicated["w2"] = duplicated["w1"]
        with self.assertRaisesRegex(CertificateError, "keys must be distinct"):
            QuorumCertificateGate(
                self.path, cluster_id="cluster-a", witness_keys=duplicated,
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
                witness_keys=self.keys, clock=lambda: 100, clock_domain="test-clock")


if __name__ == "__main__":
    unittest.main()
