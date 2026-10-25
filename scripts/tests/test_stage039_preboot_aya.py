"""Fail-closed supervisor checks for the preboot host Aya lifecycle."""
from __future__ import annotations

import io
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_preboot_aya as preboot


class FakeGuard:
    def __init__(self, receipt: str = ""):
        self.stdin = io.StringIO()
        self.receipt = receipt
        self.returncode = None

    def poll(self):
        return self.returncode

    def communicate(self, timeout):
        self.returncode = 0
        return self.receipt, ""


class PrebootAyaTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="ocaf039-preboot-")
        self.addCleanup(self.tmp.cleanup)
        self.cgroup = Path(self.tmp.name) / "vm"
        self.cgroup.mkdir()
        self.guard = preboot.PrebootAya(
            cgroup=self.cgroup, loader=Path("/unused/loader"), loader_sha="a" * 64,
            program=Path("/unused/program"), program_sha="b" * 64,
            lifetime_seconds=30)
        self.guard.proc = FakeGuard()
        self.guard.program_id = 17
        self.guard.cgroup_inode = self.cgroup.stat().st_ino

    def test_rejects_early_exit_and_replaced_cgroup_inode(self):
        self.guard.proc.returncode = 1
        with self.assertRaises(preboot.PrebootAyaError):
            self.guard.require_live()
        self.guard.proc.returncode = None
        self.cgroup.rename(self.cgroup.parent / "old")
        self.cgroup.mkdir()
        with self.assertRaises(preboot.PrebootAyaError):
            self.guard.require_live()

    def test_release_rejects_detach_for_different_program(self):
        self.guard.proc.receipt = (
            '{"status":"AYA_CGROUP_EGRESS_DETACHED",'
            '"program_id":18,"production_approved":false}\n'
        )
        with self.assertRaises(preboot.PrebootAyaError):
            self.guard.release()


if __name__ == "__main__":
    unittest.main()
