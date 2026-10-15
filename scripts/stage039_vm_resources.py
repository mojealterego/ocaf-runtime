#!/usr/bin/env python3
"""Stage 0.39.8: apply and measure actual cgroup-v2 VM resource limits.

The resource envelope constrains the HOST Firecracker/jailer process. A
guest's memory configuration alone is NOT a host CPU, memory, or PID limit.
A valid report does not prove staging-host independence or production safety.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import stat

ROOT = Path("/sys/fs/cgroup")
DEDICATED_VM = re.compile(r"ocaf-stage039-vm-[a-f0-9]{16}\.service\Z")
INTEGER = re.compile(r"(?:0|[1-9][0-9]*)\Z")
EVENT_NAME = re.compile(r"[a-z][a-z0-9_]*\Z")
MAX_READ = 4096
FILES = (
    "memory.max", "memory.swap.max", "memory.oom.group",
    "memory.current", "memory.peak", "memory.events",
    "pids.max", "pids.current", "pids.peak", "pids.events",
    "cpu.max", "cpu.stat",
)


class VmResourceError(ValueError):
    pass


def valid_target(path: Path) -> bool:
    """Only an immediate, dedicated cgroup under the unified root is allowed."""
    return (isinstance(path, Path) and path.is_absolute()
            and ".." not in path.parts and "." not in path.parts
            and path.parent == ROOT
            and DEDICATED_VM.fullmatch(path.name) is not None)


def desired_limits(guest_mem_mib: int) -> dict[str, str]:
    """Bounded 2-vCPU process budget with >=512 MiB host-side memory overhead."""
    if type(guest_mem_mib) is not int or not 64 <= guest_mem_mib <= 2048:
        raise VmResourceError("invalid bounded guest RAM configuration")
    host_mib = max(guest_mem_mib + 512, 2 * guest_mem_mib)
    return {
        "memory.max": str(host_mib * 1024 * 1024),
        "memory.swap.max": "0",
        "memory.oom.group": "1",
        "pids.max": "96",
        "cpu.max": "200000 100000",
    }


def _positive_int(raw: str, label: str) -> int:
    if not isinstance(raw, str) or INTEGER.fullmatch(raw) is None:
        raise VmResourceError("malformed kernel integer: " + label)
    return int(raw)


def _events(raw: str, name: str) -> dict[str, int]:
    if not isinstance(raw, str) or not 0 < len(raw) <= MAX_READ:
        raise VmResourceError("missing or oversized kernel events: " + name)
    found: dict[str, int] = {}
    for line in raw.splitlines():
        parts = line.split()
        if len(parts) != 2 or EVENT_NAME.fullmatch(parts[0]) is None:
            raise VmResourceError("malformed kernel events: " + name)
        if parts[0] in found:
            raise VmResourceError("duplicate kernel event: " + parts[0])
        found[parts[0]] = _positive_int(parts[1], name)
    return found


def verify_resource_snapshot(snapshot: dict[str, str],
                             guest_mem_mib: int) -> dict:
    """Fail closed if any kernel limit or observed event violates policy."""
    expected = desired_limits(guest_mem_mib)
    if not isinstance(snapshot, dict) or set(snapshot) != set(FILES):
        raise VmResourceError("incomplete resource telemetry")
    if any(not isinstance(value, str) or len(value) > MAX_READ
           for value in snapshot.values()):
        raise VmResourceError("invalid resource telemetry value")
    for name, expected_value in expected.items():
        if snapshot[name].strip() != expected_value:
            raise VmResourceError("cgroup limit mismatch: " + name)

    current = _positive_int(snapshot["memory.current"].strip(), "memory.current")
    peak = _positive_int(snapshot["memory.peak"].strip(), "memory.peak")
    pids_current = _positive_int(snapshot["pids.current"].strip(), "pids.current")
    pids_peak = _positive_int(snapshot["pids.peak"].strip(), "pids.peak")
    if (current > int(expected["memory.max"])
            or peak > int(expected["memory.max"])
            or pids_current > int(expected["pids.max"])
            or pids_peak > int(expected["pids.max"])):
        raise VmResourceError("observed resource peak exceeds hard cgroup limit")

    mem = _events(snapshot["memory.events"], "memory.events")
    pid = _events(snapshot["pids.events"], "pids.events")
    cpu = _events(snapshot["cpu.stat"], "cpu.stat")
    if not {"oom", "oom_kill"}.issubset(mem) or "max" not in pid or "usage_usec" not in cpu:
        raise VmResourceError("required kernel event counters absent")
    if any(mem.get(key, 0) != 0 for key in ("oom", "oom_kill", "oom_group_kill")):
        raise VmResourceError("VM resource envelope has OOM evidence")
    if pid["max"] != 0:
        raise VmResourceError("VM resource envelope hit task quota")

    return {
        "schema_version": 1,
        "status": "KERNEL_VM_CGROUP_LIMITS_ENFORCED",
        "guest_mem_mib": guest_mem_mib,
        "kernel_limits": dict(expected),
        "observed_memory_current_bytes": current,
        "observed_memory_peak_bytes": peak,
        "observed_pids_current": pids_current,
        "observed_pids_peak": pids_peak,
        "observed_cpu_usage_usec": cpu["usage_usec"],
        "kernel_oom_count": mem["oom"],
        "kernel_oom_kill_count": mem["oom_kill"],
        "kernel_pids_quota_hits": pid["max"],
        "guest_network_egress_tested": False,
        "stage_039_complete": False,
        "production_approved": False,
    }


def _open_dir(path: Path) -> int:
    if not valid_target(path):
        raise VmResourceError("only a dedicated top-level VM cgroup is permitted")
    try:
        fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC
                     | getattr(os, "O_NOFOLLOW", 0))
        info = os.fstat(fd)
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0:
            os.close(fd)
            raise VmResourceError("VM cgroup must be a root-owned directory")
        return fd
    except OSError as exc:
        raise VmResourceError("cannot pin VM cgroup directory") from exc


def _read(fd: int, name: str) -> str:
    if name not in FILES:
        raise VmResourceError("unrecognized cgroup telemetry file")
    f = os.open(name, os.O_RDONLY | os.O_CLOEXEC
                | getattr(os, "O_NOFOLLOW", 0), dir_fd=fd)
    try:
        with os.fdopen(f, "rb", closefd=False) as stream:
            raw = stream.read(MAX_READ + 1)
        if len(raw) > MAX_READ:
            raise VmResourceError("cgroup telemetry too long")
        return raw.decode("ascii").strip()
    finally:
        os.close(f)


def read_resource_snapshot(path: Path) -> dict[str, str]:
    fd = _open_dir(path)
    try:
        return {name: _read(fd, name) for name in FILES}
    except (OSError, UnicodeError) as exc:
        raise VmResourceError("unreadable kernel cgroup telemetry") from exc
    finally:
        os.close(fd)


def apply_resource_limits(path: Path, guest_mem_mib: int) -> dict:
    """Set limits BEFORE starting VM; verify kernel readback from pinned dirfd."""
    limits = desired_limits(guest_mem_mib)
    fd = _open_dir(path)
    try:
        for name, value in limits.items():
            f = os.open(name, os.O_WRONLY | os.O_CLOEXEC
                        | getattr(os, "O_NOFOLLOW", 0), dir_fd=fd)
            try:
                data = value.encode("ascii")
                if os.write(f, data) != len(data):
                    raise VmResourceError("incomplete kernel cgroup limit write")
            finally:
                os.close(f)
        snapshot = {name: _read(fd, name) for name in FILES}
        result = verify_resource_snapshot(snapshot, guest_mem_mib)
        result["vm_cgroup_inode"] = os.fstat(fd).st_ino
        return result
    except (OSError, UnicodeError) as exc:
        raise VmResourceError("kernel does not support required VM resource controller") from exc
    finally:
        os.close(fd)


def audit_running_resource_limits(path: Path, guest_mem_mib: int,
                                  pinned_inode: int) -> dict:
    """Check effective bounds and OOM after the VM ran; never use config alone."""
    fd = _open_dir(path)
    try:
        if type(pinned_inode) is not int or os.fstat(fd).st_ino != pinned_inode:
            raise VmResourceError("VM cgroup inode replaced")
        snapshot = {name: _read(fd, name) for name in FILES}
        result = verify_resource_snapshot(snapshot, guest_mem_mib)
        result["vm_cgroup_inode"] = pinned_inode
        return result
    except (OSError, UnicodeError) as exc:
        raise VmResourceError("cannot audit running VM resource limits") from exc
    finally:
        os.close(fd)
