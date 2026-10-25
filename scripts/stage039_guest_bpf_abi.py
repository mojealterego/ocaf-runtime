#!/usr/bin/env python3
"""Bounded guest-originated BTF/kprobe/syscall ABI observation in two KVM VMs.

The pinned Firecracker debug kernel currently has BTF and BPF syscalls but
does not enable KPROBES. This report must remain partial for that image.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re

import stage039_guest_wasi as guest

ABI = re.compile(rb"(?m)^OCAF039_GUEST_BPF_ABI_BTF=([01]) KPROBE=([01]) SYSCALL=1$")
PIN = re.compile(r"[0-9a-f]{64}\Z")


class GuestAbiDenied(ValueError):
    pass


def parse_guest_abi(serial: bytes) -> dict:
    if not isinstance(serial, bytes) or not 0 < len(serial) <= 1024 * 1024:
        raise GuestAbiDenied("bounded real guest serial required")
    normalized = serial.replace(b"\r\n", b"\n")
    matches = list(ABI.finditer(normalized))
    if (len(matches) != 1 or normalized.count(b"OCAF039_GUEST_BPF_ABI_") != 1
            or normalized.count(b"OCAF039_GUEST_BPF_SOCKET_FILTER_ID=") != 1
            or normalized.count(b"OCAF039_GUEST_OPENAT_EVENT_ID=") != 1
            or normalized.count(b"OCAF039_GUEST_PID1_BOOTED") != 1
            or matches[0].start() <= normalized.find(b"OCAF039_GUEST_PID1_BOOTED")
            or matches[0].start() >= normalized.find(b"OCAF039_GUEST_OPENAT_EVENT_ID=")):
        raise GuestAbiDenied("guest ABI marker missing, duplicated or out of order")
    return {"btf_file_observed_in_guest": matches[0].group(1) == b"1",
            "kprobe_events_observed_in_guest": matches[0].group(2) == b"1",
            "bpf_syscall_observed_in_guest": True,
            "serial_sha256": hashlib.sha256(serial).hexdigest()}


def config_flags(raw: bytes, pin: str) -> dict:
    if (not isinstance(raw, bytes) or not 0 < len(raw) <= 1024 * 1024
            or not isinstance(pin, str) or PIN.fullmatch(pin) is None
            or hashlib.sha256(raw).hexdigest() != pin):
        raise GuestAbiDenied("guest kernel config missing or not pinned")
    lines = set(raw.decode("ascii").splitlines())
    return {name: f"CONFIG_{name}=y" in lines for name in
            ("DEBUG_INFO_BTF", "KPROBES", "KPROBE_EVENTS", "BPF_SYSCALL")}


def build_report(broker: dict, jailer_base: Path, rootfs: dict[str, str],
                 config_raw: bytes, config_pin: str) -> dict:
    # Revalidate the actual broker-bound jailed guest receipts before using
    # the serial files. A detached text marker is never enough.
    verified = guest.verify_brokered_pair(broker, jailer_base, rootfs)
    flags = config_flags(config_raw, config_pin)
    observations = {}
    for tenant in ("a", "b"):
        receipt = verified["tenant_receipts"][tenant]
        vm_id = receipt["vm_id"]
        serial_path = jailer_base / "firecracker" / vm_id / "serial-console.log"
        if serial_path.is_symlink() or not serial_path.is_file():
            raise GuestAbiDenied("guest serial path changed")
        with serial_path.open("rb") as stream:
            serial = stream.read(1024 * 1024 + 1)
        observation = parse_guest_abi(serial)
        if (observation["serial_sha256"] != receipt["serial_sha256"]
                or receipt.get("guest_openat_bpf_program_id", 0) <= 0
                or receipt.get("guest_openat_event_count", 0) <= 0):
            raise GuestAbiDenied("ABI observation not bound to real guest Aya event")
        if (observation["btf_file_observed_in_guest"] and not flags["DEBUG_INFO_BTF"]
                or observation["kprobe_events_observed_in_guest"]
                and not flags["KPROBES"]):
            raise GuestAbiDenied("guest ABI observation contradicts pinned kernel config")
        observations[tenant] = observation
    if not flags["BPF_SYSCALL"]:
        raise GuestAbiDenied("pinned guest kernel lacks BPF syscall")
    full = all(o["btf_file_observed_in_guest"] and
               o["kprobe_events_observed_in_guest"] for o in observations.values())
    return {"schema_version": 1,
            "status": ("GUEST_BPF_ABI_ENDPOINTS_OBSERVED_KPROBE_LOAD_UNTESTED" if full else
                       "GUEST_BPF_ABI_PARTIAL_KPROBE_OR_BTF_UNAVAILABLE"),
            "kernel_config_sha256": config_pin, "kernel_config_flags": flags,
            "two_jailed_guest_kernel_abi_observations": observations,
            "guest_kprobe_program_loaded": False,
            "stage_039_22_complete": False, "stage_039_complete": False,
            "production_approved": False}


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--broker-report", type=Path, required=True)
    p.add_argument("--jailer-base", type=Path, required=True)
    p.add_argument("--rootfs-sha256-a", required=True)
    p.add_argument("--rootfs-sha256-b", required=True)
    p.add_argument("--kernel-config", type=Path, required=True)
    p.add_argument("--kernel-config-sha256", required=True)
    p.add_argument("--report", type=Path, required=True)
    x = p.parse_args()
    try:
        report = build_report(json.loads(x.broker_report.read_bytes()), x.jailer_base,
                              {"a": x.rootfs_sha256_a, "b": x.rootfs_sha256_b},
                              x.kernel_config.read_bytes(), x.kernel_config_sha256)
        with x.report.open("x") as stream:
            json.dump(report, stream, sort_keys=True, indent=2)
            stream.write("\n")
    except (OSError, ValueError, UnicodeError) as exc:
        p.error(str(exc))
    print(report["status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
