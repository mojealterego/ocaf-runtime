"""0.39.2 durable broker admission regression tests — never launch guest VMs."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import stage039_broker_ledger as broker  # noqa: E402


class BrokerAdmissionTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.base = Path(temp.name)
        self.assets = self.base / "approved"
        self.assets.mkdir(mode=0o700)
        self.state = self.base / "state"
        self.state.mkdir(mode=0o700)
        assets = {}
        for name in broker.microvm.REQUIRED_ASSETS:
            p = self.assets / (name + ".bin")
            p.write_bytes(name.encode() + b":synthetic-test-content")
            p.chmod(0o700 if name == "firecracker" else 0o600)
            assets[name] = {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
        self.manifest = json.dumps({
            "schema_version": 1, "workload_id": "alpha",
            "tenant_id": "tenant-a", "agent_id": "agent-a",
            "isolation": {"hypervisor": "firecracker", "jailer": True,
                          "network": "none", "guest_egress": False,
                          "rootfs_readonly": True, "mmds_enabled": False,
                          "seccomp": "default", "vcpus": 1, "mem_mib": 256},
            "assets": assets
        }).encode()
        self.now = 1_000_000
        self.request = {
            "schema_version": 1, "request_id": "1" * 32,
            "tenant_id": "tenant-a", "agent_id": "agent-a", "workload_id": "alpha",
            "leader_epoch": 1, "issued_unix_secs": self.now,
            "expires_unix_secs": self.now + 60,
            "manifest_sha256": hashlib.sha256(self.manifest).hexdigest(),
        }
        self.host = mock.patch.object(
            broker.microvm, "preflight",
            return_value={"status": "HOST_PREREQUISITES_ONLY"})
        self.host.start()
        self.addCleanup(self.host.stop)
        self.ledger = broker.AdmissionLedger(self.state / "ledger.sqlite")
        self.addCleanup(self.ledger.close)

    def raw(self, **changes):
        return json.dumps({**self.request, **changes}).encode()

    def claim(self, **changes):
        return self.ledger.reserve(self.raw(**changes), self.manifest, self.assets,
                                   now=self.now)

    def test_admission_is_not_vm_start_or_ebpf_attach(self):
        with mock.patch("subprocess.run") as run, mock.patch("subprocess.Popen") as spawn:
            result = self.claim()
        run.assert_not_called()
        spawn.assert_not_called()
        self.assertEqual(result["status"], "RESERVED_NOT_EXECUTED")
        for name in ("microvm_launched", "ebpf_attached", "production_approved",
                     "stage_039_complete"):
            self.assertIs(result[name], False)
        self.assertEqual((self.state / "ledger.sqlite").stat().st_mode & 0o777, 0o600)

    def test_replay_even_after_release_and_epoch_increases(self):
        self.claim()
        self.assertTrue(self.ledger.release(
            workload_id="alpha", request_id=self.request["request_id"], epoch=1))
        with self.assertRaises(broker.AdmissionError):
            self.claim()
        result = self.claim(request_id="2"*32, leader_epoch=2)
        self.assertEqual(result["leader_epoch"], 2)

    def test_stale_epoch_or_active_workload_denied(self):
        self.claim()
        with self.assertRaises(broker.AdmissionError):
            self.claim(request_id="2"*32, leader_epoch=2)
        with self.assertRaises(broker.AdmissionError):
            self.claim(request_id="3"*32, leader_epoch=1)

    def test_expired_lease_requires_higher_epoch(self):
        self.claim()
        self.now += 62
        with self.assertRaises(broker.AdmissionError):
            self.claim(request_id="4"*32, leader_epoch=1,
                       issued_unix_secs=self.now, expires_unix_secs=self.now+60)
        result = self.claim(request_id="5"*32, leader_epoch=2,
                            issued_unix_secs=self.now, expires_unix_secs=self.now+60)
        self.assertEqual(result["status"], "RESERVED_NOT_EXECUTED")

    def test_release_wrong_credentials_and_double_release(self):
        self.claim()
        self.assertFalse(self.ledger.release(workload_id="alpha", request_id="2"*32, epoch=1))
        self.assertFalse(self.ledger.release(workload_id="alpha", request_id="1"*32, epoch=2))
        self.assertTrue(self.ledger.release(workload_id="alpha", request_id="1"*32, epoch=1))
        self.assertFalse(self.ledger.release(workload_id="alpha", request_id="1"*32, epoch=1))

    def test_untrusted_raw_request_rejected(self):
        for mod in [
            {"schema_version": True}, {"request_id": "bad"}, {"leader_epoch": True},
            {"leader_epoch": 0}, {"leader_epoch": (1 << 63)}, {"tenant_id": "tenant-b"},
            {"agent_id": "attacker"}, {"workload_id": "other"},
            {"manifest_sha256": "0"*64},
            {"issued_unix_secs": self.now-100}, {"issued_unix_secs": self.now+100},
            {"expires_unix_secs": self.now}, {"expires_unix_secs": self.now+301},
            {"expires_unix_secs": True}, {"leader_epoch": 1.0},
        ]:
            with self.subTest(mod=mod), self.assertRaises(broker.AdmissionError):
                self.claim(**mod)
        for raw in (b"{}", b'{"schema_version":1,"schema_version":1}',
                    self.raw() + b"garbage", b"x"*broker.MAX_REQUEST_BYTES):
            with self.subTest(raw=raw[:20]), self.assertRaises(broker.AdmissionError):
                self.ledger.reserve(raw, self.manifest, self.assets, now=self.now)

    def test_host_denial_stops_admission_without_state_write(self):
        with mock.patch.object(broker.microvm, "preflight",
                               return_value={"status": "NO_GO"}):
            with self.assertRaises(broker.AdmissionError):
                self.claim()
        self.assertEqual(
            self.ledger.conn.execute("SELECT COUNT(*) FROM lease_history").fetchone()[0], 0)

    def test_tampered_assets_denied(self):
        (self.assets / "guest_kernel.bin").write_bytes(b"tampered")
        with self.assertRaises(ValueError):
            self.claim()

    def test_db_reopen_replay_survives_restart(self):
        self.claim()
        second = broker.AdmissionLedger(self.state / "ledger.sqlite")
        try:
            with self.assertRaises(broker.AdmissionError):
                second.reserve(self.raw(), self.manifest, self.assets, now=self.now)
        finally:
            second.close()

    def test_simultaneous_sqlite_clients_cannot_claim_twice(self):
        second = broker.AdmissionLedger(self.state / "ledger.sqlite")
        try:
            self.claim()
            with self.assertRaises(broker.AdmissionError):
                second.reserve(self.raw(request_id="3"*32, leader_epoch=2),
                               self.manifest, self.assets, now=self.now)
        finally:
            second.close()

    def test_history_capacity_is_fail_closed(self):
        self.claim()
        with mock.patch.object(broker, "MAX_HISTORY_ROWS", 1):
            with self.assertRaises(broker.AdmissionError):
                self.claim(request_id="2"*32, leader_epoch=2)
        self.assertEqual(
            self.ledger.conn.execute("SELECT COUNT(*) FROM lease_history").fetchone()[0], 1)

    def test_active_capacity_limits_new_workload(self):
        self.claim()
        data = json.loads(self.manifest)
        data["workload_id"] = "beta"
        other_manifest = json.dumps(data).encode()
        request = self.raw(request_id="b"*32, workload_id="beta",
                           manifest_sha256=hashlib.sha256(other_manifest).hexdigest())
        with mock.patch.object(broker, "MAX_ACTIVE_LEASES", 1):
            with self.assertRaises(broker.AdmissionError):
                self.ledger.reserve(request, other_manifest, self.assets, now=self.now)

    def test_ledger_requires_private_directory(self):
        self.state.chmod(0o755)
        with self.assertRaises(broker.AdmissionError):
            broker.AdmissionLedger(self.state / "second.sqlite")
        self.state.chmod(0o700)

    def test_ledger_rejects_symlink(self):
        (self.state / "other.db").write_text("")
        (self.state / "alias.db").symlink_to(self.state / "other.db")
        with self.assertRaises(broker.AdmissionError):
            broker.AdmissionLedger(self.state / "alias.db")

    def test_lease_owner_never_changes(self):
        self.claim()
        self.now += 62
        m = json.loads(self.manifest)
        m["tenant_id"] = "tenant-b"
        other = json.dumps(m).encode()
        with self.assertRaises(broker.AdmissionError):
            self.ledger.reserve(self.raw(request_id="f"*32, tenant_id="tenant-b",
                          leader_epoch=2, issued_unix_secs=self.now,
                          expires_unix_secs=self.now+60,
                          manifest_sha256=hashlib.sha256(other).hexdigest()),
                other, self.assets, now=self.now)

    def test_report_has_no_private_paths_or_secrets(self):
        with mock.patch.dict(os.environ, {"OCAF_SECRET": "DO_NOT_EMIT"}):
            report = self.claim()
        dumped = json.dumps(report)
        self.assertNotIn(str(self.assets), dumped)
        self.assertNotIn("DO_NOT_EMIT", dumped)


if __name__ == "__main__":
    unittest.main()
