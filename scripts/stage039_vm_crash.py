#!/usr/bin/env python3
"""Stage 0.39.9: kernel SIGKILL fault test for a dedicated jailed Firecracker.

Only a proven running OCAF-owned VM process can be signalled. The test
requires waitpid(SIGKILL) and cgroup-v2 populated=0 for the FULL hierarchy,
not only the process's immediate cgroup.procs. This is ephemeral CI fault
evidence, not a signed production crash/recovery/HA acceptance.
"""
from __future__ import annotations

from datetime import datetime, timezone
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import time

import stage039_vm_egress as vm_egress

MAX_EVENTS = 1024
FIELD = re.compile(r"[a-z][a-z0-9_]*\Z")
UNSIGNED = re.compile(r"(?:0|[1-9][0-9]*)\Z")


class VmCrashError(ValueError):
    pass


def validate_drained_subtree(events: str) -> dict:
    if not isinstance(events, str) or not 0 < len(events) <= MAX_EVENTS:
        raise VmCrashError("missing or oversized cgroup.events report")
    parsed = {}
    for line in events.splitlines():
        pair = line.split()
        if (len(pair) != 2 or FIELD.fullmatch(pair[0]) is None
                or UNSIGNED.fullmatch(pair[1]) is None
                or pair[0] in parsed):
            raise VmCrashError("malformed or duplicate kernel cgroup.events field")
        parsed[pair[0]] = int(pair[1])
    if "populated" not in parsed or parsed["populated"] != 0:
        raise VmCrashError("VM subtree still contains running processes")
    return {
        "status": "VM_CGROUP_SUBTREE_EMPTY_AFTER_SIGKILL",
        "kernel_cgroup_populated": 0,
    }


def _cgroup_events(cgroup: Path, pinned_inode: int) -> str:
    if (not vm_egress.egress.permitted_probe_cgroup(cgroup)
            or not vm_egress.VM_CGROUP.fullmatch(cgroup.name)
            or type(pinned_inode) is not int or pinned_inode <= 0):
        raise VmCrashError("untrusted cgroup scope for SIGKILL confirmation")
    flags = os.O_RDONLY | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0)
    dirfd = None
    fd = None
    try:
        dirfd = os.open(cgroup, flags | os.O_DIRECTORY)
        info = os.fstat(dirfd)
        if (not stat.S_ISDIR(info.st_mode) or info.st_uid != 0
                or info.st_ino != pinned_inode):
            raise VmCrashError("VM cgroup inode replaced before SIGKILL proof")
        fd = os.open("cgroup.events", flags, dir_fd=dirfd)
        raw = os.read(fd, MAX_EVENTS + 1)
        if len(raw) > MAX_EVENTS:
            raise VmCrashError("oversized cgroup.events")
        return raw.decode("ascii")
    except (OSError, UnicodeError) as exc:
        raise VmCrashError("kernel cgroup.events unavailable") from exc
    finally:
        if fd is not None:
            os.close(fd)
        if dirfd is not None:
            os.close(dirfd)


def kill_and_verify(proc: subprocess.Popen, vm_id: str, cgroup: Path,
                    deadline_seconds: int = 5) -> dict:
    """SIGKILL only the original verified VM PID, then verify subtree empty."""
    if type(deadline_seconds) is not int or not 1 <= deadline_seconds <= 10:
        raise VmCrashError("unbounded VM crash test deadline")
    try:
        inode = vm_egress.require_vm_running(proc, vm_id, cgroup)
    except (vm_egress.VmCgroupEgressError, OSError, ValueError) as exc:
        raise VmCrashError("untrusted VM process; SIGKILL prohibited") from exc
    if proc.poll() is not None:
        raise VmCrashError("VM exited before intentional SIGKILL")
    try:
        proc.kill()
        code = proc.wait(timeout=deadline_seconds)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise VmCrashError("VM SIGKILL failed or waitpid timed out") from exc
    if code != -signal.SIGKILL:
        raise VmCrashError("VM did not terminate from expected SIGKILL")

    deadline = time.monotonic() + deadline_seconds
    while True:
        try:
            drained = validate_drained_subtree(_cgroup_events(cgroup, inode))
            break
        except VmCrashError as exc:
            if time.monotonic() >= deadline:
                raise VmCrashError(
                    "VM cgroup subtree not fully reaped after SIGKILL"
                ) from exc
            time.sleep(0.1)
    return {
        "schema_version": 1,
        "status": "VM_SIGKILL_CGROUP_SUBTREE_DRAINED",
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "vm_id": vm_id,
        "firecracker_pid": proc.pid,
        "vm_cgroup": cgroup.name,
        "vm_cgroup_inode": inode,
        "sigkill_executed": True,
        "sigkill_exit_code": code,
        "cgroup_subtree_empty": drained["kernel_cgroup_populated"] == 0,
        "host_reboot_tested": False,
        "multi_host_failover_tested": False,
        "stage_039_complete": False,
        "production_approved": False,
    }
