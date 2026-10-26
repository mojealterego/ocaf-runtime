#!/usr/bin/env python3
"""Real jailed KVM guest attempt to read a VMM-host-only canary file.

This is one filesystem boundary negative, not proof that VM escape is impossible.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import tempfile
import time

import stage039_guest_network as guest
import stage039_live_firecracker as fc
import stage039_microvm_gate as gate
import stage039_vm_crash as vm_crash
import stage039_vm_resources as vm_resources


class CanaryDenied(ValueError):
    pass


def require_guest_marker(serial: str, nonce: str) -> None:
    if not isinstance(serial, str) or re.fullmatch(r"[0-9a-f]{32}", nonce) is None:
        raise CanaryDenied("invalid guest canary receipt")
    expected = "OCAF039_GUEST_HOST_CANARY_DENIED:" + nonce
    if expected not in serial.splitlines():
        raise CanaryDenied("missing exact guest-originated canary denial marker")
    if "OCAF039_GUEST_HOST_CANARY_FAIL:" + nonce in serial:
        raise CanaryDenied("guest observed forbidden host canary")


def execute(*, firecracker: Path, firecracker_sha: str, jailer: Path,
            jailer_sha: str, jailer_base: Path, kernel: Path, kernel_sha: str,
            rootfs: Path, rootfs_sha: str, probe_source: Path,
            report: Path) -> dict:
    if os.geteuid() != 0 or os.uname().machine != "x86_64":
        raise CanaryDenied("root on actual x86_64 KVM host required")
    fc._verified_binary(firecracker, firecracker_sha)
    fc._verified_binary(jailer, jailer_sha)
    if fc._matching_release(jailer, firecracker) != "1.17.0":
        raise CanaryDenied("pinned Firecracker and jailer release 1.17.0 required")
    fc._private_base(jailer_base)
    guest._check_pinned(kernel, kernel_sha)
    guest._check_pinned(rootfs, rootfs_sha)
    if (not probe_source.is_absolute() or probe_source.is_symlink()
            or not probe_source.is_file()):
        raise CanaryDenied("checked-in guest probe source required")
    nonce = secrets.token_hex(16)
    canary_name = "ocaf039-host-canary-" + nonce
    vm_id = "ocaf-" + secrets.token_hex(12)
    cgroup_name = "ocaf-stage039-vm-" + secrets.token_hex(8) + ".service"
    cgroup = Path("/sys/fs/cgroup") / cgroup_name
    root = jailer_base / firecracker.name / vm_id / "root"
    socket_path = fc.validate_api_socket_path(jailer_base, firecracker.name, vm_id)
    lab = Path(tempfile.mkdtemp(prefix="ocaf039-canary-", dir="/tmp"))
    proc = None
    try:
        guest_init = lab / "canary-init"
        subprocess.run([
            "gcc", "-static", "-Os", "-s", "-Wall", "-Wextra", "-Werror",
            "-DCANARY_NAME=\"" + canary_name + "\"",
            "-DPROBE_NONCE=\"" + nonce + "\"",
            str(probe_source), "-o", str(guest_init),
        ], check=True, capture_output=True, timeout=60)
        image = lab / "guest.ext4"
        image_sha = guest._stage_guest_rootfs(rootfs, image, guest_init)
        image.chmod(0o400)
        cgroup.mkdir(mode=0o700)
        resource = vm_resources.apply_resource_limits(cgroup, 256)
        fc._secure_staging_directories(root)
        fc._copy_pinned(kernel, root / "guest/vmlinux.bin", kernel_sha)
        fc._copy_pinned(image, root / "guest/rootfs.ext4", image_sha)
        canary = root / canary_name
        canary_bytes = secrets.token_bytes(64)
        with canary.open("xb") as stream:
            stream.write(canary_bytes)
            stream.flush()
            os.fsync(stream.fileno())
        canary.chmod(0o444)
        canary_sha = hashlib.sha256(canary_bytes).hexdigest()
        config = {
            "boot-source": {"kernel_image_path": "/guest/vmlinux.bin",
                            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off root=/dev/vda ro init=/ocaf_probe_init"},
            "drives": [{"drive_id": "rootfs", "path_on_host": "/guest/rootfs.ext4",
                        "is_root_device": True, "is_read_only": True}],
            "machine-config": {"vcpu_count": 1, "mem_size_mib": 256, "smt": False},
        }
        config_path = root / "vmconfig.json"
        with os.fdopen(os.open(config_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o444),
                       "w", encoding="utf-8") as stream:
            json.dump(config, stream, separators=(",", ":"))
            stream.flush()
            os.fsync(stream.fileno())
            os.fchmod(stream.fileno(), 0o444)
        log = root.parent / "serial-console.log"
        outfd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        try:
            proc = subprocess.Popen(fc.build_jailer_command(
                jailer, firecracker, vm_id, 1002, 1002, jailer_base, cgroup_name),
                stdout=outfd, stderr=subprocess.STDOUT, close_fds=True)
        finally:
            os.close(outfd)
        deadline = time.monotonic() + 45
        observed = None
        while time.monotonic() < deadline and proc.poll() is None:
            serial = fc._bounded_console_evidence(log)
            if "OCAF039_GUEST_HOST_CANARY_" in serial:
                require_guest_marker(serial, nonce)
                current = Path(f"/proc/{proc.pid}/cgroup").read_text()
                observed = fc.validate_running(fc._api_state(socket_path), serial,
                                               current, vm_id, cgroup_name)
                observed["running_rootfs_fd"] = fc.attest_running_rootfs_fd(
                    proc, root / "guest/rootfs.ext4", image_sha)
                break
            time.sleep(0.15)
        if observed is None:
            # A failed guest start must expose enough bounded context to tell
            # an early jailer exit from a booted guest that rejected the probe.
            tail = fc._bounded_console_evidence(log)[-600:]
            raise CanaryDenied(
                "real guest canary result not observed before deadline; "
                f"jailer_exit={proc.poll()!r}; console_tail={tail!r}")
        if hashlib.sha256(canary.read_bytes()).hexdigest() != canary_sha:
            raise CanaryDenied("host canary changed while guest was running")
        killed = vm_crash.kill_and_verify(proc, vm_id, cgroup)
        if hashlib.sha256(canary.read_bytes()).hexdigest() != canary_sha:
            raise CanaryDenied("host canary changed before VM cgroup drain")
        result = {
            "schema_version": 1,
            "status": "JAILED_GUEST_HOST_ONLY_CANARY_ACCESS_DENIED",
            "host_boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "vm_id": vm_id, "guest_probe_nonce": nonce,
            "guest_probe_sha256": hashlib.sha256(guest_init.read_bytes()).hexdigest(),
            "rootfs_sha256": image_sha, "host_canary_sha256": canary_sha,
            "guest_direct_and_proc_root_open_denied": True,
            "host_canary_unchanged": True, "guest_marker_observed": True,
            "live_vm": observed, "vm_sigkill": killed, "resource_limits": resource,
            "guest_escape_proven_impossible": False,
            "stage_039_complete": False, "production_approved": False,
        }
        gate.save_report(report, result)
        return result
    finally:
        if proc is not None and proc.poll() is None:
            proc.kill()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
        try:
            cgroup.rmdir()
        except OSError:
            pass
        shutil.rmtree(lab, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("firecracker", "jailer", "jailer-base", "kernel", "rootfs",
                 "probe-source", "report"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ("firecracker-sha256", "jailer-sha256", "kernel-sha256",
                 "rootfs-sha256"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--ack-real-jailed-guest-canary-probe", action="store_true")
    args = parser.parse_args()
    if not args.ack_real_jailed_guest_canary_probe:
        parser.error("explicit real jailed KVM guest negative probe acknowledgement required")
    try:
        result = execute(firecracker=args.firecracker,
                         firecracker_sha=args.firecracker_sha256,
                         jailer=args.jailer, jailer_sha=args.jailer_sha256,
                         jailer_base=args.jailer_base, kernel=args.kernel,
                         kernel_sha=args.kernel_sha256, rootfs=args.rootfs,
                         rootfs_sha=args.rootfs_sha256,
                         probe_source=args.probe_source, report=args.report)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        parser.error(str(exc))
    print(result["status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
