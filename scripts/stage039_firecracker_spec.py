#!/usr/bin/env python3
"""OCAF 0.39.5: verify staged jailer artifacts and generate a netless VM config.

This code does NOT start Firecracker, attach Aya/eBPF or invoke jailer. A
verified configuration is not proof the reviewed inodes were the ones run.
Those properties require distinct live host and signed execution evidence.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import stat

import stage039_microvm_gate as gate

GUEST_KERNEL = "/guest/vmlinux.bin"
GUEST_ROOTFS = "/guest/rootfs.ext4"


def enable_guest_entropy_for_wasi(config:dict)->dict:
    """Explicit Firecracker virtio-rng for guest getrandom() initialization.

    The pinned Firecracker v1.17 API supports an 'entropy' device with an
    optional rate limiter. Fail closed on any networked/invalid config.
    This is NOT a claim that a particular guest Linux kernel has the
    CONFIG_HW_RANDOM_VIRTIO frontend; actual guest execution must prove it.
    """
    if (not isinstance(config,dict) or config.get("network-interfaces")!=[]
        or not isinstance(config.get("machine-config"),dict)
        or type(config["machine-config"].get("vcpu_count")) is not int
        or config["machine-config"]["vcpu_count"]<1
        or config.get("entropy") is not None):
        raise ValueError("cannot attach virtio-rng to unreviewed networked guest configuration")
    return {**config,"entropy":{}}


def build(manifest_raw: bytes, approved_root: Path, jailer_root: Path) -> dict:
    manifest = gate.parse_manifest(manifest_raw)
    src = gate.inspect_manifest(manifest_raw, Path(approved_root))
    if src["status"] != "STATIC_ARTIFACTS_VERIFIED_ONLY":
        raise ValueError("source pinned artifacts not verified")
    jail = Path(jailer_root)
    if not jail.is_absolute() or jail.is_symlink() or ".." in jail.parts:
        raise ValueError("untrusted jailer root")
    info = jail.stat()
    if (not stat.S_ISDIR(info.st_mode) or info.st_mode & 0o022 or
            info.st_uid not in {0, os.geteuid()}):
        raise ValueError("mutable/unauthorized jailer root")
    # Both named files must physically be staged inside the jail root,
    # and are hashed from a nofollow FD with private parent directories.
    digests = {}
    for asset, guest_path in (("guest_kernel", GUEST_KERNEL),
                               ("guest_rootfs", GUEST_ROOTFS)):
        path = jail / guest_path.lstrip("/")
        digest = gate._artifact_digest(
            str(path), jail, gate.ASSET_MAXIMUM_BYTES[asset], False)
        if digest != manifest["assets"][asset]["sha256"]:
            raise ValueError("jailed artifact diverges from pinned source: " + asset)
        digests[asset] = digest
    policy = manifest["isolation"]
    config = {
        "boot-source": {
            "kernel_image_path": GUEST_KERNEL,
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off root=/dev/vda ro",
        },
        "drives": [{
            "drive_id": "rootfs",
            "path_on_host": GUEST_ROOTFS,
            "is_root_device": True,
            "is_read_only": True,
        }],
        "machine-config": {
            "vcpu_count": policy["vcpus"],
            "mem_size_mib": policy["mem_mib"],
            "smt": False,
        },
        "network-interfaces": [],
    }
    return {
        "schema_version": 1,
        "status": "JAILED_STATIC_CONFIG_VERIFIED_NOT_BOOTED",
        "firecracker_config": config,
        "staged_sha256": digests,
        "workload_id": manifest["workload_id"],
        "tenant_id": manifest["tenant_id"],
        "agent_id": manifest["agent_id"],
        "firecracker_booted": False,
        "aya_attached": False,
        "production_approved": False,
        "stage_039_complete": False,
        "jailer_executed": False,
        "limitations": (
            "Artifacts have been checked, but no jailer or VM started. "
            "TOCTOU is still possible unless the runtime binds to the "
            "reviewed staged inodes; this command is not guest attestation."
        ),
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--manifest", type=Path, required=True)
    ap.add_argument("--approved-root", type=Path, required=True)
    ap.add_argument("--jailer-root", type=Path, required=True)
    ap.add_argument("--report", type=Path, required=True)
    args = ap.parse_args()
    try:
        result = build(args.manifest.read_bytes(), args.approved_root, args.jailer_root)
        gate.save_report(args.report, result)
    except (OSError, ValueError) as exc:
        ap.error(str(exc))
    print(json.dumps({"status": result["status"], "report": str(args.report),
                      "firecracker_booted": False, "aya_attached": False}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
