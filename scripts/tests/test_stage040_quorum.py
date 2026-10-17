import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_quorum import QuorumError, QuorumGate


class QuorumGateTests(unittest.TestCase):
    def setUp(self):
        self.now = [100.0]
        self.gate = QuorumGate(clock=lambda: self.now[0])

    def test_witness_epoch_fences_old_primary_before_promotion(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        self.gate.authorize_effect("node-b", lease.epoch)
        self.gate.authorize_promotion("node-b", lease.epoch)
        with self.assertRaises(QuorumError):
            self.gate.authorize_effect("node-b", lease.epoch)

    def test_expiry_denies_promotion(self):
        lease = self.gate.grant("node-b", ttl_seconds=1)
        self.now[0] += 1
        with self.assertRaises(QuorumError):
            self.gate.authorize_promotion("node-b", lease.epoch)

    def test_stale_holder_and_epoch_are_denied(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        with self.assertRaises(QuorumError):
            self.gate.authorize_promotion("node-c", lease.epoch)
        with self.assertRaises(QuorumError):
            self.gate.authorize_promotion("node-b", lease.epoch - 1)


if __name__ == "__main__":
    unittest.main()
