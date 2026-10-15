#!/usr/bin/env python3
"""Factual cross-runner independent-kernel acceptance; not inter-host RPC.

Compares separately uploaded GitHub runner evidence from distinct jobs,
both of which must have actually booted jailed KVM and survived dedicated
VM SIGKILL/cgroup drain. Merely different hosts do not prove network
partitions, interhost mTLS or durable HA/failover.
"""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path

class IndependentHostError(ValueError):
    pass


def verify_hosts(a:dict,b:dict)->dict:
    if not isinstance(a,dict) or not isinstance(b,dict):
        raise IndependentHostError("missing actual independent host reports")
    if a.get("tenant_id")!="tenant-a" or b.get("tenant_id")!="tenant-b":
        raise IndependentHostError("expected distinct tenant attestations")
    for key in ("host_boot_id","hostname"):
        if (not isinstance(a.get(key),str) or not isinstance(b.get(key),str)
                or not a[key] or not b[key] or a[key]==b[key]):
            raise IndependentHostError("two separate observed hosts required: "+key)
    ga,gb=a.get("guest"),b.get("guest")
    if not isinstance(ga,dict) or not isinstance(gb,dict):
        raise IndependentHostError("missing real guest boot evidence")
    if (not ga.get("vm_id") or not gb.get("vm_id") or
            ga["vm_id"]==gb["vm_id"]):
        raise IndependentHostError("reused microVM boot identity")
    for expected,report in (("tenant-a",ga),("tenant-b",gb)):
        crash=report.get("vm_sigkill_fault")
        if (report.get("tenant_id") != expected or
            report.get("status") != "JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED" or
            report.get("firecracker_api_running") is not True or
            report.get("guest_kernel_console_seen") is not True or
            report.get("production_approved") is not False or
            not isinstance(crash,dict) or
            crash.get("status") != "VM_SIGKILL_CGROUP_SUBTREE_DRAINED" or
            crash.get("cgroup_subtree_empty") is not True or
            type(crash.get("sigkill_exit_code")) is not int or
            crash["sigkill_exit_code"] != -9):
            raise IndependentHostError("incomplete independent real jailed VM fault proof")
    return {
        "schema_version":1,
        "status":"TWO_INDEPENDENT_EPHEMERAL_KERNEL_HOST_KVM_SIGKILL_PROVED",
        "verified_at_utc":datetime.now(timezone.utc).isoformat(),
        "two_separate_kernel_boot_ids_observed":True,
        "two_distinct_hostnames_observed":True,
        "two_distinct_vm_ids_observed":True,
        "two_separate_guest_kernels_booted":True,
        "two_independent_vm_sigkill_and_cgroup_drain_tests":True,
        "tenant_a_kernel_boot_id":a["host_boot_id"],
        "tenant_b_kernel_boot_id":b["host_boot_id"],
        "interhost_rpc_tested":False,
        "interhost_network_partition_tested":False,
        "multi_host_failover_tested":False,
        "durable_production_host_independence_proved":False,
        "stage_039_complete":False,
        "production_approved":False,
    }


def main()->int:
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--tenant-a",required=True,type=Path)
    p.add_argument("--tenant-b",required=True,type=Path)
    p.add_argument("--report",required=True,type=Path)
    x=p.parse_args()
    try:
        output=verify_hosts(json.loads(x.tenant_a.read_text()),
                            json.loads(x.tenant_b.read_text()))
        x.report.write_text(json.dumps(output,indent=2,sort_keys=True)+"\n")
    except (OSError,ValueError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":output["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
