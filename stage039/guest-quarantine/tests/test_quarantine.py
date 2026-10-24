"""Operator permit and reversible PID quarantine, on the current Linux host.

These tests never claim Firecracker/KVM acceptance or anomaly classification.
"""
from __future__ import annotations

import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

SOURCE = Path(__file__).resolve().parents[1] / "quarantine.c"


class QuarantineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(dir="/dev/shm")
        cls.base = Path(cls.temp.name)
        cls.key = Ed25519PrivateKey.generate()
        cls.pubkey = cls.base / "trusted.pub"
        cls.pubkey.write_bytes(cls.key.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw))
        cls.pubkey.chmod(0o400)
        cls.binary = cls.base / "quarantine"
        subprocess.run([
            "gcc", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
            '-DOCAF_Q_TENANT="tenant-a"',
            f'-DOCAF_Q_TRUSTED_KEY="{cls.pubkey}"',
            str(SOURCE), "-o", str(cls.binary), "-lcrypto",
        ], check=True)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def setUp(self):
        self.target = subprocess.Popen(["sleep", "10"])
        self.addCleanup(self._stop_target)
        self.boot_id = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
        stat = Path(f"/proc/{self.target.pid}/stat").read_text()
        self.start_ticks = int(stat.rpartition(") ")[2].split()[19])

    def _stop_target(self):
        self.target.terminate()
        try:
            self.target.wait(timeout=1)
        except subprocess.TimeoutExpired:
            self.target.kill()
            self.target.wait(timeout=1)

    def permit(self, *, tenant="tenant-a", start=None, count=3,
               expires=None, hold_ms=150, corrupt_signature=False):
        start = self.start_ticks if start is None else start
        expires = int(time.time()) + 60 if expires is None else expires
        evidence = hashlib.sha256(b"guest openat + host cgroup evidence").hexdigest()
        content = (f"OCAF039Q1|{tenant}|{'a' * 32}|{self.boot_id}|{self.target.pid}|"
                   f"{start}|17|107|{count}|{hold_ms}|{expires}|{evidence}\n").encode()
        permit = self.base / f"permit-{time.time_ns()}"
        signature = self.base / f"signature-{time.time_ns()}"
        permit.write_bytes(content)
        raw = self.key.sign(content)
        signature.write_bytes(bytes([raw[0] ^ 1]) + raw[1:] if corrupt_signature else raw)
        self.addCleanup(permit.unlink)
        self.addCleanup(signature.unlink)
        return permit, signature

    def test_reversible_stop_and_release_of_exact_child(self):
        permit, signature = self.permit()
        proc = subprocess.Popen([str(self.binary), str(permit), str(signature)],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        first = proc.stdout.readline()
        if not first.startswith("OCAF039_GUEST_QUARANTINE_STOPPED"):
            _, err = proc.communicate(timeout=3)
            self.fail(f"quarantine failed before stop: {first!r} {err!r}")
        state = Path(f"/proc/{self.target.pid}/status").read_text().split("State:\t", 1)[1][0]
        self.assertIn(state, "tT")
        out, err = proc.communicate(timeout=3)
        self.assertEqual(proc.returncode, 0, err)
        self.assertIn("OCAF039_GUEST_QUARANTINE_RELEASED", out)
        self.assertIsNone(self.target.poll())

    def test_untrusted_or_stale_permits_never_stop_target(self):
        cases = (
            {"corrupt_signature": True},
            {"tenant": "tenant-b"},
            {"start": self.start_ticks + 1},
            {"count": 1},
            {"expires": int(time.time()) - 1},
        )
        for case in cases:
            with self.subTest(case=case):
                permit, signature = self.permit(**case)
                result = subprocess.run([str(self.binary), str(permit), str(signature)],
                                        capture_output=True, text=True, timeout=2)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("STOPPED", result.stdout)
                self.assertIsNone(self.target.poll())


if __name__ == "__main__":
    unittest.main()
