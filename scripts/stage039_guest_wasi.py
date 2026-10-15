#!/usr/bin/env python3
"""0.39.19: offline verify the TWO real jailed Firecracker WASI serial receipts.

This validator never substitutes host-side WASI execution or a fake VM.
Reports MUST match an authenticated broker's real two-tenant KVM run,
the pinned guest-rootfs SHA and distinct actual jailer-VM serial paths.
"""
from __future__ import annotations
import argparse, hashlib, json, os, re, stat
from pathlib import Path
from typing import Any

OUTPUT=b"OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE\n"
PASS=b"OCAF039_GUEST_WASI_EXECUTION_PASS_EXIT_0\n"
INIT=b"OCAF039_GUEST_PID1_BOOTED\n"
FAILED=b"OCAF039_GUEST_WASI_EXECUTION_FAILED"
HEX=re.compile(r"[0-9a-f]{64}\Z")
VM=re.compile(r"ocaf-[0-9a-f]{24}\Z")

class GuestWasiError(ValueError):pass

RNG_READY=(b"OCAF039_GUEST_CRNG_READY\n",
           b"OCAF039_GUEST_CRNG_READY_AFTER_CPU_RNG\n")


def verify_console(raw:bytes)->dict:
    """Require actual guest CRNG initialization before Wasmtime child output.

    An old Linux 4.14 guest without entropy may block getrandom() forever.
    Wasmtime must never report a successful fd_write when its guest kernel
    RNG was uninitialized, the child exited nonzero, or markers are forged.
    """
    if not isinstance(raw,bytes) or not 0<len(raw)<=1024*1024:
        raise GuestWasiError("guest serial empty or over limit")
    boot=raw.find(INIT)
    output=raw.find(OUTPUT)
    done=raw.find(PASS)
    crng=[(marker,raw.find(marker)) for marker in RNG_READY
          if raw.count(marker)==1]
    if (raw.count(INIT)!=1 or raw.count(OUTPUT)!=1 or raw.count(PASS)!=1
        or sum(raw.count(m) for m in RNG_READY)!=1 or len(crng)!=1
        or FAILED in raw or b"OCAF039_GUEST_HW_RNG_SEED_DENIED" in raw
        or b"OCAF039_GUEST_CRNG_STILL_UNAVAILABLE" in raw
        or b"Linux version " not in raw
        or not 0<=boot<crng[0][1]<output<done):
        raise GuestWasiError("genuine bounded Linux guest PID1 WASIp1 exit/CRNG proof absent")
    return {
        "guest_pid1_started":True,"wasi_preview1_fd_write_observed_inside_guest":True,
        "guest_wasmtime_child_exited_zero":True,
        "guest_crng_ready_before_execution":True,
        "guest_crng_seeded_by_cpu_rng":crng[0][0]==RNG_READY[1],
        "serial_sha256":hashlib.sha256(raw).hexdigest(),
    }


