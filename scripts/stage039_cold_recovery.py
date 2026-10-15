#!/usr/bin/env python3
"""0.39.13: fail-closed artifact-mediated cold-restart evidence on two CI kernels.

This does NOT test real-time interhost RPC, heartbeat, leases or automatic HA.
The handoff is serialized using GitHub Actions artifact storage.
"""
from __future__ import annotations
import argparse
from datetime import datetime,timezone
import hashlib
import json
from pathlib import Path
import re

class ColdHandoffError(ValueError):
    pass

def validate_handoff(a:dict,b:dict)->dict:
    if not isinstance(a,dict) or not isinstance(b,dict):
        raise ColdHandoffError("two actual host guest proofs required")
    nonce=a.get("checkpoint_nonce")
    if (not isinstance(nonce,str) or
            re.fullmatch("[a-f0-9]{16}",nonce) is None or
            b.get("checkpoint_nonce")!=nonce):
        raise ColdHandoffError("cross-host nonce mismatch or reuse")
    if (not isinstance(a.get("host_boot_id"),str) or
            not isinstance(b.get("host_boot_id"),str) or
            not a["host_boot_id"] or not b["host_boot_id"] or
            a["host_boot_id"]==b["host_boot_id"]):
        raise ColdHandoffError("real kernel boot identities not distinct")
    try:
        t_a=datetime.fromisoformat(a["observed_at_utc"])
        t_b=datetime.fromisoformat(b["observed_at_utc"])
    except (ValueError,TypeError,KeyError) as exc:
        raise ColdHandoffError("missing host handoff event timestamps") from exc
    if (t_a.tzinfo is None or t_b.tzinfo is None
            or not t_b>t_a or (t_b-t_a).total_seconds()>1800):
        raise ColdHandoffError("recovery event did not follow source VM crash")
    for entry in (a,b):
        if (entry.get("status")!="TWO_JAILED_GUEST_ORIGINATED_UDP_TENANTS_ISOLATED_AND_FAULTED" or
                entry.get("jailer_used_for_both_guests") is not True or
                entry.get("guest_originated_network_tested") is not True or
                entry.get("tenant_a_denied_by_nftables_ingress") is not True or
                entry.get("tenant_a_recovered_after_firewall_removal") is not True or
                entry.get("tenant_b_survived_a_vm_sigkill") is not True or
                entry.get("production_approved") is not False):
            raise ColdHandoffError("guest network/kvm acceptance missing on a host")
        for key in ("tenant_a_vm_sigkill","tenant_b_vm_sigkill"):
            fault=entry.get(key)
            if (not isinstance(fault,dict) or
                    type(fault.get("sigkill_exit_code")) is not int or
                    fault["sigkill_exit_code"]!=-9 or
                    fault.get("cgroup_subtree_empty") is not True):
                raise ColdHandoffError("VM SIGKILL and cgroup empty not proven")
    return {
       "schema_version":1,
       "status":"TWO_KERNEL_JAILED_GUEST_NONCE_COLD_RESTART_PASSED",
       "source_host_boot_id":a["host_boot_id"],
       "recovery_host_boot_id":b["host_boot_id"],
       "separate_kernel_boot_ids":True,
       "guest_nonce_delivered_on_recovery_host":True,
       "artifact_mediated_restart_tested":True,
       "source_vm_sigkill_observed_before_recovery":True,
       "host_power_loss_tested":False,
       "live_interhost_rpc_failover_tested":False,
       "mTLS_interhost_transport_tested":False,
       "durable_multi_host_production_tested":False,
       "stage_039_complete":False,
       "production_approved":False,
    }


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for key in ("source","recovery","report"):
        parser.add_argument("--"+key,type=Path,required=True)
    a=parser.parse_args()
    try:
        proof=validate_handoff(json.loads(a.source.read_text()),
                               json.loads(a.recovery.read_text()))
        a.report.write_text(json.dumps(proof,sort_keys=True,indent=2)+"\n")
    except (OSError,ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":proof["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
