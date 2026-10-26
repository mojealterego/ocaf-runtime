from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_pg_witness_effect_lab import admitted_effect, LabError
from stage040_quorum_certificate import CertificateError, QuorumCertificateGate, issue_vote


class PostgreSQLEffectAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.keys = {name: name.encode().ljust(32, b"!")
                     for name in ("w1", "w2", "w3")}
        self.gate = QuorumCertificateGate(
            Path(self.tmp.name)/"quorum.sqlite", cluster_id="pg-lab",
            witness_keys=self.keys, clock=lambda: 100, clock_domain="test-clock")
        self.addCleanup(self.gate.close)
        self.executed = []

    def votes(self, candidate="node-a", epoch=1, witnesses=("w1", "w2")):
        return [issue_vote(witness_id=name, key=self.keys[name],
                           cluster_id="pg-lab", candidate_id=candidate,
                           epoch=epoch, lease_id=f"lease-{epoch}",
                           issued_at=100, expires_at=120) for name in witnesses]

    def execute(self, epoch, candidate, nonce):
        self.executed.append((epoch, candidate, nonce))

    def test_majority_is_required_before_effect(self):
        with self.assertRaises(CertificateError):
            admitted_effect(self.gate, self.votes(witnesses=("w1",)),
                            candidate="node-a", nonce="a"*32,
                            execute=self.execute)
        self.assertEqual(self.executed, [])
        self.assertEqual(admitted_effect(self.gate, self.votes(), candidate="node-a",
                                        nonce="b"*32, execute=self.execute), 1)
        self.assertEqual(self.executed, [(1, "node-a", "b"*32)])

    def test_new_epoch_fences_old_writer_before_effect(self):
        admitted_effect(self.gate, self.votes(), candidate="node-a",
                        nonce="a"*32, execute=self.execute)
        admitted_effect(self.gate, self.votes("node-b", 2, ("w2", "w3")),
                        candidate="node-b", nonce="b"*32, execute=self.execute)
        with self.assertRaises(CertificateError):
            admitted_effect(self.gate, self.votes(), candidate="node-a",
                            nonce="c"*32, execute=self.execute)
        self.assertEqual(len(self.executed), 2)

    def test_wrong_candidate_and_injection_never_reach_effect(self):
        with self.assertRaises(LabError):
            admitted_effect(self.gate, self.votes(), candidate="node-b",
                            nonce="a"*32, execute=self.execute)
        with self.assertRaises(LabError):
            admitted_effect(self.gate, self.votes(), candidate="node-a';DROP",
                            nonce="a"*32, execute=self.execute)
        self.assertEqual(self.executed, [])
        # A mismatched call must not advance the durable epoch before denial.
        self.assertEqual(admitted_effect(
            self.gate, self.votes(candidate="node-b"), candidate="node-b",
            nonce="b"*32, execute=self.execute), 1)


if __name__ == "__main__":
    unittest.main()