def verify_brokered_pair(broker_report:dict,jailer_base:Path,rootfs_sha256:str)->dict:
    if (not isinstance(broker_report,dict)
        or not isinstance(jailer_base,Path) or not jailer_base.is_absolute()
        or jailer_base.is_symlink() or not jailer_base.is_dir()
        or not isinstance(rootfs_sha256,str) or HEX.fullmatch(rootfs_sha256) is None
        or broker_report.get("status")!="TWO_CONCURRENT_TENANTS_VM_BOOT_AND_FAULT_ISOLATION_PASSED"
        or broker_report.get("production_approved") is not False
        or broker_report.get("stage_039_complete") is not False):
        raise GuestWasiError("untrusted rootfs digest, VM base or broker KVM report")
    binding=broker_report.get("broker_vm_binding")
    if (not isinstance(binding,dict)
        or binding.get("status")!="BROKER_SO_PEERCRED_TWO_REAL_JAILED_VM_BOOT_BOUND"
        or binding.get("real_kernel_so_peercred_authenticated_broker") is not True
        or binding.get("durable_sqlite_leases_matched") is not True
        or binding.get("admission_before_vm_launch") is not True
        or binding.get("durable_broker_leases_released_after_vm_crashes") is not True
        or binding.get("wasi_executed_inside_guest") is not True
        or binding.get("signed_wasi_manifest_verified") is not False
        or binding.get("production_approved") is not False):
        allowed_keys=("status","real_kernel_so_peercred_authenticated_broker",
                     "durable_sqlite_leases_matched","admission_before_vm_launch",
                     "durable_broker_leases_released_after_vm_crashes",
                     "wasi_executed_inside_guest","production_approved")
        diagnostics={k:binding.get(k) for k in allowed_keys} if isinstance(binding,dict) else {"binding_is_dict":False}
        raise GuestWasiError("missing original kernel-authenticated broker reservation: "+json.dumps(diagnostics,sort_keys=True))
    receipts={}
    names=[]
    for key in ("a","b"):
        vm=broker_report.get("tenant_"+key)
        if (not isinstance(vm,dict)
            or vm.get("tenant_id")!="tenant-"+key
            or vm.get("guest_rootfs_sha256")!=rootfs_sha256
            or vm.get("firecracker_api_running") is not True
            or vm.get("guest_kernel_console_seen") is not True
            or vm.get("process_in_expected_cgroup") is not True
            or vm.get("production_approved") is not False):
            raise GuestWasiError("unauthorized actual guest rootfs or tenant mismatch")
        vm_id=vm.get("vm_id")
        if not isinstance(vm_id,str) or VM.fullmatch(vm_id) is None:
            raise GuestWasiError("invalid bounded VM ID")
        path=jailer_base/"firecracker"/vm_id/"serial-console.log"
        if path.is_symlink() or not path.is_file():
            raise GuestWasiError("missing real jailed Firecracker serial file")
        flags=os.O_RDONLY|getattr(os,"O_NOFOLLOW",0)
        fd=os.open(path,flags)
        try:
            before=os.fstat(fd)
            if not stat.S_ISREG(before.st_mode) or before.st_nlink!=1 or not 0<before.st_size<=1024*1024:
                raise GuestWasiError("tampered or oversized real KVM serial")
            raw=os.read(fd,1024*1024+1)
            after=os.fstat(fd)
            if before.st_ino!=after.st_ino or before.st_size!=after.st_size or len(raw)!=before.st_size:
                raise GuestWasiError("guest evidence changed during read")
        finally:os.close(fd)
        receipts[key]={**verify_console(raw),"vm_id":vm_id}
        names.append(vm_id)
    if len(set(names))!=2:
        raise GuestWasiError("two different KVM guests required")
    return {
        "status":"TWO_REAL_BROKERED_JAILED_FIRECRACKER_GUESTS_WASIP1_FD_WRITE_EXECUTED",
        "real_guest_wasi_execution_verified":True,
        "both_tenants_own_vm_and_broker_reservations":True,
        "guest_rootfs_sha256":rootfs_sha256,
        "tenant_receipts":receipts,
        "two_independent_hosts":False,
        "different_wasm_task_per_tenant_tested":False,
        "signed_wasi_manifest_verified":False,
        "production_approved":False,
        "stage_039_complete":False,
    }

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--broker-report",type=Path,required=True)
    p.add_argument("--jailer-base",type=Path,required=True)
    p.add_argument("--rootfs-sha256",required=True)
    p.add_argument("--report",type=Path,required=True)
    opt=p.parse_args()
    try:
        broker_report=json.loads(opt.broker_report.read_bytes())
        proof=verify_brokered_pair(broker_report,opt.jailer_base,opt.rootfs_sha256)
        with opt.report.open("x") as fp:
            json.dump(proof,fp,sort_keys=True,indent=2)
            fp.write("\n")
    except (OSError,ValueError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":proof["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
