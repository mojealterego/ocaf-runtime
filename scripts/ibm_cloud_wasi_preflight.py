#!/usr/bin/env python3
"""Read-only IBM Cloud VPC Linux host readiness check for OCAF 0.38.

Run INSIDE a staging VSI, not against the public IBM Cloud API. This script
does not provision, install, restart, kill processes, or read cloud credentials.
"""
from __future__ import annotations

import argparse
import ctypes
import fcntl
import json
import os
from pathlib import Path
import platform
import shutil
import sys
from datetime import datetime, timezone


REQUIRED_CONTROLLERS = frozenset({"cpu", "memory", "pids"})


def controllers_ready(text: str) -> bool:
    return REQUIRED_CONTROLLERS.issubset(text.split())


def memory_kib(text: str) -> int | None:
    for line in text.splitlines():
        if line.startswith("MemTotal:"):
            parts = line.split()
            return int(parts[1]) if len(parts) >= 2 and parts[1].isdigit() else None
    return None


def inspect_memfd_seals() -> tuple[bool, str]:
    if not hasattr(os, "memfd_create"):
        return False, "Python/Linux memfd_create unavailable"
    try:
        fd = os.memfd_create("ocaf-038-readiness", os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING)
        try:
            os.write(fd, b"probe-only")
            required = (
                fcntl.F_SEAL_WRITE
                | fcntl.F_SEAL_GROW
                | fcntl.F_SEAL_SHRINK
                | fcntl.F_SEAL_SEAL
            )
            fcntl.fcntl(fd, fcntl.F_ADD_SEALS, required)
            actual = fcntl.fcntl(fd, fcntl.F_GET_SEALS)
            return (actual & required) == required, "Linux immutable memfd F_ADD_SEALS/F_GET_SEALS"
        finally:
            os.close(fd)
    except (AttributeError, OSError) as exc:
        return False, f"memfd sealing failed: {type(exc).__name__}"


def inspect_pdeathsig() -> tuple[bool, str]:
    try:
        libc = ctypes.CDLL(None, use_errno=True)
        result = ctypes.c_int()
        # PR_GET_PDEATHSIG=2 reads state only: never changes signals.
        code = libc.prctl(2, ctypes.byref(result), 0, 0, 0)
        if code != 0:
            return False, f"PR_GET_PDEATHSIG failed errno={ctypes.get_errno()}"
        return True, "PR_GET_PDEATHSIG syscall supported"
    except (OSError, AttributeError) as exc:
        return False, f"prctl probe failed: {type(exc).__name__}"


def build_report() -> dict:
    checks: list[dict[str, object]] = []

    def add(name: str, ok: bool, detail: str) -> None:
        checks.append({"name": name, "ok": bool(ok), "detail": detail})

    linux = sys.platform == "linux"
    add("linux", linux, platform.system())
    add("x86_64", platform.machine() in ("x86_64", "AMD64"), platform.machine())
    add("proc_fd", Path("/proc/self/fd").is_dir(), "/proc/self/fd")
    controllers = Path("/sys/fs/cgroup/cgroup.controllers")
    try:
        available = controllers.read_text()
        ok = controllers_ready(available)
        add("cgroup_v2_controllers", ok, "available: " + " ".join(sorted(available.split())))
    except OSError:
        add("cgroup_v2_controllers", False, "unavailable or cgroup v1")
    init_comm = Path("/proc/1/comm")
    try:
        init = init_comm.read_text().strip()
    except OSError:
        init = "unavailable"
    add("systemd_pid1", init == "systemd", "PID 1: " + init)
    add("systemctl", shutil.which("systemctl") is not None, "systemctl executable")
    sealed, info = inspect_memfd_seals() if linux else (False, "Linux-only check")
    add("immutable_memfd", sealed, info)
    signal_ok, info = inspect_pdeathsig() if linux else (False, "Linux-only check")
    add("parent_death_signal", signal_ok, info)
    add("cargo", shutil.which("cargo") is not None, "Cargo needed for acceptance testing")
    add("git", shutil.which("git") is not None, "Git needed for commit verification")
    try:
        ram_kib = memory_kib(Path("/proc/meminfo").read_text())
    except OSError:
        ram_kib = None
    # Heuristic, not an IBM billing/profile requirement. Warn, do not
    # silently change or provision any resources.
    warnings = []
    if ram_kib is None or ram_kib < 8 * 1024 * 1024:
        warnings.append("Recommend at least 8 GiB RAM for in-host Rust/Wasmtime acceptance")
    return {
        "schema_version": 1,
        "purpose": "OCAF 0.38 IBM Cloud VPC staging readiness; not production approval",
        "checked_at_utc": datetime.now(timezone.utc).isoformat(),
        "kernel": platform.release(),
        "checks": checks,
        "passed": all(bool(item["ok"]) for item in checks),
        "observed_total_memory_kib": ram_kib,
        "warnings": warnings,
        "external_gates": [
            "IBM Cloud VPC network and IAM reviewed",
            "systemd unit deployed under dedicated user and cgroup",
            "signed WASI manifest and approved Wasmtime configured by operator",
            "full WASI real-engine stress evidence and failure injection",
            "independent security sign-off"
        ]
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="Optional JSON report, created only if absent")
    args = parser.parse_args()
    report = build_report()
    payload = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        previous = os.umask(0o077)
        try:
            with args.report.open("x", encoding="utf-8") as fp:
                fp.write(payload)
        finally:
            os.umask(previous)
    else:
        print(payload, end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
