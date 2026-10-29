#!/usr/bin/env python3
"""Classify separate KVM and PostgreSQL fault evidence without false integration."""
from __future__ import annotations
import argparse
import json
from pathlib import Path


class EvidenceError(ValueError):
    pass


def classify(quarantine: dict, broker: dict, recovery: dict) -> dict:
    if (not isinstance(quarantine,dict)
            or quarantine.get('status')!='ONE_HOST_TWO_JAILED_VMS_SIGNED_GUEST_QUARANTINE_LAB'
            or quarantine.get('tenant_b_survived_a_quarantine') is not True
            or quarantine.get('production_approved') is not False):
        raise EvidenceError('real KVM peer-safe guest quarantine receipt missing')
    a=broker.get('tenant_a') if isinstance(broker,dict) else None
    if (broker.get('status')!='ONE_HOST_GUEST_AYA_LIVE_BROKER_SIGKILL_DENIED'
            or not isinstance(a,dict)
            or a.get('broker_process_sigkill_observed') is not True
            or a.get('tenant_b_live_during_denial') is not True
            or a.get('permit_issued') is not False
            or a.get('host_aya_program_id',0)<=0
            or a.get('guest_aya_program_id',0)<=0):
        raise EvidenceError('live broker fault or Aya guest-host denial missing')
    if (recovery.get('status')!='TWO_LINUX_KERNELS_REAL_PHYSICAL_WAL_PITR_RECOVERY_PASSED'
            or recovery.get('source_boot_id')==recovery.get('recovery_boot_id')
            or recovery.get('real_wal_replay_exited_recovery') is not True
            or recovery.get('production_approved') is not False
            or recovery.get('stage_041_complete') is not False):
        raise EvidenceError('independent kernel PostgreSQL recovery missing')
    return {'schema_version':1,'status':'SEPARATE_FAULT_LABS_VERIFIED',
            'real_guest_quarantine_peer_survived':True,
            'real_broker_sigkill_guest_host_aya_denied':True,
            'real_postgresql_second_kernel_pitr':True,
            'same_fault_window_tested':False,
            'broker_vm_postgres_integrated_fault_tested':False,
            'stage_041_complete':False,'production_approved':False}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('quarantine','broker','recovery','report'):
        p.add_argument('--'+name,type=Path,required=True)
    a=p.parse_args()
    result=classify(*[json.loads(getattr(a,name).read_text())
                      for name in ('quarantine','broker','recovery')])
    with a.report.open('x') as out:
        json.dump(result,out,sort_keys=True,indent=2)
        out.write('\n')


if __name__=='__main__': main()
