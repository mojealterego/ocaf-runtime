#!/usr/bin/env python3
"""Stage 0.39.10: ACTUAL two simultaneous jailed KVM Firecracker microVMs.

Tenant A and B boot with separate identities, VM IDs, cgroup inodes and
unprivileged UIDs; a barrier requires both guest kernels to run at once.
Tenant A undergoes Aya host-cgroup test then SIGKILL. Tenant B must still
be alive AFTER A's SIGKILL, then itself runs its own Aya and SIGKILL cleanup.
This is ONE HOST with TWO concurrent real guests; not two-host failover
or guest-originated Ethernet/UDP isolation.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import threading

import stage039_live_firecracker as fc
import stage039_microvm_gate as gate
import stage039_vm_egress as egress
import stage039_traceparent as traceparent

IDENTITY = re.compile(r"[a-z0-9][a-z0-9_-]{0,63}\Z")


class DualTenantError(ValueError):
    pass


def validate_distinct_tenants(a: dict, b: dict) -> tuple[str, str]:
    if not isinstance(a, dict) or not isinstance(b, dict):
        raise DualTenantError("tenant manifest missing")
    for key in ("tenant_id", "agent_id", "workload_id"):
        va, vb = a.get(key), b.get(key)
        if (not isinstance(va, str) or not isinstance(vb, str)
                or IDENTITY.fullmatch(va) is None or IDENTITY.fullmatch(vb) is None
                or va == vb):
            raise DualTenantError("both tenants must use distinct bounded " + key)
    return a["tenant_id"], b["tenant_id"]


def validate_dual_evidence(a: dict, b: dict, overlapping: bool) -> dict:
    if overlapping is not True:
        raise DualTenantError("two real guests not simultaneously running")
    validate_distinct_tenants(
        {k:a.get(k) for k in ("tenant_id","agent_id","workload_id")},
        {k:b.get(k) for k in ("tenant_id","agent_id","workload_id")},
    )
    for key in ("vm_id", "cgroup_inode"):
        if a.get(key) == b.get(key) or a.get(key) is None or b.get(key) is None:
            raise DualTenantError("VM identifiers or kernel cgroups collided")
    if (not a.get("host_boot_id") or a["host_boot_id"] != b.get("host_boot_id")
            or any(report.get("status") !=
                   "JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED"
                   or report.get("firecracker_api_running") is not True
                   or report.get("guest_kernel_console_seen") is not True
                   or report.get("production_approved") is not False
                   for report in (a,b))):
        raise DualTenantError("simultaneous guest boot or host evidence invalid")
    return {
        "status": "TWO_CONCURRENT_JAILED_FIRECRACKER_GUESTS_OBSERVED",
        "two_guests_simultaneously_running": True,
        "separate_vm_ids": True,
        "separate_cgroup_inodes": True,
        "same_ephemeral_host": True,
        "multi_host_failover_tested": False,
        "guest_network_egress_tested": False,
        "production_approved": False,
    }


def validate_peer_survived_a_crash(a: dict, b_still_running: bool) -> dict:
    crash = a.get("vm_sigkill_fault") if isinstance(a,dict) else None
    if (not isinstance(crash,dict)
            or crash.get("status") != "VM_SIGKILL_CGROUP_SUBTREE_DRAINED"
            or crash.get("cgroup_subtree_empty") is not True
            or b_still_running is not True):
        raise DualTenantError("tenant B did not independently survive tenant A crash")
    return {
        "tenant_b_survived_tenant_a_sigkill": True,
        "host_failure_tested": False,
        "multi_host_failover_tested": False,
        "production_approved": False,
    }


def execute(*, manifest_a:Path, manifest_b:Path, approved_root:Path,
            firecracker:Path, firecracker_sha:str, jailer:Path,
            jailer_sha:str, chroot_base:Path,
            aya_loader:Path|None=None, aya_loader_sha:str|None=None,
            aya_program:Path|None=None, aya_program_sha:str|None=None,
            aya_seconds:int=15, timeout:int=90,
            require_guest_wasi:bool=False,
            traceparents:dict[str,str]|None=None) -> dict:
    ma = gate.parse_manifest(manifest_a.read_bytes())
    mb = gate.parse_manifest(manifest_b.read_bytes())
    validate_distinct_tenants(ma, mb)
    if ma["isolation"] != mb["isolation"]:
        raise DualTenantError("lab tenant isolation contracts diverge")
    for k in ("firecracker","guest_kernel","aya_cgroup_device"):
        if ma["assets"][k]["sha256"] != mb["assets"][k]["sha256"]:
            raise DualTenantError("different unreviewed pinned artifacts")
    if require_guest_wasi:
        if (not isinstance(traceparents,dict) or set(traceparents)!={"a","b"}
            or any(not traceparent.is_valid(x) for x in traceparents.values())
            or len(set(traceparents.values()))!=2):
            raise DualTenantError("two distinct broker-provided VM traceparents required")
        if ("wasi" not in ma or "wasi" not in mb
                or ma["wasi"]["task_nonce"] == mb["wasi"]["task_nonce"]
                or ma["wasi"]["module_sha256"] == mb["wasi"]["module_sha256"]
                or ma["assets"]["guest_rootfs"]["sha256"] == mb["assets"]["guest_rootfs"]["sha256"]):
            raise DualTenantError("WASI tenants require different nonces, modules and rootfs images")
    elif ma["assets"]["guest_rootfs"]["sha256"] != mb["assets"]["guest_rootfs"]["sha256"]:
        raise DualTenantError("different unreviewed pinned artifacts")
    for p in (manifest_a, manifest_b):
        if gate.inspect_manifest(p.read_bytes(), approved_root)["status"] != "STATIC_ARTIFACTS_VERIFIED_ONLY":
            raise DualTenantError("tenant artifacts failed preflight")

    barrier = threading.Barrier(2, timeout=60)
    killed_a = threading.Event()
    observed: dict[str, tuple] = {}
    guard = threading.Lock()
    overlap = {"passed":False, "peer_b_survived":False}

    def hook_a(result, proc, vm_id, cgroup):
        with guard:
            observed["a"] = (dict(result), proc, vm_id, cgroup)
        try:
            barrier.wait()
        except threading.BrokenBarrierError as exc:
            raise DualTenantError("tenant B did not join live KVM barrier") from exc
        with guard:
            x,y = observed["a"],observed["b"]
            va,vb = x[0],y[0]
            validate_dual_evidence(va,vb,True)
            if (egress.require_vm_running(x[1],x[2],x[3]) != va["cgroup_inode"]
                    or egress.require_vm_running(y[1],y[2],y[3]) != vb["cgroup_inode"]):
                raise DualTenantError("live kernel cgroup identity changed")
            overlap["passed"]=True

    def hook_b(result, proc, vm_id, cgroup):
        with guard:
            observed["b"] = (dict(result), proc, vm_id, cgroup)
        try:
            barrier.wait()
        except threading.BrokenBarrierError as exc:
            raise DualTenantError("tenant A did not join live KVM barrier") from exc
        # Keep tenant B running while tenant A is deliberately SIGKILLed.
        if not killed_a.wait(timeout=110):
            raise DualTenantError("tenant A did not finish bounded crash test")
        if egress.require_vm_running(proc, vm_id, cgroup) != result["cgroup_inode"]:
            raise DualTenantError("tenant B exited or changed cgroup after A crash")
        with guard:
            overlap["peer_b_survived"]=True

    def launch(manifest,uid,hook,trace):
        return fc.run(
            manifest=manifest, approved_root=approved_root, jailer=jailer,
            jailer_sha=jailer_sha, firecracker=firecracker,
            firecracker_sha=firecracker_sha, chroot_base=chroot_base,
            uid=uid, gid=uid, timeout=timeout,
            aya_loader=aya_loader, aya_loader_sha=aya_loader_sha,
            aya_program=aya_program, aya_program_sha=aya_program_sha,
            aya_seconds=aya_seconds, crash_after_proof=True,
            boot_hook=hook,require_guest_wasi=require_guest_wasi,
            traceparent=trace,preboot_host_aya=require_guest_wasi)

    with ThreadPoolExecutor(max_workers=2,thread_name_prefix="ocaf39-tenant") as pool:
        fa = pool.submit(launch,manifest_a,1002,hook_a,
                         traceparents["a"] if traceparents else None)
        fb = pool.submit(launch,manifest_b,1003,hook_b,
                         traceparents["b"] if traceparents else None)
        try:
            a = fa.result(timeout=140)
        finally:
            killed_a.set()
        b = fb.result(timeout=140)

    concurrency = validate_dual_evidence(a,b,overlap["passed"])
    survival = validate_peer_survived_a_crash(a,overlap["peer_b_survived"])
    if a["vm_sigkill_fault"]["vm_cgroup_inode"] == b["vm_sigkill_fault"]["vm_cgroup_inode"]:
        raise DualTenantError("two crash tests used the same kernel cgroup")

    return {
        "schema_version":1,
        "status":"TWO_CONCURRENT_TENANTS_VM_BOOT_AND_FAULT_ISOLATION_PASSED",
        "observed_at_utc":datetime.now(timezone.utc).isoformat(),
        "tenant_a":a,"tenant_b":b,
        "simultaneous_guest_boot":concurrency,
        "fault_isolation":survival,
        "host_boot_id":a["host_boot_id"],
        "two_dedicated_uid_gid":True,
        "two_independent_hosts":False,
        "guest_network_egress_tested":False,
        "stage_039_complete":False,
        "production_approved":False,
    }


def main() -> int:
    ap=argparse.ArgumentParser(description=__doc__)
    for arg in ("manifest-a","manifest-b","approved-root","firecracker",
                "jailer","chroot-base","report"):
        ap.add_argument("--"+arg,required=True,type=Path)
    ap.add_argument("--firecracker-sha256",required=True)
    ap.add_argument("--jailer-sha256",required=True)
    ap.add_argument("--aya-loader",type=Path)
    ap.add_argument("--aya-loader-sha256")
    ap.add_argument("--aya-program",type=Path)
    ap.add_argument("--aya-program-sha256")
    ap.add_argument("--aya-seconds",type=int,default=15)
    ap.add_argument("--timeout",type=int,default=90)
    ap.add_argument("--ack-two-tenant-kvm-sigkill",action="store_true")
    args=ap.parse_args()
    if not args.ack_two_tenant_kvm_sigkill:
        ap.error("explicit --ack-two-tenant-kvm-sigkill is required")
    values=(args.aya_loader,args.aya_loader_sha256,args.aya_program,args.aya_program_sha256)
    if any(x is None for x in values):
        ap.error("both live tenants require all pinned Aya inputs")
    try:
        report=execute(
            manifest_a=args.manifest_a,manifest_b=args.manifest_b,
            approved_root=args.approved_root,firecracker=args.firecracker,
            firecracker_sha=args.firecracker_sha256,jailer=args.jailer,
            jailer_sha=args.jailer_sha256,chroot_base=args.chroot_base,
            aya_loader=args.aya_loader,aya_loader_sha=args.aya_loader_sha256,
            aya_program=args.aya_program,aya_program_sha=args.aya_program_sha256,
            aya_seconds=args.aya_seconds,timeout=args.timeout)
        gate.save_report(args.report,report)
    except (OSError,ValueError) as exc:
        ap.error(str(exc))
    print(json.dumps({"status":report["status"],"production_approved":False,
                      "report":str(args.report)}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
