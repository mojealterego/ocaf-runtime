#!/usr/bin/env python3
"""Capture and compare two cold boots of two operator-owned KVM hosts.

This checks *reported* identity and real-soak receipts. It cannot authenticate
remote hosts or prove inter-host traffic; the owner must sign reviewed evidence.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import socket


class BundleDenied(ValueError):
    pass


UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\Z")
HEX = re.compile(r"[0-9a-f]{64}\Z")


def _value(path: Path) -> str:
    value = path.read_text().strip().lower()
    if not value or len(value) > 128:
        raise BundleDenied(f"empty or excessive host identity: {path}")
    return value


def validate_soak(soak: dict, tenant: str, boot: str) -> None:
    cycles = soak.get("cycles") if isinstance(soak, dict) else None
    if (not isinstance(soak, dict)
            or soak.get("status") != "ONE_HOST_100_REAL_KVM_JAILED_VM_SIGKILL_CYCLES_PASS"
            or soak.get("tenant_id") != tenant or soak.get("host_boot_id") != boot
            or soak.get("cycle_count") != 100 or soak.get("distinct_vm_ids") != 100
            or soak.get("all_running_rootfs_fd_inodes_pinned") is not True
            or soak.get("all_vm_cgroups_drained_after_sigkill") is not True
            or soak.get("durable_host_identity_proved") is not False
            or not isinstance(soak.get("receipt_chain_sha256"), str)
            or not HEX.fullmatch(soak["receipt_chain_sha256"])
            or not isinstance(cycles, list) or len(cycles) != 100):
        raise BundleDenied("100-cycle jailed KVM soak proof missing")
    seen = set()
    chain = bytes(32)
    for index, cycle in enumerate(cycles, 1):
        if (not isinstance(cycle, dict) or cycle.get("cycle") != index
                or not isinstance(cycle.get("vm_id"), str) or not cycle["vm_id"]
                or cycle["vm_id"] in seen
                or not isinstance(cycle.get("vm_report_sha256"), str)
                or not HEX.fullmatch(cycle["vm_report_sha256"])
                or type(cycle.get("rootfs_fd_inode")) is not int
                or cycle["rootfs_fd_inode"] <= 0
                or cycle.get("cgroup_drained_after_sigkill") is not True):
            raise BundleDenied("malformed or repeated KVM cycle receipt")
        seen.add(cycle["vm_id"])
        chain = hashlib.sha256(chain + bytes.fromhex(cycle["vm_report_sha256"])).digest()
    if chain.hex() != soak["receipt_chain_sha256"]:
        raise BundleDenied("KVM soak receipt hash chain changed")


def capture(tenant: str, soak_path: Path) -> dict:
    if os.uname().sysname != "Linux" or tenant not in ("tenant-a", "tenant-b"):
        raise BundleDenied("Linux tenant host required")
    boot = _value(Path("/proc/sys/kernel/random/boot_id"))
    machine = _value(Path("/etc/machine-id"))
    product = _value(Path("/sys/class/dmi/id/product_uuid"))
    if (not UUID.fullmatch(boot) or not UUID.fullmatch(product)
            or not re.fullmatch(r"[0-9a-f]{32}", machine)
            or product == "00000000-0000-0000-0000-000000000000"):
        raise BundleDenied("host boot, machine or DMI product identity invalid")
    raw = soak_path.read_bytes()
    soak = json.loads(raw)
    validate_soak(soak, tenant, boot)
    return {
        "schema_version": 1,
        "status": "HOST_COLD_BOOT_SOAK_RECEIPT_CAPTURED",
        "tenant_id": tenant,
        "hostname": socket.gethostname(),
        "machine_id": machine,
        "dmi_product_uuid": product,
        "host_boot_id": boot,
        "soak_sha256": hashlib.sha256(raw).hexdigest(),
        "soak": soak,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "host_identity_cryptographically_attested": False,
        "interhost_partition_tested": False,
        "stage_039_complete": False,
        "production_approved": False,
    }


def compare(reports: list[dict]) -> dict:
    if len(reports) != 4:
        raise BundleDenied("two host reports before and after cold reboot required")
    for r in reports:
        if (not isinstance(r, dict) or r.get("schema_version") != 1
                or r.get("status") != "HOST_COLD_BOOT_SOAK_RECEIPT_CAPTURED"
                or r.get("host_identity_cryptographically_attested") is not False
                or r.get("interhost_partition_tested") is not False
                or r.get("stage_039_complete") is not False
                or r.get("production_approved") is not False):
            raise BundleDenied("invalid host capture")
        for key, pattern in (("machine_id", r"[0-9a-f]{32}"),
                             ("dmi_product_uuid", UUID), ("host_boot_id", UUID),
                             ("soak_sha256", HEX)):
            value = r.get(key)
            if not isinstance(value, str) or re.fullmatch(pattern, value) is None:
                raise BundleDenied("host identity or digest missing: " + key)
        validate_soak(r.get("soak"), r.get("tenant_id"), r["host_boot_id"])
        raw = json.dumps(r["soak"], sort_keys=True, indent=2).encode() + b"\n"
        if hashlib.sha256(raw).hexdigest() != r["soak_sha256"]:
            raise BundleDenied("embedded soak differs from captured file")
    a0, b0, a1, b1 = reports
    if [r["tenant_id"] for r in reports] != ["tenant-a", "tenant-b", "tenant-a", "tenant-b"]:
        raise BundleDenied("tenant/boot report order invalid")
    for early, late in ((a0, a1), (b0, b1)):
        if (early["machine_id"] != late["machine_id"]
                or early["dmi_product_uuid"] != late["dmi_product_uuid"]
                or early["host_boot_id"] == late["host_boot_id"]):
            raise BundleDenied("same durable host across distinct cold boots not shown")
    if (a0["machine_id"] == b0["machine_id"]
            or a0["dmi_product_uuid"] == b0["dmi_product_uuid"]
            or len({r["host_boot_id"] for r in reports}) != 4):
        raise BundleDenied("two distinct hosts and four distinct boots not shown")
    return {
        "schema_version": 1,
        "status": "TWO_REPORTED_STABLE_HOST_IDENTITIES_FOUR_KVM_BOOTS_400_CYCLES_VERIFIED",
        "two_distinct_machine_ids": True,
        "two_distinct_dmi_product_uuids": True,
        "four_distinct_kernel_boot_ids": True,
        "cycles_per_host_per_boot": 100,
        "total_jail_vm_cycles": 400,
        "reports_sha256": [hashlib.sha256(json.dumps(r, sort_keys=True).encode()).hexdigest()
                           for r in reports],
        "host_identity_cryptographically_attested": False,
        "interhost_partition_tested": False,
        "stage_039_complete": False,
        "production_approved": False,
    }


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="mode", required=True)
    c = sub.add_parser("capture")
    c.add_argument("--tenant", required=True)
    c.add_argument("--soak", required=True, type=Path)
    c.add_argument("--report", required=True, type=Path)
    v = sub.add_parser("compare")
    v.add_argument("--a-before", required=True, type=Path)
    v.add_argument("--b-before", required=True, type=Path)
    v.add_argument("--a-after", required=True, type=Path)
    v.add_argument("--b-after", required=True, type=Path)
    v.add_argument("--report", required=True, type=Path)
    x = p.parse_args()
    try:
        if x.report.exists():
            raise BundleDenied("report path already exists")
        result = (capture(x.tenant, x.soak) if x.mode == "capture" else
                  compare([json.loads(path.read_bytes()) for path in
                           (x.a_before, x.b_before, x.a_after, x.b_after)]))
        x.report.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
    except (OSError, ValueError) as exc:
        p.error(str(exc))
    print(result["status"])


if __name__ == "__main__":
    main()
