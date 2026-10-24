"""Stage 0.39 signed input SBOM and observed inode mutation checks."""
from __future__ import annotations

import base64
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_microvm_gate as gate
import stage039_signed_manifest as signed
import stage039_supply_chain as supply


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class SupplyChainTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="ocaf039-supply-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.approved = self.root / "approved"
        self.repository = self.root / "repository"
        self.approved.mkdir(mode=0o700)
        self.repository.mkdir(mode=0o700)
        self.key = Ed25519PrivateKey.generate()
        public = self.key.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        self.write(self.approved / "stage039-ci-public.key", public)
        for name in supply.SOURCE_LOCKS + supply.SOURCE_BUILDS:
            path = self.repository / name
            path.parent.mkdir(parents=True, exist_ok=True)
            self.write(path, (name + "\n").encode())
        for tenant in ("a", "b"):
            assets = {}
            for name in sorted(gate.REQUIRED_ASSETS | gate.WASI_ASSETS):
                data = (tenant + ":" + name).encode()
                path = self.approved / f"{tenant}-{name}"
                self.write(path, data, 0o555 if name in {
                    "firecracker", "wasmtime_runtime", "guest_openat_probe"} else 0o444)
                assets[name] = {"path": str(path), "sha256": digest(data)}
            request = {
                "schema_version": 1, "workload_id": "ocaf-" + tenant,
                "tenant_id": "tenant-" + tenant, "agent_id": "agent-" + tenant,
                "isolation": {"hypervisor": "firecracker", "jailer": True,
                              "network": "none", "guest_egress": False,
                              "rootfs_readonly": True, "mmds_enabled": False,
                              "seccomp": "default", "vcpus": 1, "mem_mib": 128},
                "assets": assets,
                "wasi": {"task_nonce": ("a" if tenant == "a" else "b") * 32,
                         "module_sha256": assets["wasi_module"]["sha256"]},
            }
            self.write(self.approved / f"request-wasi-{tenant}.json",
                       (json.dumps(request, sort_keys=True) + "\n").encode())
            envelope = {
                "key_id": "stage039-ci", "public_key": base64.b64encode(public).decode(),
                "signature": base64.b64encode(
                    self.key.sign(signed.canonical_payload(request))).decode(),
                "payload": request,
            }
            self.write(self.approved / f"signed-wasi-{tenant}.json",
                       (json.dumps(envelope, sort_keys=True) + "\n").encode())

    @staticmethod
    def write(path: Path, raw: bytes, mode: int = 0o444):
        path.write_bytes(raw)
        path.chmod(mode)

    def test_deterministic_signed_input_sbom(self):
        bom, proof = supply.inventory(self.approved, self.repository)
        again, proof_again = supply.inventory(self.approved, self.repository)
        self.assertEqual(bom, again)
        self.assertEqual(proof, proof_again)
        self.assertEqual(proof["component_count"], 2 * 7 + 7)
        self.assertFalse(proof["running_inode_verified"])
        self.assertFalse(proof["independent_signer_verified"])
        self.assertEqual(len(proof["signed_envelope_sha256"]), 2)

    def test_mutated_artifact_or_envelope_is_rejected(self):
        path = self.approved / "a-guest_openat_probe"
        path.chmod(0o644)
        path.write_bytes(b"altered")
        path.chmod(0o444)
        with self.assertRaises(ValueError):
            supply.inventory(self.approved, self.repository)
        path.chmod(0o644)
        path.write_bytes(b"a:guest_openat_probe")
        path.chmod(0o444)
        envelope = self.approved / "signed-wasi-a.json"
        obj = json.loads(envelope.read_text())
        obj["payload"]["agent_id"] = "agent-b"
        envelope.chmod(0o644)
        envelope.write_text(json.dumps(obj, sort_keys=True) + "\n")
        envelope.chmod(0o444)
        with self.assertRaises(ValueError):
            supply.inventory(self.approved, self.repository)

    def test_observed_same_length_inode_change_during_read_is_rejected(self):
        path = self.repository / "Cargo.lock"
        original_read = os.read
        changed = False

        def read_and_mutate(fd, count):
            nonlocal changed
            data = original_read(fd, count)
            if data and not changed:
                changed = True
                path.chmod(0o644)
                path.write_bytes(b"X" * len(data))
                path.chmod(0o444)
            return data

        with patch.object(supply.os, "read", side_effect=read_and_mutate):
            with self.assertRaisesRegex(supply.SupplyChainDenied,
                                        "inode changed during read"):
                supply.stable_digest(path, supply.MAX_SOURCE_BYTES,
                                     root=self.repository)


if __name__ == "__main__":
    unittest.main()
