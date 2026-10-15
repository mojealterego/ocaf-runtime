#!/usr/bin/env python3
"""0.39: empirical Aya deny/recovery test while a jailed Firecracker guest runs.

The packets originate from a host TCP probe in the VM's *host cgroup*, not
from the guest. A valid result does not demonstrate guest egress isolation.
Requires independently authenticated/pinned Aya executable and BPF ELF.
"""
from __future__ import annotations

from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import secrets
import subprocess

import stage039_live_egress as egress

VM_ID = re.compile(r"ocaf-[0-9a-f]{24}\Z")
VM_CGROUP = re.compile(r"ocaf-stage039-vm-[0-9a-f]{16}\.service\Z")


class VmCgroupEgressError(ValueError):
    pass


def validate_vm_identity(vm_id: str, vm_cgroup: Path,
                         pid_cgroups: str) -> bool:
    """Pin a fresh test VM cgroup, not a production or unrelated scope."""
    if not isinstance(vm_id, str) or VM_ID.fullmatch(vm_id) is None:
        raise VmCgroupEgressError("untrusted Firecracker VM identifier")
    if (not isinstance(vm_cgroup, Path)
            or vm_cgroup.parent != egress.CGROUP_ROOT
            or VM_CGROUP.fullmatch(vm_cgroup.name) is None
            or not vm_cgroup.is_absolute()):
        raise VmCgroupEgressError("untrusted VM cgroup scope")
    if not isinstance(pid_cgroups, str):
        raise VmCgroupEgressError("missing kernel process cgroup identity")
    joined = any(
        line.startswith("0::/")
        and vm_cgroup.name in line[4:].split("/")
        for line in pid_cgroups.splitlines()
    )
    if not joined:
        raise VmCgroupEgressError("VM process not resident in target cgroup")
    return True


def require_vm_running(proc: subprocess.Popen, vm_id: str,
                       vm_cgroup: Path) -> int:
    if proc.poll() is not None or type(proc.pid) is not int or proc.pid <= 0:
        raise VmCgroupEgressError("Firecracker process not running")
    if vm_cgroup.is_symlink() or not vm_cgroup.is_dir():
        raise VmCgroupEgressError("VM cgroup missing or linked")
    info = vm_cgroup.stat()
    if info.st_uid != 0:
        raise VmCgroupEgressError("VM cgroup must be root-owned")
    try:
        membership = Path(f"/proc/{proc.pid}/cgroup").read_text(encoding="ascii")
    except (OSError, UnicodeError) as exc:
        raise VmCgroupEgressError("kernel VM PID cgroup unavailable") from exc
    validate_vm_identity(vm_id, vm_cgroup, membership)
    if proc.poll() is not None:
        raise VmCgroupEgressError("Firecracker process exited during identity check")
    return info.st_ino


def evaluate_vm_host_probe(*, before_a: bool, before_b: bool,
                           attached: bool, during_a: bool, during_b: bool,
                           after_a: bool) -> dict:
    try:
        result = egress.evaluate_probes(
            before_a=before_a, before_b=before_b, attached=attached,
            during_a=during_a, during_b=during_b, after_a=after_a)
    except egress.AcceptanceError as exc:
        raise VmCgroupEgressError(str(exc)) from exc
    return {
        "status": "VM_HOST_CGROUP_EBPF_PROBE_PASSED",
        "host_vm_cgroup_egress_enforced": True,
        "other_cgroup_remained_reachable": result["tenant_b_not_blocked"],
        "network_recovered_after_detach": result["tenant_a_recovered_after_detach"],
        "guest_network_egress_tested": False,
        "stage_039_complete": False,
        "production_approved": False,
    }


def _confirmed_detach(text: str, program_id: int) -> bool:
    for line in text.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if (isinstance(event, dict)
                and event.get("status") == "AYA_CGROUP_EGRESS_DETACHED"
                and type(event.get("program_id")) is int
                and event["program_id"] == program_id
                and event.get("production_approved") is False):
            return True
    return False


