#!/usr/bin/env python3
"""OCAF 0.39.4: exercise the KVM API and create/close a real VM file descriptor.

This verifies that a Linux host can create a KVM VM kernel object via ioctl.
It does NOT start Firecracker, boot a guest, attach Aya/eBPF, establish
isolation or approve production. It does not incur a cloud provisioning cost.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import stat
import sys

import stage039_microvm_gate as gate

KVM_GET_API_VERSION = 0xAE00
KVM_CREATE_VM = 0xAE01
KVM_API_VERSION = 12
KVM_CHAR_MAJOR = 10
KVM_CHAR_MINOR = 232


def try_create_kvm_vm() -> dict:
    """Prove KVM_GET_API_VERSION and KVM_CREATE_VM using real Linux ioctls.

    Close every returned kernel VM fd immediately; this is not a guest.
    No fake positive result may arise from detecting a device node alone.
    """
    result: dict[str, object] = {
        "api_version_verified": False,
        "kvm_api_version": None,
        "kvm_vm_fd_created": False,
        "guest_booted": False,
        "firecracker_booted": False,
        "ebpf_attached": False,
    }
    fd = None
    vmfd = None
    try:
        st = os.lstat("/dev/kvm")
        if (not stat.S_ISCHR(st.st_mode) or
                os.major(st.st_rdev) != KVM_CHAR_MAJOR or
                os.minor(st.st_rdev) != KVM_CHAR_MINOR or
                st.st_mode & 0o002):
            return result
        fd = os.open("/dev/kvm",
                     os.O_RDWR | getattr(os, "O_NOFOLLOW", 0) |
                     getattr(os, "O_CLOEXEC", 0))
        version = fcntl.ioctl(fd, KVM_GET_API_VERSION, 0)
        if type(version) is not int:
            return result
        result["kvm_api_version"] = version
        if version != KVM_API_VERSION:
            return result
        result["api_version_verified"] = True
        vmfd = fcntl.ioctl(fd, KVM_CREATE_VM, 0)
        if type(vmfd) is not int or vmfd < 0:
            return result
        result["kvm_vm_fd_created"] = True
        return result
    except OSError:
        return result
    finally:
        if vmfd is not None and type(vmfd) is int and vmfd >= 0:
            os.close(vmfd)
        if fd is not None:
            os.close(fd)


def collect() -> dict:
    observed = gate.host_observations()
    if (not isinstance(observed, dict) or set(observed) != gate.HOST_CHECKS
            or any(type(v) is not bool for v in observed.values())):
        raise ValueError("incomplete or invalid factual Linux host observations")
    # Only run KVM_CREATE_VM after the kernel device and access probes pass.
    if all(observed[k] for k in ("linux_x86_64", "kvm_device_character",
                                  "kvm_device_accessible")):
        verified = try_create_kvm_vm()
    else:
        verified = {
            "api_version_verified": False, "kvm_api_version": None,
            "kvm_vm_fd_created": False, "guest_booted": False,
            "firecracker_booted": False, "ebpf_attached": False,
        }
    positive = all(observed.values()) and verified["kvm_vm_fd_created"] is True
    return {
        "schema_version": 1,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "purpose": "Factual KVM kernel VM file descriptor capability, no guest boot",
        "checks": observed,
        **verified,
        "status": "KVM_IOCTL_VM_CREATED_NOT_BOOTED" if positive else "NO_GO",
        "microvm_launched": False,
        "stage_039_complete": False,
        "production_approved": False,
        "limitations": (
            "A successfully opened KVM VM fd has not run a Firecracker jailer, "
            "booted a kernel, executed any guest, attached Aya or enforced "
            "any eBPF policy. This report is not production acceptance."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True, type=Path,
                        help="new, create-once private JSON report")
    args = parser.parse_args()
    try:
        report = collect()
        gate.save_report(args.report, report)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status": report["status"],
                      "kvm_vm_fd_created": report["kvm_vm_fd_created"],
                      "microvm_launched": False,
                      "ebpf_attached": False,
                      "report": str(args.report)}))
    return 0 if report["kvm_vm_fd_created"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
