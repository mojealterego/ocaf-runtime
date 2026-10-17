import math
import multiprocessing
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_quorum import QuorumError, QuorumGate, WitnessLease


def _competing_claim(path, holder, ready, start, outcome):
    """Independent interpreter/connection: a shared lock is not a lease."""
    with QuorumGate(path) as gate:
        ready.put(holder)
        start.wait(10)
        try:
            lease = gate.grant(holder, ttl_seconds=30)
            outcome.put((holder, "granted", lease.epoch))
        except QuorumError:
            outcome.put((holder, "denied", None))


class QuorumGateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "witness.sqlite3"
        self.now = [100.0]
        self.gate = self.open_gate()
        self.addCleanup(self.gate.close)

    def open_gate(self, **kwargs):
        return QuorumGate(self.path, clock=lambda: self.now[0],
                          clock_domain="test-monotonic-clock", **kwargs)

    def test_holder_can_write_only_after_promotion(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        with self.assertRaises(QuorumError):
            self.gate.authorize_effect("node-b", lease.epoch)
        self.assertEqual(self.gate.authorize_promotion("node-b", lease.epoch), lease)
        self.gate.authorize_effect("node-b", lease.epoch)
        self.gate.authorize_promotion("node-b", lease.epoch)
        self.gate.authorize_effect("node-b", lease.epoch)

    def test_live_lease_cannot_be_replaced_even_by_same_holder(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        for holder in ("node-c", "node-b"):
            with self.subTest(holder=holder), self.assertRaises(QuorumError):
                self.gate.grant(holder, ttl_seconds=30)
        self.gate.authorize_promotion("node-b", lease.epoch)
        self.gate.authorize_effect("node-b", lease.epoch)

    def test_foreign_holder_and_epoch_never_authorize_or_modify_live_lease(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        self.gate.authorize_promotion("node-b", lease.epoch)
        for holder, epoch in (("node-c", lease.epoch), ("node-b", lease.epoch + 1)):
            for operation in (self.gate.authorize_promotion,
                              self.gate.authorize_effect, self.gate.release):
                with self.subTest(operation=operation.__name__, holder=holder), self.assertRaises(QuorumError):
                    operation(holder, epoch)
            with self.assertRaises(QuorumError):
                self.gate.renew(holder, epoch, ttl_seconds=30)
        self.gate.authorize_effect("node-b", lease.epoch)

    def test_expiry_denies_all_operations_and_next_epoch_fences_old_holder(self):
        old = self.gate.grant("node-b", ttl_seconds=1)
        self.gate.authorize_promotion("node-b", old.epoch)
        self.now[0] = old.expires_at
        for operation in (self.gate.authorize_promotion,
                          self.gate.authorize_effect, self.gate.release):
            with self.subTest(operation=operation.__name__), self.assertRaises(QuorumError):
                operation("node-b", old.epoch)
        with self.assertRaises(QuorumError):
            self.gate.renew("node-b", old.epoch, ttl_seconds=30)
        new = self.gate.grant("node-c", ttl_seconds=30)
        self.assertEqual(new.epoch, old.epoch + 1)
        self.gate.authorize_promotion("node-c", new.epoch)
        self.gate.authorize_effect("node-c", new.epoch)
        with self.assertRaises(QuorumError):
            self.gate.authorize_effect("node-b", old.epoch)

    def test_renew_preserves_epoch_promotion_and_extends_without_shortening(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        self.gate.authorize_promotion("node-b", lease.epoch)
        self.now[0] += 10
        renewed = self.gate.renew("node-b", lease.epoch, ttl_seconds=30)
        self.assertEqual(renewed.epoch, lease.epoch)
        self.assertEqual(renewed.expires_at, 140)
        shorter = self.gate.renew("node-b", lease.epoch, ttl_seconds=1)
        self.assertEqual(shorter.expires_at, renewed.expires_at)
        self.now[0] = 139
        self.gate.authorize_effect("node-b", lease.epoch)

    def test_release_immediately_revokes_effects_and_retains_epoch(self):
        old = self.gate.grant("node-b", ttl_seconds=30)
        self.gate.authorize_promotion("node-b", old.epoch)
        self.gate.release("node-b", old.epoch)
        with self.assertRaises(QuorumError):
            self.gate.authorize_effect("node-b", old.epoch)
        new = self.gate.grant("node-c", ttl_seconds=30)
        self.assertEqual(new.epoch, old.epoch + 1)
        with self.assertRaises(QuorumError):
            self.gate.release("node-b", old.epoch)

    def test_restart_preserves_lease_promotion_and_monotonic_epoch(self):
        old = self.gate.grant("node-b", ttl_seconds=30)
        self.gate.authorize_promotion("node-b", old.epoch)
        self.gate.close()
        with self.open_gate() as reopened:
            reopened.authorize_effect("node-b", old.epoch)
            with self.assertRaises(QuorumError):
                reopened.grant("node-c", ttl_seconds=30)
            reopened.release("node-b", old.epoch)
        with self.open_gate() as reopened:
            new = reopened.grant("node-c", ttl_seconds=30)
            self.assertEqual(new.epoch, old.epoch + 1)
            with self.assertRaises(QuorumError):
                reopened.authorize_promotion("node-b", old.epoch)

    def test_clock_rollback_fails_closed_across_restart(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        self.now[0] = 110
        self.gate.authorize_promotion("node-b", lease.epoch)
        self.gate.close()
        self.now[0] = 109
        with self.open_gate() as reopened:
            with self.assertRaises(QuorumError):
                reopened.authorize_effect("node-b", lease.epoch)
        self.now[0] = 110
        with self.open_gate() as reopened:
            reopened.authorize_effect("node-b", lease.epoch)

    def test_denied_expired_effect_persists_clock_and_prevents_resurrection(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        self.gate.authorize_promotion("node-b", lease.epoch)
        self.now[0] = lease.expires_at
        with self.assertRaises(QuorumError):
            self.gate.authorize_effect("node-b", lease.epoch)
        self.gate.close()
        self.now[0] = lease.expires_at - 1
        with self.open_gate() as reopened:
            with self.assertRaises(QuorumError):
                reopened.authorize_effect("node-b", lease.epoch)
            with self.assertRaises(QuorumError):
                reopened.renew("node-b", lease.epoch, ttl_seconds=30)

    def test_different_clock_domain_never_reuses_lease_after_reboot(self):
        self.gate.grant("node-b", ttl_seconds=30)
        with self.assertRaises(QuorumError):
            QuorumGate(self.path, clock=lambda: 1000, clock_domain="another-boot")

    def test_invalid_clock_is_rejected_without_grant(self):
        for now in (float("nan"), float("inf"), -float("inf"), -1, True, "100", 1e30, 10**1000):
            with self.subTest(now=now):
                self.now[0] = now
                with self.assertRaises(QuorumError):
                    self.gate.grant("node-b", ttl_seconds=30)
        self.now[0] = 100
        self.assertEqual(self.gate.grant("node-b", ttl_seconds=30).epoch, 1)

    def test_invalid_ttl_holder_and_epoch_are_rejected(self):
        for ttl in (math.nan, math.inf, -math.inf, 0, -1, 301, True, "30", 1e-300, 10**1000):
            with self.subTest(ttl=ttl), self.assertRaises(QuorumError):
                self.gate.grant("node-b", ttl_seconds=ttl)
        for holder in ("", "node\n", "two nodes", "../node", "ą", "a" * 129, None):
            with self.subTest(holder=holder), self.assertRaises(QuorumError):
                self.gate.grant(holder, ttl_seconds=30)
        lease = self.gate.grant("node-b", ttl_seconds=30)
        for epoch in (True, 0, -1, 1.0, "1", 2**63):
            with self.subTest(epoch=epoch), self.assertRaises(QuorumError):
                self.gate.authorize_promotion("node-b", epoch)
        self.gate.authorize_promotion("node-b", lease.epoch)

    def test_epoch_exhaustion_never_wraps_or_reuses_token(self):
        self.gate.close()
        with sqlite3.connect(self.path) as db:
            db.execute("UPDATE witness_state SET epoch=?", (2**63 - 1,))
        with self.open_gate() as reopened:
            with self.assertRaises(QuorumError):
                reopened.grant("node-b", ttl_seconds=30)

    def test_storage_file_is_private_and_symlink_is_rejected(self):
        self.assertEqual(self.path.stat().st_mode & 0o777, 0o600)
        link = Path(self.tmp.name) / "link.sqlite3"
        link.symlink_to(self.path)
        with self.assertRaises(QuorumError):
            QuorumGate(link)

    def test_absent_lease_never_allows_authority_operations(self):
        for operation in (self.gate.authorize_promotion,
                          self.gate.authorize_effect, self.gate.release):
            with self.subTest(operation=operation.__name__), self.assertRaises(QuorumError):
                operation("node-b", 1)
        with self.assertRaises(QuorumError):
            self.gate.renew("node-b", 1, ttl_seconds=30)

    def test_missing_durable_row_is_not_silently_reinitialized(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        self.gate.close()
        with sqlite3.connect(self.path) as db:
            db.execute("DELETE FROM witness_state")
        with self.assertRaises(QuorumError):
            self.open_gate()
        with sqlite3.connect(self.path) as db:
            self.assertEqual(db.execute("SELECT COUNT(*) FROM witness_state").fetchone()[0], 0)
        self.assertEqual(lease.epoch, 1)

    def test_injected_clock_requires_explicit_bounded_domain(self):
        for domain in (None, "", "x" * 129, "boot\n"):
            with self.subTest(domain=domain), self.assertRaises(QuorumError):
                QuorumGate(self.path, clock=lambda: 100, clock_domain=domain)

    def test_expiry_predicate_rejects_nonfinite_inputs_and_is_not_authorization(self):
        lease = self.gate.grant("node-b", ttl_seconds=30)
        self.assertTrue(lease.valid_for("node-b", 100))
        self.assertFalse(lease.valid_for("node-b", lease.expires_at))
        self.assertFalse(lease.valid_for("node-c", 100))
        for now in (math.nan, math.inf, -1, True, 10**1000):
            with self.subTest(now=now):
                self.assertFalse(lease.valid_for("node-b", now))
        self.assertFalse(WitnessLease("node-b", 1, math.inf).valid_for("node-b", 100))
        with self.assertRaises(QuorumError):
            self.gate.authorize_effect("node-b", lease.epoch)

    def test_closed_gate_fails_instead_of_silently_reopening(self):
        self.gate.close()
        with self.assertRaises(QuorumError):
            self.gate.grant("node-b", ttl_seconds=30)


class IndependentProcessTests(unittest.TestCase):
    def test_two_independent_processes_cannot_hold_overlapping_leases(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "witness.sqlite3"
            with QuorumGate(path):
                pass
            ctx = multiprocessing.get_context("spawn")
            ready, outcome, start = ctx.Queue(), ctx.Queue(), ctx.Event()
            workers = [ctx.Process(target=_competing_claim,
                                   args=(path, holder, ready, start, outcome))
                       for holder in ("node-a", "node-b")]
            try:
                for worker in workers:
                    worker.start()
                self.assertEqual({ready.get(timeout=10), ready.get(timeout=10)}, {"node-a", "node-b"})
                start.set()
                answers = [outcome.get(timeout=10), outcome.get(timeout=10)]
                self.assertEqual(sorted(answer[1] for answer in answers), ["denied", "granted"])
                winner = next(answer for answer in answers if answer[1] == "granted")
                with QuorumGate(path) as reopened:
                    reopened.authorize_promotion(winner[0], winner[2])
                    reopened.authorize_effect(winner[0], winner[2])
                for worker in workers:
                    worker.join(timeout=10)
                    self.assertEqual(worker.exitcode, 0)
            finally:
                for worker in workers:
                    if worker.is_alive():
                        worker.terminate()
                    worker.join(timeout=10)
                ready.close()
                outcome.close()


if __name__ == "__main__":
    unittest.main()
