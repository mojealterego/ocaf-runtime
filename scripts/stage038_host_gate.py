#!/usr/bin/env python3
"""Read-only, fail-closed Linux x86_64 host/systemd evidence for OCAF 0.38.

Never creates/starts/stops a service, provisions resources or grants release GO.
With --unit it inspects an ALREADY deployed staging service and its cgroup.
Without --unit it only validates the current Linux host prerequisites.
"""
from __future__ import annotations

import argparse
import ctypes
import fcntl
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone

CGROUP_ROOT = Path("/sys/fs/cgroup")
REQUIRED_CONTROLLERS = frozenset({"cpu", "memory", "pids"})
UNIT_NAME = re.compile(r"ocaf-[a-z0-9@_.-]{1,100}\.service\Z")
MAX_MEMORY_BYTES = 2 * 1024 ** 3
MAX_TASKS = 256

def parse_unit_properties(output: str) -> dict[str, str]:
    """Reject duplicate, malformed and unexpected systemctl keys."""
    allowed = {
        "Id", "LoadState", "ActiveState", "User", "NoNewPrivileges",
        "KillMode", "MemoryMax", "TasksMax", "CPUQuotaPerSecUSec",
        "PrivateTmp", "ProtectSystem", "ControlGroup", "FragmentPath",
    }
    result: dict[str, str] = {}
    for line in output.splitlines():
        key, sep, value = line.partition("=")
        if sep != "=" or key not in allowed or key in result or len(value) > 1024:
            raise ValueError("malformed or duplicate systemd property")
        result[key] = value
    if set(result) != allowed:
        raise ValueError("incomplete systemd properties")
    return result

def finite_positive_bound(value: str, maximum: int) -> bool:
    return value.isdecimal() and 0 < int(value) <= maximum

def cpu_quota_bounded(value: str) -> bool:
    """systemd D-Bus property is formatted as e.g. '2s' or 'infinity'."""
    if not value.endswith("s") or value == "infinity":
        return False
    try:
        n = float(value[:-1])
    except ValueError:
        return False
    return 0 < n <= 2.0

def read_cgroup_stats(path: Path) -> dict[str, object]:
    """Read scalar cgroup counters; absence stays visible, never fabricated."""
    root = CGROUP_ROOT.resolve()
    target = path.resolve()
    if not target.is_relative_to(root):
        raise ValueError("cgroup traversal rejected")
    out: dict[str, object] = {}
    for name in ("memory.peak", "memory.current", "memory.events",
                 "pids.peak", "pids.current", "cpu.stat",
                 "memory.max", "pids.max", "cpu.max"):
        file = target / name
        try:
            data = file.read_text(encoding="ascii")
            if len(data) > 4096:
                raise ValueError("cgroup metric oversized")
            out[name] = data.strip()
        except (OSError, UnicodeError, ValueError):
            out[name] = None
    return out


def kernel_limits_enforced(metrics: dict[str, object]) -> bool:
    """Kernel cgroup v2 limit files, not merely configured systemd properties."""
    mem = metrics.get("memory.max")
    pids = metrics.get("pids.max")
    cpu = metrics.get("cpu.max")
    if not isinstance(mem, str) or not finite_positive_bound(mem, MAX_MEMORY_BYTES):
        return False
    if not isinstance(pids, str) or not finite_positive_bound(pids, MAX_TASKS):
        return False
    if not isinstance(cpu, str):
        return False
    pair = cpu.split()
    if len(pair) != 2 or not all(x.isdecimal() for x in pair):
        return False
    quota, period = map(int, pair)
    return quota > 0 and period > 0 and quota <= 2 * period

def check_memfd_sealing() -> bool:
    if not hasattr(os, "memfd_create") or not hasattr(os, "MFD_ALLOW_SEALING"):
        return False
    try:
        fd = os.memfd_create("ocaf-stage038-host-probe",
                             os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING)
        try:
            os.write(fd, b"staging-probe")
            seals = (fcntl.F_SEAL_WRITE | fcntl.F_SEAL_SHRINK |
                     fcntl.F_SEAL_GROW | fcntl.F_SEAL_SEAL)
            fcntl.fcntl(fd, fcntl.F_ADD_SEALS, seals)
            return (fcntl.fcntl(fd, fcntl.F_GET_SEALS) & seals) == seals
        finally:
            os.close(fd)
    except (OSError, AttributeError):
        return False

def check_pdeathsig() -> bool:
    try:
        libc = ctypes.CDLL(None, use_errno=True)
        value = ctypes.c_int()
        return libc.prctl(2, ctypes.byref(value), 0, 0, 0) == 0
    except (OSError, AttributeError):
        return False

