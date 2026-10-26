#!/usr/bin/env python3
"""Bind guest kprobe counts to two brokered jailed KVM WASI/Aya receipts."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re

import stage039_guest_wasi as guest

MARKER = re.compile(rb"(?m)^OCAF039_GUEST_KPROBE_INTEGRATED_ID=([1-9][0-9]*) "
                    rb"COUNT=([1-9][0-9]*) NONCE=([0-9a-f]{32})$")
HEX = re.compile(r"[0-9a-f]{64}\Z")


class KprobePairDenied(ValueError):
    pass


def parse_marker(serial: bytes, nonce: str) -> dict:
    if not isinstance(serial, bytes) or not 0 < len(serial) <= 1024 * 1024:
        raise KprobePairDenied("bounded guest serial required")
    normalized = serial.replace(b"\r\n", b"\n")
    hits = list(MARKER.finditer(normalized))
    if (len(hits) != 1 or normalized.count(b"OCAF039_GUEST_KPROBE_INTEGRATED_ID=") != 1
            or b"OCAF039_GUEST_KPROBE_INTEGRATION_FAIL" in normalized
            or hits[0].group(3).decode() != nonce
            or not normalized.find(b"OCAF039_GUEST_BPF_ABI_") < hits[0].start()
            < normalized.find(b"OCAF039_GUEST_OPENAT_EVENT_ID=")):
        raise KprobePairDenied("guest kprobe marker absent, duplicated, wrong tenant or order")
    return {"guest_kprobe_program_id": int(hits[0].group(1)),
            "guest_kprobe_event_count": int(hits[0].group(2)),
            "serial_sha256": hashlib.sha256(serial).hexdigest()}


def verify_pair(broker: dict, jailer_base: Path, rootfs: dict[str, str],
                kernel_sha256: str, config_raw: bytes, config_sha256: str) -> dict:
    if (not isinstance(kernel_sha256, str) or HEX.fullmatch(kernel_sha256) is None
            or not isinstance(config_sha256, str) or HEX.fullmatch(config_sha256) is None
            or hashlib.sha256(config_raw).hexdigest() != config_sha256):
        raise KprobePairDenied("unverified custom kernel/config input")
    flags = set(config_raw.decode("ascii").splitlines())
    required = {f"CONFIG_{name}=y" for name in
                ("MODULES", "KPROBES", "KPROBE_EVENTS", "DEBUG_INFO_BTF",
                 "BPF_EVENTS", "PERF_EVENTS", "BPF_SYSCALL")}
    if not required <= flags:
        raise KprobePairDenied("custom kernel lacks required kprobe/BTF options")
    verified = guest.verify_brokered_pair(broker, jailer_base, rootfs)
    observations = {}
    for tenant in ("a", "b"):
        vm = broker["tenant_" + tenant]
        if vm.get("guest_kernel_sha256") != kernel_sha256:
            raise KprobePairDenied("brokered VM used an unexpected kernel digest")
        receipt = verified["tenant_receipts"][tenant]
        path = jailer_base / "firecracker" / receipt["vm_id"] / "serial-console.log"
        if path.is_symlink() or not path.is_file():
            raise KprobePairDenied("redirected or missing jailed guest serial")
        with path.open("rb") as stream:
            raw = stream.read(1024 * 1024 + 1)
        probe = parse_marker(raw, receipt["task_nonce"])
        host_aya = vm.get("vm_preboot_aya_egress", {})
        if (probe["serial_sha256"] != receipt["serial_sha256"]
                or receipt["guest_openat_bpf_program_id"] <= 0
                or receipt["guest_openat_event_count"] <= 0
                or host_aya.get("guest_wasi_observed_while_attached") is not True
                or host_aya.get("aya_detached_after_vm_stop") is not True):
            raise KprobePairDenied("kprobe lacks guest Aya, host Aya or serial binding")
        observations[tenant] = {**probe, "vm_id": receipt["vm_id"],
                                "task_nonce": receipt["task_nonce"],
                                "guest_aya_openat_program_id": receipt["guest_openat_bpf_program_id"],
                                "guest_aya_openat_event_count": receipt["guest_openat_event_count"],
                                "traceparent": receipt["traceparent"]}
    if (observations["a"]["vm_id"] == observations["b"]["vm_id"]
            or observations["a"]["task_nonce"] == observations["b"]["task_nonce"]
            or observations["a"]["traceparent"] == observations["b"]["traceparent"]):
        raise KprobePairDenied("two independently bound tenants required")
    return {"schema_version": 1,
            "status": "TWO_BROKERED_JAILED_KVM_GUEST_KPROBE_AYA_WASI_LAB_PASS",
            "kernel_sha256": kernel_sha256, "kernel_config_sha256": config_sha256,
            "tenants": observations, "host_aya_and_guest_aya_correlated": True,
            "two_independent_hosts": False, "durable_kernel_provenance": False,
            "stage_039_22_complete": False, "stage_039_complete": False,
            "production_approved": False}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--broker-report", type=Path, required=True)
    parser.add_argument("--jailer-base", type=Path, required=True)
    parser.add_argument("--rootfs-sha256-a", required=True)
    parser.add_argument("--rootfs-sha256-b", required=True)
    parser.add_argument("--kernel-sha256", required=True)
    parser.add_argument("--kernel-config", type=Path, required=True)
    parser.add_argument("--kernel-config-sha256", required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    try:
        proof = verify_pair(json.loads(args.broker_report.read_bytes()), args.jailer_base,
                            {"a": args.rootfs_sha256_a, "b": args.rootfs_sha256_b},
                            args.kernel_sha256, args.kernel_config.read_bytes(),
                            args.kernel_config_sha256)
        with args.report.open("x") as stream:
            json.dump(proof, stream, sort_keys=True, indent=2)
            stream.write("\n")
    except (OSError, ValueError, UnicodeError) as error:
        parser.error(str(error))
    print(proof["status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
