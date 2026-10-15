"""0.39.3: real AF_UNIX SO_PEERCRED enforcement, framed input and deny tests.

No virtual machine is launched by any test. Both principals are synthetic.
"""
import asyncio
import base64
import hashlib
import json
import os
from pathlib import Path
import socket
import struct
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_broker_ipc as ipc
import stage039_broker_ledger as broker


class BrokerIPCTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.working = self.root / "private"
        self.working.mkdir(mode=0o700)
        self.assets = self.working / "approved"
        self.assets.mkdir(mode=0o700)
        assets = {}
        for name in broker.microvm.REQUIRED_ASSETS:
            p = self.assets / (name + ".bin")
            p.write_bytes(("verified-" + name).encode())
            p.chmod(0o700 if name == "firecracker" else 0o600)
            assets[name] = {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
        self.manifest = json.dumps({
            "schema_version": 1, "workload_id": "workload-x",
            "tenant_id": "tenant-x", "agent_id": "agent-x",
            "isolation": {"hypervisor": "firecracker", "jailer": True,
                          "network": "none", "guest_egress": False,
                          "rootfs_readonly": True, "mmds_enabled": False,
                          "seccomp": "default", "vcpus": 1, "mem_mib": 256},
            "assets": assets
        }).encode()
        self.now = 1_000_000
        req = {"schema_version": 1, "request_id": "a"*32,
               "tenant_id": "tenant-x", "agent_id": "agent-x",
               "workload_id": "workload-x", "leader_epoch": 1,
               "issued_unix_secs": self.now,
               "expires_unix_secs": self.now+60,
               "manifest_sha256": hashlib.sha256(self.manifest).hexdigest()}
        self.req = json.dumps(req).encode()
        self.ledger = broker.AdmissionLedger(self.working / "leases.sqlite")
        self.addAsyncCleanup(self._close)
        self.patch = mock.patch.object(
            broker.microvm, "preflight",
            return_value={"status": "HOST_PREREQUISITES_ONLY"})
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.policy = ipc.PeerPolicy(
            allowed_uids=frozenset({os.getuid()}),
            allowed_pairs=frozenset({("tenant-x", "agent-x")}))
        self.sockpath = self.working / "broker.sock"
        self.service = ipc.BrokerIPC(
            self.sockpath, self.ledger, self.assets, self.policy)
        await self.service.start()
        self.addAsyncCleanup(self.service.close)

    async def _close(self):
        self.ledger.close()

    async def send(self, obj, *, raw=None, length_override=None):
        reader, writer = await asyncio.open_unix_connection(str(self.sockpath))
        data = raw if raw is not None else json.dumps(obj).encode()
        count = len(data) if length_override is None else length_override
        writer.write(struct.pack("!I", count) + data)
        await writer.drain()
        try:
            header = await asyncio.wait_for(reader.readexactly(4), timeout=2)
            size = struct.unpack("!I", header)[0]
            self.assertLess(size, 16_384)
            body = await asyncio.wait_for(reader.readexactly(size), timeout=2)
            return json.loads(body)
        finally:
            writer.close()
            await writer.wait_closed()

    def claim(self):
        return {"version": 1, "op": "reserve",
                "request_b64": base64.b64encode(self.req).decode(),
                "manifest_b64": base64.b64encode(self.manifest).decode()}

    async def test_real_socket_reserve_release_and_replay(self):
        reserved = await self.send(self.claim())
        self.assertEqual(reserved["status"], "RESERVED_NOT_EXECUTED")
        self.assertFalse(reserved["microvm_launched"])
        self.assertFalse(reserved["production_approved"])
        twice = await self.send(self.claim())
        self.assertEqual(twice["status"], "DENIED")
        release = await self.send({
            "version": 1, "op": "release", "workload_id": "workload-x",
            "request_id": "a"*32, "leader_epoch": 1,
        })
        self.assertEqual(release["status"], "RELEASED_NOT_EXECUTED")
        again = await self.send(self.claim())
        self.assertEqual(again["status"], "DENIED")

    async def test_socket_filesystem_is_private(self):
        self.assertEqual(self.sockpath.stat().st_mode & 0o777, 0o600)
        self.assertTrue(self.sockpath.is_socket())
        self.assertEqual(self.working.stat().st_mode & 0o777, 0o700)

    async def test_prohibit_unapproved_tenant_and_agent(self):
        for tenant, agent in (("tenant-y", "agent-x"), ("tenant-x", "agent-y")):
            packet = json.loads(self.manifest)
            packet["tenant_id"], packet["agent_id"] = tenant, agent
            raw = json.dumps(packet).encode()
            req = json.loads(self.req)
            req["tenant_id"], req["agent_id"] = tenant, agent
            req["manifest_sha256"] = hashlib.sha256(raw).hexdigest()
            data = self.claim()
            data["request_b64"] = base64.b64encode(json.dumps(req).encode()).decode()
            data["manifest_b64"] = base64.b64encode(raw).decode()
            self.assertEqual((await self.send(data))["status"], "DENIED")

    async def test_unauthorized_kernel_uid_denied(self):
        self.service.policy = ipc.PeerPolicy(
            allowed_uids=frozenset({os.getuid()+10_000}),
            allowed_pairs=frozenset({("tenant-x", "agent-x")}))
        res = await self.send(self.claim())
        self.assertEqual(res["status"], "DENIED")
        count = self.ledger.conn.execute("SELECT COUNT(*) FROM lease_history").fetchone()[0]
        self.assertEqual(count, 0)

    async def test_duplicate_json_keys_and_unknown_ops_fail(self):
        payloads = [b'{"version":1,"version":1,"op":"health"}',
                    b'{"version":1,"op":"exec"}',
                    b'{"version":1,"op":"reserve","secret":"password"}',
                    b'{"version":1,"op":"health","extra":true}',
                    b'{"version":true,"op":"health"}']
        for p in payloads:
            with self.subTest(p=p):
                self.assertEqual((await self.send(None, raw=p))["status"], "DENIED")

    async def test_reject_oversized_frame_without_reading_body(self):
        reader, writer = await asyncio.open_unix_connection(str(self.sockpath))
        writer.write(struct.pack("!I", ipc.MAX_FRAME_BYTES + 1))
        await writer.drain()
        header = await asyncio.wait_for(reader.readexactly(4), timeout=2)
        size = struct.unpack("!I", header)[0]
        payload = await asyncio.wait_for(reader.readexactly(size), timeout=2)
        self.assertEqual(json.loads(payload)["status"], "DENIED")
        writer.close()
        await writer.wait_closed()

    async def test_no_endpoint_ever_claims_microvm_evidence(self):
        r = await self.send({"version": 1, "op": "health"})
        self.assertEqual(r["status"], "ADMISSION_ONLY")
        for key in ("microvm_launched", "ebpf_attached", "production_approved"):
            self.assertIs(r[key], False)

    async def test_forged_peercred_in_json_is_rejected(self):
        frame = self.claim()
        frame["uid"] = os.getuid()
        self.assertEqual((await self.send(frame))["status"], "DENIED")

    async def test_untrusted_socket_parent_refused(self):
        directory = self.root / "bad"
        directory.mkdir(mode=0o755)
        with self.assertRaises(ipc.IPCSecurityError):
            await ipc.BrokerIPC(directory / "broker.sock", self.ledger,
                                self.assets, self.policy).start()

    async def test_bound_socket_not_overwritten(self):
        with self.assertRaises(ipc.IPCSecurityError):
            await ipc.BrokerIPC(self.sockpath, self.ledger,
                                self.assets, self.policy).start()
        self.assertTrue(self.sockpath.exists())


if __name__ == "__main__":
    unittest.main()
