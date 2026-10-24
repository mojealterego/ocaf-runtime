#!/usr/bin/env python3
"""Run real Wasmtime WASIp1 negative budgets on the current Linux host.

This harness verifies a pinned Wasmtime executable and pinned WAT fixtures,
executes actual engine traps and a parent SIGKILL, then reports HOST scope.
It does not substitute for running the same modules inside Firecracker.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "stage039" / "guest-wasi-negative"
MODULES = {
    "fuel": ("fuel_spin.wat", "c2cde9d58faa091dd504e732a216677535fb662f5b4345f3c363924900d73494"),
    "timeout": ("fuel_spin.wat", "c2cde9d58faa091dd504e732a216677535fb662f5b4345f3c363924900d73494"),
    "cpu_signal": ("fuel_spin.wat", "c2cde9d58faa091dd504e732a216677535fb662f5b4345f3c363924900d73494"),
    "parent_kill": ("fuel_spin.wat", "c2cde9d58faa091dd504e732a216677535fb662f5b4345f3c363924900d73494"),
    "memory": ("memory_grow.wat", "932cf0d95f147d552b8580a0a0b7726abea7904e7ed7f7a68eaf04af3357b3a1"),
}
TRAPS = {
    "fuel": "wasm trap: all fuel consumed by WebAssembly",
    "timeout": "wasm trap: interrupt",
    "memory": "forcing trap when growing memory to 196608 bytes",
}
SHA256 = re.compile(r"[0-9a-f]{64}\Z")


class BudgetError(ValueError):
    pass


def _pinned(path: Path, digest: str, limit: int, executable: bool) -> None:
    if (not isinstance(path, Path) or not path.is_absolute() or path.is_symlink()
        or not path.is_file() or not isinstance(digest, str)
        or SHA256.fullmatch(digest) is None):
        raise BudgetError("untrusted budget test asset")
    stat = path.stat()
    if (not 0 < stat.st_size <= limit or stat.st_nlink != 1
        or stat.st_mode & 0o022 or executable and not os.access(path, os.X_OK)):
        raise BudgetError("unbounded or mutable budget test asset")
    with path.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    if actual != digest:
        raise BudgetError("pinned budget test asset digest mismatch")


def _child_limits(mode: str) -> None:
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_CPU, (1, 2) if mode == "cpu_signal" else (2, 2))
    resource.setrlimit(resource.RLIMIT_AS, (8 * 1024**3, 8 * 1024**3))
    # A subprocess cannot gain privileges even if the binary is replaced.
    import ctypes
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(38, 1, 0, 0, 0) != 0:  # PR_SET_NO_NEW_PRIVS
        raise OSError(ctypes.get_errno(), "PR_SET_NO_NEW_PRIVS")


def run_case(wasmtime: Path, wasmtime_sha256: str, mode: str) -> dict:
    if mode not in MODULES:
        raise BudgetError("unknown negative WASI budget case")
    module_name, module_sha = MODULES[mode]
    module = FIXTURES / module_name
    _pinned(wasmtime, wasmtime_sha256, 256 * 1024**2, True)
    _pinned(module, module_sha, 1024**2, False)
    flags = {
        "fuel": ["-W", "fuel=10000"],
        "timeout": ["-W", "timeout=100ms"],
        "memory": ["-W", "max-memory-size=65536", "-W", "trap-on-grow-failure=y"],
        "cpu_signal": [],
        "parent_kill": [],
    }[mode]
    command = [str(wasmtime), "run", "-C", "cache=no", *flags, str(module)]
    started = time.monotonic()
    proc = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, start_new_session=True,
                            preexec_fn=lambda: _child_limits(mode),
                            env={"PATH": "/usr/bin:/bin", "LANG": "C"})
    try:
        if mode == "parent_kill":
            time.sleep(0.2)
            if proc.poll() is not None:
                raise BudgetError("spin workload exited before parent SIGKILL")
            os.killpg(proc.pid, signal.SIGKILL)
            output, _ = proc.communicate(timeout=2)
            if proc.returncode != -signal.SIGKILL:
                raise BudgetError("parent SIGKILL did not terminate WASI process group")
            signal_name = "SIGKILL"
        else:
            output, _ = proc.communicate(timeout=3)
            if mode == "cpu_signal":
                if proc.returncode not in (-signal.SIGXCPU, -signal.SIGKILL):
                    raise BudgetError("RLIMIT_CPU kernel signal absent")
                signal_name = signal.Signals(-proc.returncode).name
            else:
                if proc.returncode == 0 or TRAPS[mode] not in output.decode("utf-8", "replace"):
                    raise BudgetError("expected real Wasmtime budget trap absent")
                signal_name = None
    except (subprocess.TimeoutExpired, BudgetError):
        if proc.poll() is None:
            os.killpg(proc.pid, signal.SIGKILL)
        proc.communicate(timeout=2)
        raise
    if len(output) > 8192:
        raise BudgetError("negative workload output exceeded bound")
    return {
        "status": "HOST_WASMTIME_NEGATIVE_BUDGET_VERIFIED",
        "case": mode,
        "engine_sha256": wasmtime_sha256,
        "module_sha256": module_sha,
        "returncode": proc.returncode,
        "output_sha256": hashlib.sha256(output).hexdigest(),
        "elapsed_ms": round((time.monotonic() - started) * 1000),
        "expected_trap": TRAPS.get(mode),
        "kernel_or_parent_signal": signal_name,
        "guest_kvm_tested": False,
        "production_approved": False,
        "stage_039_complete": False,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wasmtime", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    report = {"cases": {mode: run_case(args.wasmtime, args.sha256, mode)
                         for mode in ("fuel", "timeout", "memory", "cpu_signal", "parent_kill")},
              "guest_kvm_tested": False, "stage_039_complete": False,
              "production_approved": False}
    with args.report.open("x", encoding="utf-8") as file:
        json.dump(report, file, sort_keys=True, indent=2)
        file.write("\n")
    print("STAGE039_HOST_WASMTIME_NEGATIVE_BUDGETS_VERIFIED")


if __name__ == "__main__":
    main()