def prove_vm_host_cgroup_egress(*, proc: subprocess.Popen, vm_id: str,
                               cgroup: Path, loader: Path, loader_sha: str,
                               program: Path, program_sha: str,
                               seconds: int = 15) -> dict:
    """Probe the exact *live VM host cgroup* with Aya and unaffected peer B."""
    if not 12 <= seconds <= 60:
        raise VmCgroupEgressError("bounded Aya attachment 12..60 seconds required")
    try:
        host = egress.preflight()
        vm_inode = require_vm_running(proc, vm_id, cgroup)
        egress.verified_executable(loader, loader_sha)
        if egress.sha256_file(program) != program_sha:
            raise VmCgroupEgressError("pinned eBPF ELF digest mismatch")
    except egress.AcceptanceError as exc:
        raise VmCgroupEgressError(str(exc)) from exc

    peer = egress.CGROUP_ROOT / (
        "ocaf-stage039-b-" + secrets.token_hex(8) + ".service")
    if not egress.permitted_probe_cgroup(peer) or not peer.parent == cgroup.parent:
        raise VmCgroupEgressError("peer probe cgroup not permitted")

    aya = None
    created = False
    try:
        peer.mkdir(mode=0o700)
        created = True
        if peer.is_symlink() or peer.stat().st_uid != 0:
            raise VmCgroupEgressError("peer cgroup must be root-owned")
        with egress.local_tcp_server() as port:
            before_a = egress.probe_connect(cgroup, port)
            before_b = egress.probe_connect(peer, port)
            if not (before_a and before_b):
                raise VmCgroupEgressError("VM host and peer baseline TCP probes failed")
            if require_vm_running(proc, vm_id, cgroup) != vm_inode:
                raise VmCgroupEgressError("VM cgroup inode changed before attach")

            aya = subprocess.Popen(
                [str(loader), "--program", str(program), "--sha256", program_sha,
                 "--cgroup", str(cgroup), "--seconds", str(seconds),
                 "--ack-attach"],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                text=True, env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
            )
            attached = egress.await_attach(aya, vm_inode)
            if require_vm_running(proc, vm_id, cgroup) != vm_inode:
                raise VmCgroupEgressError("VM cgroup changed during attach")
            during_a = egress.probe_connect(cgroup, port)
            during_b = egress.probe_connect(peer, port)
            if require_vm_running(proc, vm_id, cgroup) != vm_inode:
                raise VmCgroupEgressError("VM process stopped during Aya probe")
            try:
                tail, _ = aya.communicate(timeout=seconds + 8)
            except subprocess.TimeoutExpired as exc:
                raise VmCgroupEgressError("Aya did not detach within deadline") from exc
            if aya.returncode != 0 or not _confirmed_detach(
                    tail, attached["program_id"]):
                raise VmCgroupEgressError("verified Aya detach event not recorded")
            after_a = egress.probe_connect(cgroup, port)
            if require_vm_running(proc, vm_id, cgroup) != vm_inode:
                raise VmCgroupEgressError("VM process stopped before final proof")

        report = evaluate_vm_host_probe(
            before_a=before_a, before_b=before_b, attached=True,
            during_a=during_a, during_b=during_b, after_a=after_a)
        report.update({
            "schema_version": 1,
            "observed_at_utc": datetime.now(timezone.utc).isoformat(),
            "scope": "host-process cgroup only; 127.0.0.1 loopback; not guest network",
            "vm_id": vm_id,
            "firecracker_pid": proc.pid,
            "vm_cgroup": cgroup.name,
            "vm_cgroup_inode": vm_inode,
            "host_boot_id": host["boot_id"],
            "kernel_bpf_program_id": attached["program_id"],
            "aya_loader_sha256": loader_sha,
            "bpf_elf_sha256": program_sha,
            "attachment_interval_seconds": seconds,
            "aya_detached": True,
        })
        return report
    except egress.AcceptanceError as exc:
        raise VmCgroupEgressError(str(exc)) from exc
    finally:
        if aya is not None and aya.poll() is None:
            aya.kill()
            aya.communicate(timeout=3)
        if created:
            try:
                peer.rmdir()
            except OSError as exc:
                raise VmCgroupEgressError("peer test cgroup cleanup incomplete") from exc
