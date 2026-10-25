#!/usr/bin/env python3
"""Bounded 100-cycle jailed Firecracker/KVM SIGKILL soak on one host.

This runner records a chain of actual VM reports. It cannot certify durable
host ownership, interhost RPC, signer independence or operator acceptance.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys

import stage039_live_firecracker as live
import stage039_microvm_gate as gate


class SoakDenied(ValueError):
    pass


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify_cycle(report: dict, *, tenant: str, host_boot_id: str | None,
                 rootfs_sha256: str) -> tuple[str, str]:
    if not isinstance(report, dict) or report.get("tenant_id") != tenant:
        raise SoakDenied("soak VM tenant mismatch")
    boot = report.get("host_boot_id")
    vm_id = report.get("vm_id")
    crash = report.get("vm_sigkill_fault")
    fd = report.get("running_rootfs_fd")
    if (not isinstance(boot, str) or len(boot) < 16
            or host_boot_id is not None and boot != host_boot_id
            or not isinstance(vm_id, str) or live.VALID_VM.fullmatch(vm_id) is None
            or report.get("status") != "JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED"
            or report.get("firecracker_api_running") is not True
            or report.get("guest_kernel_console_seen") is not True
            or not isinstance(fd, dict)
            or fd.get("status") != "RUNNING_VMM_ROOTFS_FD_MATCHES_PINNED_INODE"
            or fd.get("rootfs_sha256") != rootfs_sha256
            or type(fd.get("matching_fd_count")) is not int
            or fd["matching_fd_count"] < 1
            or not isinstance(crash, dict)
            or crash.get("status") != "VM_SIGKILL_CGROUP_SUBTREE_DRAINED"
            or crash.get("cgroup_subtree_empty") is not True
            or crash.get("sigkill_exit_code") != -9
            or report.get("stage_039_complete") is not False
            or report.get("production_approved") is not False):
        raise SoakDenied("soak VM kernel, rootfs FD or crash proof absent")
    return boot, vm_id


def run(prefix: Path, report_path: Path, *, tenant: str, uid: int,
        cycles: int = 100) -> dict:
    if (os.geteuid() != 0 or cycles != 100 or tenant not in ("tenant-a", "tenant-b")
            or uid not in (1002, 1003) or not prefix.is_absolute()
            or not prefix.name.startswith("o39h") or prefix.is_symlink()
            or not prefix.is_dir() or report_path.exists()):
        raise SoakDenied("fresh privileged 100-cycle KVM host scope required")
    approved = prefix / "approved"
    if approved.is_symlink() or approved.stat().st_uid != 0 \
            or approved.stat().st_mode & 0o077:
        raise SoakDenied("private root-owned image registry required")
    manifest_path = approved / "request.json"
    raw = manifest_path.read_bytes()
    manifest = gate.parse_manifest(raw)
    gate.inspect_manifest(raw, approved)
    if manifest["tenant_id"] != tenant:
        raise SoakDenied("soak tenant differs from pinned manifest")
    rootfs_sha = manifest["assets"]["guest_rootfs"]["sha256"]
    fc = approved / "firecracker"
    jailer = approved / "jailer"
    seen: set[str] = set()
    host_boot_id = None
    chain = bytes(32)
    summaries = []
    for sequence in range(1, cycles + 1):
        proof = live.run(
            manifest=manifest_path, approved_root=approved,
            jailer=jailer, jailer_sha=digest(jailer),
            firecracker=fc, firecracker_sha=manifest["assets"]["firecracker"]["sha256"],
            chroot_base=prefix / "jailer", uid=uid, gid=uid,
            timeout=70, crash_after_proof=True)
        host_boot_id, vm_id = verify_cycle(
            proof, tenant=tenant, host_boot_id=host_boot_id,
            rootfs_sha256=rootfs_sha)
        if vm_id in seen:
            raise SoakDenied("VM identity repeated during 100-cycle soak")
        seen.add(vm_id)
        canonical = json.dumps(proof, sort_keys=True, separators=(",", ":")).encode()
        proof_sha = hashlib.sha256(canonical).hexdigest()
        chain = hashlib.sha256(chain + bytes.fromhex(proof_sha)).digest()
        summaries.append({"cycle": sequence, "vm_id": vm_id,
                          "vm_report_sha256": proof_sha,
                          "rootfs_fd_inode": proof["running_rootfs_fd"]["rootfs_inode"],
                          "cgroup_drained_after_sigkill": True})
        # The VM process and cgroup have been confirmed stopped. Keep the
        # bounded receipt, release multi-GB staging before the next cycle.
        vm_dir = prefix / "jailer" / "firecracker" / vm_id
        if vm_dir.is_symlink() or not vm_dir.is_dir():
            raise SoakDenied("VM staging directory changed before cleanup")
        shutil.rmtree(vm_dir)
    result = {
        "status": "ONE_HOST_100_REAL_KVM_JAILED_VM_SIGKILL_CYCLES_PASS",
        "tenant_id": tenant, "host_boot_id": host_boot_id,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "cycle_count": cycles, "distinct_vm_ids": len(seen),
        "all_running_rootfs_fd_inodes_pinned": True,
        "all_vm_cgroups_drained_after_sigkill": True,
        "receipt_chain_sha256": chain.hex(), "cycles": summaries,
        "durable_host_identity_proved": False,
        "interhost_network_partition_tested": False,
        "stage_039_complete": False, "production_approved": False,
    }
    report_path.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--tenant", required=True)
    parser.add_argument("--uid", required=True, type=int)
    parser.add_argument("--ack-real-kvm-100-cycle-soak", action="store_true")
    args = parser.parse_args()
    if not args.ack_real_kvm_100_cycle_soak:
        parser.error("explicit privileged KVM soak acknowledgment required")
    try:
        result = run(args.prefix, args.report, tenant=args.tenant, uid=args.uid)
        print(result["status"])
    except (OSError, ValueError) as exc:
        detail = f"{type(exc).__name__}: {exc}"
        detail = detail.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
        print(f"::error title=stage039-soak::{detail[:500]}", file=sys.stderr)
        parser.error(str(exc))


if __name__ == "__main__":
    main()
