"""Real host Wasmtime and kernel seccomp negatives; guest integration is separate."""
from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_wasi_budget_lab as budget

PINNED_WASMTIME_SHA = "b213b0678c0afc807c53130e5b7ca357f0307c1f8deb18cac35fcc65480b9c85"
WASMTIME = Path(os.environ["OCAF039_WASMTIME"]) if "OCAF039_WASMTIME" in os.environ else None
SOURCE = Path(__file__).resolve().parents[2] / "stage039/guest-wasi-negative/seccomp_socket_probe.c"


class WasiNegativeBudgets(unittest.TestCase):
    def test_rejects_mutated_pinned_fixture_before_executing(self):
        fixture=budget.FIXTURES / "fuel_spin.wat"
        with self.assertRaises(budget.BudgetError):
            budget._pinned(fixture, "0" * 64, 1024**2, False)

    @unittest.skipUnless(WASMTIME is not None, "set OCAF039_WASMTIME to pinned v49.0.3 CLI")
    def test_real_wasmtime_fuel_timeout_memory_cpu_and_parent_kill(self):
        for mode in ("fuel", "timeout", "memory", "cpu_signal", "parent_kill"):
            with self.subTest(mode=mode):
                result=budget.run_case(WASMTIME,PINNED_WASMTIME_SHA,mode)
                self.assertEqual(result["status"],"HOST_WASMTIME_NEGATIVE_BUDGET_VERIFIED")
                self.assertFalse(result["guest_kvm_tested"])
                self.assertNotEqual(result["returncode"],0)
        with self.assertRaises(budget.BudgetError):
            budget.run_case(WASMTIME,"0"*64,"fuel")

    def test_real_kernel_seccomp_does_not_allow_socket(self):
        if shutil.which("gcc") is None:
            self.skipTest("gcc unavailable")
        with tempfile.TemporaryDirectory(prefix="ocaf039-seccomp-",dir="/dev/shm") as directory:
            binary=Path(directory)/"probe"
            subprocess.run(["gcc","-std=c11","-O2","-static","-Wall","-Wextra","-Werror",
                            str(SOURCE),"-o",str(binary)],check=True,capture_output=True)
            completed=subprocess.run([str(binary)],check=False,capture_output=True,
                                     text=True,timeout=3)
            self.assertEqual(completed.returncode,0,completed.stderr)
            self.assertEqual(completed.stdout,"OCAF039_SECCOMP_SOCKET_DENIED\n")


if __name__ == "__main__":
    unittest.main()