def build_report(unit: str | None = None) -> dict[str, object]:
    checks: dict[str, bool] = {}
    checks["linux_x86_64"] = sys.platform == "linux" and platform.machine() in ("x86_64", "AMD64")
    checks["proc_fd"] = Path("/proc/self/fd").is_dir()
    checks["systemd_pid1"] = False
    try:
        checks["systemd_pid1"] = Path("/proc/1/comm").read_text().strip() == "systemd"
    except OSError:
        pass
    checks["systemctl_installed"] = shutil.which("systemctl") is not None
    controllers = ""
    try:
        controllers = (CGROUP_ROOT / "cgroup.controllers").read_text()
    except OSError:
        pass
    checks["cgroup_v2_cpu_memory_pids"] = REQUIRED_CONTROLLERS.issubset(controllers.split())
    checks["memfd_sealed"] = check_memfd_sealing() if checks["linux_x86_64"] else False
    checks["pdeathsig_supported"] = check_pdeathsig() if checks["linux_x86_64"] else False

    record: dict[str, object] = {
        "schema_version": 1,
        "purpose": "OCAF 0.38 factual host acceptance probe; not production approval",
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "architecture": platform.machine(),
        "kernel": platform.release(),
        "mode": "staging-unit" if unit else "host-only",
        "checks": checks,
        "cgroup_controllers": sorted(controllers.split()),
        "unit": None,
        "metrics": None,
        "production_approved": False,
        "stage_038_complete": False,
        "external_gates": [
            "real multi-tenant staging with independent mTLS endpoints",
            "controlled coordinator SIGKILL/restart, cgroup limit stress and partition recovery",
            "operator-signed provenance, key rotation and revocation",
            "independent adversarial security audit and formal operator sign-off",
        ],
    }
    if unit is not None:
        if not UNIT_NAME.fullmatch(unit):
            raise ValueError("staging unit must be a literal ocaf-*.service name")
        keys = ",".join([
            "Id", "LoadState", "ActiveState", "User", "NoNewPrivileges",
            "KillMode", "MemoryMax", "TasksMax", "CPUQuotaPerSecUSec",
            "PrivateTmp", "ProtectSystem", "ControlGroup", "FragmentPath",
        ])
        try:
            proc = subprocess.run(
                ["systemctl", "show", unit, "--property=" + keys, "--no-pager"],
                capture_output=True, text=True, check=False, timeout=8,
            )
            if proc.returncode != 0 or len(proc.stdout) > 8192:
                raise ValueError("systemctl show unavailable")
            p = parse_unit_properties(proc.stdout)
            unit_checks = {
                "unit_matches_requested": p["Id"] == unit,
                "unit_loaded_active": p["LoadState"] == "loaded" and p["ActiveState"] == "active",
                "dedicated_unprivileged_user": p["User"] == "ocaf",
                "no_new_privileges": p["NoNewPrivileges"] == "yes",
                "cgroup_kill_mode": p["KillMode"] == "control-group",
                "private_tmp": p["PrivateTmp"] == "yes",
                "protected_system": p["ProtectSystem"] == "strict",
                "bounded_memory": finite_positive_bound(p["MemoryMax"], MAX_MEMORY_BYTES),
                "bounded_tasks": finite_positive_bound(p["TasksMax"], MAX_TASKS),
                "bounded_cpu": cpu_quota_bounded(p["CPUQuotaPerSecUSec"]),
                "unit_cgroup_exists": False,
            }
            cg = p["ControlGroup"]
            if cg.startswith("/") and ".." not in cg.split("/"):
                cgpath = (CGROUP_ROOT / cg.lstrip("/")).resolve()
                if cgpath.is_relative_to(CGROUP_ROOT.resolve()) and cgpath.is_dir():
                    unit_checks["unit_cgroup_exists"] = True
                    record["metrics"] = read_cgroup_stats(cgpath)
                    checks["unit_kernel_limits_enforced"] = kernel_limits_enforced(record["metrics"])
                    checks["unit_resource_accounting"] = all(
                        record["metrics"].get(k) is not None
                        for k in ("memory.peak", "memory.events", "pids.current", "cpu.stat")
                    )
            checks.update(unit_checks)
            record["unit"] = {
                "id": p["Id"], "fragment_path": p["FragmentPath"],
                "control_group": p["ControlGroup"],
            }
        except (OSError, ValueError, subprocess.TimeoutExpired):
            checks["systemd_unit_verifiable"] = False
    record["passed"] = all(checks.values())
    record["status"] = (
        "NO_GO" if not record["passed"]
        else "STAGING_CONFIGURATION_VERIFIED_UNAUDITED" if unit
        else "HOST_PREREQUISITES_VERIFIED_ONLY"
    )
    return record

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--unit", help="existing OCAF staging systemd service; never started")
    parser.add_argument("--report", type=Path, required=True,
                        help="new private JSON file; never overwritten")
    args = parser.parse_args()
    try:
        report = build_report(args.unit)
        args.report.parent.mkdir(parents=True, exist_ok=True)
        prior = os.umask(0o077)
        try:
            with args.report.open("x", encoding="utf-8") as fp:
                json.dump(report, fp, indent=2, sort_keys=True)
                fp.write("\n")
        finally:
            os.umask(prior)
    except (ValueError, FileExistsError, OSError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status": report["status"], "passed": report["passed"],
                      "mode": report["mode"], "report": str(args.report)}))
    return 0 if report["passed"] else 1

if __name__ == "__main__":
    raise SystemExit(main())
