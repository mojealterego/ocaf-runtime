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
import stage039_traceparent as traceparent

OUTPUT=b"OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE\n"
PASS=b"OCAF039_GUEST_WASI_EXECUTION_PASS_EXIT_0\n"
INIT=b"OCAF039_GUEST_PID1_BOOTED\n"
FAILED=b"OCAF039_GUEST_WASI_EXECUTION_FAILED"
HEX=re.compile(r"[0-9a-f]{64}\Z")
VM=re.compile(r"ocaf-[0-9a-f]{24}\Z")
BPF_ID=re.compile(rb"(?m)^OCAF039_GUEST_BPF_SOCKET_FILTER_ID=([1-9][0-9]{0,9})$")
OPENAT_READY=re.compile(rb"(?m)^OCAF039_GUEST_OPENAT_PROBE_READY_ID=([1-9][0-9]{0,9})$")
OPENAT_EVENT=re.compile(rb"(?m)^OCAF039_GUEST_OPENAT_EVENT_ID=([1-9][0-9]{0,9}) COUNT=([1-9][0-9]{0,9}) TGID=([1-9][0-9]{0,9}) PID=([1-9][0-9]{0,9})$")
NONCE=re.compile(r"[0-9a-f]{32}\Z")
TRACEPARENT=traceparent.TRACEPARENT
NEGATIVE_MARKERS=(
    b"OCAF039_GUEST_SECCOMP_SOCKET_DENIED\n",
    b"OCAF039_GUEST_WASI_FUEL_TRAP\n",
    b"OCAF039_GUEST_WASI_TIMEOUT_TRAP\n",
    b"OCAF039_GUEST_WASI_MEMORY_TRAP\n",
    b"OCAF039_GUEST_WASI_CPU_SIGNAL\n",
    b"OCAF039_GUEST_WASI_CHILD_SIGKILL\n",
)

class GuestWasiError(ValueError):pass

RNG_READY=(b"OCAF039_GUEST_CRNG_READY\n",
           b"OCAF039_GUEST_CRNG_READY_AFTER_CPU_RNG\n")


def verify_console(raw:bytes,*,expected_wasi:dict|None=None,
                   tenant_id:str|None=None,
                   expected_traceparent:str|None=None)->dict:
    """Require actual guest CRNG initialization before Wasmtime child output.

    An old Linux 4.14 guest without entropy may block getrandom() forever.
    Wasmtime must never report a successful fd_write when its guest kernel
    RNG was uninitialized, the child exited nonzero, or markers are forged.
    """
    if not isinstance(raw,bytes) or not 0<len(raw)<=1024*1024:
        raise GuestWasiError("guest serial empty or over limit")
    serial_sha256=hashlib.sha256(raw).hexdigest()
    # Linux ttyS0 may translate guest LF writes to CRLF. Normalize only that
    # line ending for marker parsing, while hashing the original serial bytes.
    raw=raw.replace(b"\r\n",b"\n")
    linux=raw.find(b"Linux version ")
    if expected_wasi is None:
        expected_output=OUTPUT
    else:
        if (not isinstance(expected_wasi,dict)
            or set(expected_wasi)!={"task_nonce","module_sha256"}
            or not isinstance(expected_wasi["task_nonce"],str)
            or NONCE.fullmatch(expected_wasi["task_nonce"]) is None
            or not isinstance(expected_wasi["module_sha256"],str)
            or HEX.fullmatch(expected_wasi["module_sha256"]) is None
            or not isinstance(tenant_id,str)
            or tenant_id not in ("tenant-a","tenant-b")
            or not traceparent.is_valid(expected_traceparent)):
            raise GuestWasiError("invalid expected signed tenant WASI task")
        expected_output=(b"OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE:"+
                         tenant_id.encode()+b":"+expected_wasi["task_nonce"].encode()+b"\n")
    for name, marker in (("pid1", INIT), ("fd_write", expected_output), ("exit", PASS)):
        position=raw.find(marker)
        if position<0 or (position>0 and raw[position-1:position]!=bytes([10])):
            previous=raw[max(0,position-24):position] if position>=0 else b""
            raise GuestWasiError(
                f"serial {name} marker must begin at line boundary; previous={previous!r}")
    boot=raw.find(INIT)
    output=raw.find(expected_output)
    done=raw.find(PASS)
    crng=[(marker,raw.find(marker)) for marker in RNG_READY
          if raw.count(marker)==1]
    bpf_ids=list(BPF_ID.finditer(raw))
    if (raw.count(INIT)!=1 or raw.count(expected_output)!=1 or raw.count(PASS)!=1
        or sum(raw.count(m) for m in RNG_READY)!=1 or len(crng)!=1
        or len(bpf_ids)!=1 or raw.count(b"OCAF039_GUEST_BPF_SOCKET_FILTER_ID=")!=1
        or b"OCAF039_GUEST_BPF_LOAD_DENIED" in raw
        or FAILED in raw or b"OCAF039_GUEST_HW_RNG_SEED_DENIED" in raw
        or b"OCAF039_GUEST_CRNG_STILL_UNAVAILABLE" in raw
        or raw.count(b"Linux version ")!=1
        or not 0<=linux<boot<crng[0][1]<bpf_ids[0].start()<output<done):
        raise GuestWasiError(
            "genuine bounded Linux guest PID1 WASIp1 exit/CRNG proof absent; "
            f"counts linux={raw.count(b'Linux version ')} pid1={raw.count(INIT)} "
            f"crng={sum(raw.count(m) for m in RNG_READY)} bpf={len(bpf_ids)} "
            f"output={raw.count(expected_output)} pass={raw.count(PASS)}")
    if expected_wasi is not None:
        module_marker=(b"OCAF039_GUEST_WASI_MODULE_SHA256="+
                       expected_wasi["module_sha256"].encode()+b"\n")
        trace_marker=(b"OCAF039_GUEST_TRACEPARENT="+
                      expected_traceparent.encode()+b"\n")
        tenant_marker=b"OCAF039_GUEST_WASI_TENANT="+tenant_id.encode()+b"\n"
        nonce_marker=(b"OCAF039_GUEST_WASI_NONCE="+
                      expected_wasi["task_nonce"].encode()+b"\n")
        module_at=raw.find(module_marker)
        trace_at=raw.find(trace_marker)
        tenant_at=raw.find(tenant_marker)
        nonce_at=raw.find(nonce_marker)
        ready=list(OPENAT_READY.finditer(raw))
        events=list(OPENAT_EVENT.finditer(raw))
        negative_positions=[raw.find(marker) for marker in NEGATIVE_MARKERS]
        if (raw.count(module_marker)!=1 or raw.count(trace_marker)!=1
            or raw.count(tenant_marker)!=1
            or raw.count(nonce_marker)!=1
            or raw.count(b"OCAF039_GUEST_WASI_MODULE_SHA256=")!=1
            or raw.count(b"OCAF039_GUEST_TRACEPARENT=")!=1
            or raw.count(b"OCAF039_GUEST_WASI_TENANT=")!=1
            or raw.count(b"OCAF039_GUEST_WASI_NONCE=")!=1
            or len(ready)!=1 or len(events)!=1
            or raw.count(b"OCAF039_GUEST_OPENAT_PROBE_READY_ID=")!=1
            or raw.count(b"OCAF039_GUEST_OPENAT_EVENT_ID=")!=1
            or ready[0].group(1)!=events[0].group(1)
            or b"OCAF039_GUEST_OPENAT_WASMTIME_EVENT_TIMEOUT" in raw
            or b"OCAF039_GUEST_OPENAT_BPF_VERIFIER_DENIED" in raw
            or b"OCAF039_GUEST_WASI_MODULE_DIGEST_DENIED" in raw
            or b"OCAF039_GUEST_NEGATIVE_BUDGET_DENIED" in raw
            or any(raw.count(marker)!=1 for marker in NEGATIVE_MARKERS)
            or any(position<=0 or raw[position-1:position]!=b"\n"
                   for position in negative_positions)
            or not bpf_ids[0].start()<trace_at<module_at<ready[0].start()<events[0].start()<output<tenant_at<nonce_at<negative_positions[0]
            or not all(a<b for a,b in zip(negative_positions,negative_positions[1:]))
            or not negative_positions[-1]<done):
            raise GuestWasiError("guest module, identity, openat or negative budget receipt mismatch")
    result={
        "guest_pid1_started":True,"wasi_preview1_fd_write_observed_inside_guest":True,
        "guest_wasmtime_child_exited_zero":True,
        "guest_crng_ready_before_execution":True,
        "guest_crng_seeded_by_cpu_rng":crng[0][0]==RNG_READY[1],
        "guest_kernel_bpf_program_id":int(bpf_ids[0].group(1)),
        "guest_bpf_socket_filter_attached":True,
        "serial_sha256":serial_sha256,
    }
    if expected_wasi is not None:
        result.update({"tenant_id":tenant_id,
                       "task_nonce":expected_wasi["task_nonce"],
                       "module_sha256":expected_wasi["module_sha256"],
                       "guest_module_digest_checked":True,
                       "guest_openat_bpf_program_id":int(ready[0].group(1)),
                       "guest_openat_event_count":int(events[0].group(2)),
                       "guest_wasmtime_tgid":int(events[0].group(3)),
                       "guest_openat_event_pid":int(events[0].group(4)),
                       "guest_negative_wasi_budgets_verified":True,
                       "guest_seccomp_socket_denied":True})
        result["traceparent"]=expected_traceparent
    return result


def verify_brokered_pair(broker_report:dict,jailer_base:Path,
                         rootfs_sha256:str|dict[str,str])->dict:
    rootfs_by_tenant=(rootfs_sha256 if isinstance(rootfs_sha256,dict)
                      else {"a":rootfs_sha256,"b":rootfs_sha256})
    if (not isinstance(broker_report,dict)
        or not isinstance(jailer_base,Path) or not jailer_base.is_absolute()
        or jailer_base.is_symlink() or not jailer_base.is_dir()
        or not isinstance(rootfs_by_tenant,dict) or set(rootfs_by_tenant)!={"a","b"}
        or any(not isinstance(x,str) or HEX.fullmatch(x) is None
               for x in rootfs_by_tenant.values())
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
        or binding.get("signed_wasi_manifest_verified") is not True
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
            or vm.get("guest_rootfs_sha256")!=rootfs_by_tenant[key]
            or vm.get("firecracker_api_running") is not True
            or vm.get("guest_kernel_console_seen") is not True
            or vm.get("process_in_expected_cgroup") is not True
            or vm.get("production_approved") is not False):
            raise GuestWasiError("unauthorized actual guest rootfs or tenant mismatch")
        vm_id=vm.get("vm_id")
        if not isinstance(vm_id,str) or VM.fullmatch(vm_id) is None:
            raise GuestWasiError("invalid bounded VM ID")
        firecracker_dir=jailer_base/"firecracker"
        vm_dir=firecracker_dir/vm_id
        path=vm_dir/"serial-console.log"
        # O_NOFOLLOW on the final file does not protect ancestor directories.
        # Reject redirected jailer trees before consuming serial evidence.
        if (firecracker_dir.is_symlink() or vm_dir.is_symlink()
            or not firecracker_dir.is_dir() or not vm_dir.is_dir()
            or path.is_symlink() or not path.is_file()):
            raise GuestWasiError("missing or redirected jailed Firecracker serial file")
        flags=os.O_RDONLY|getattr(os,"O_NOFOLLOW",0)|getattr(os,"O_NONBLOCK",0)
        fd=os.open(path,flags)
        try:
            before=os.fstat(fd)
            if not stat.S_ISREG(before.st_mode) or before.st_nlink!=1 or not 0<before.st_size<=1024*1024:
                raise GuestWasiError("tampered or oversized real KVM serial")
            raw=os.read(fd,1024*1024+1)
            after=os.fstat(fd)
            if (before.st_dev!=after.st_dev or before.st_ino!=after.st_ino
                or before.st_mtime_ns!=after.st_mtime_ns
                or before.st_ctime_ns!=after.st_ctime_ns
                or before.st_size!=after.st_size or len(raw)!=before.st_size):
                raise GuestWasiError("guest evidence changed during read")
        finally:os.close(fd)
        expected_wasi=vm.get("wasi")
        receipts[key]={**verify_console(raw,expected_wasi=expected_wasi,
                                        tenant_id=vm.get("tenant_id") if expected_wasi else None,
                                        expected_traceparent=vm.get("traceparent") if expected_wasi else None),
                       "vm_id":vm_id}
        names.append(vm_id)
    if len(set(names))!=2:
        raise GuestWasiError("two different KVM guests required")
    distinct_tasks=(rootfs_by_tenant["a"]!=rootfs_by_tenant["b"]
                    and all("wasi" in broker_report["tenant_"+k] for k in ("a","b"))
                    and receipts["a"].get("task_nonce")!=receipts["b"].get("task_nonce")
                    and receipts["a"].get("module_sha256")!=receipts["b"].get("module_sha256"))
    if any("wasi" in broker_report["tenant_"+k] for k in ("a","b")) and not distinct_tasks:
        raise GuestWasiError("distinct signed WASI tenant tasks required")
    if distinct_tasks:
        expected_traces=binding.get("tenant_traceparents")
        if (binding.get("guest_traceparent_propagated") is not True
            or not isinstance(expected_traces,dict) or set(expected_traces)!={"a","b"}
            or len(set(expected_traces.values()))!=2
            or any(receipts[k].get("traceparent")!=expected_traces[k]
                   for k in ("a","b"))):
            raise GuestWasiError("cross-tenant guest trace propagation mismatch")
    return {
        "status":"TWO_REAL_BROKERED_JAILED_FIRECRACKER_GUESTS_WASIP1_FD_WRITE_EXECUTED",
        "real_guest_wasi_execution_verified":True,
        "both_tenants_own_vm_and_broker_reservations":True,
        "guest_rootfs_sha256":rootfs_by_tenant,
        "tenant_receipts":receipts,
        "two_independent_hosts":False,
        "different_wasm_task_per_tenant_tested":distinct_tasks,
        "signed_wasi_manifest_verified":True,
        "production_approved":False,
        "stage_039_complete":False,
    }

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--broker-report",type=Path,required=True)
    p.add_argument("--jailer-base",type=Path,required=True)
    p.add_argument("--rootfs-sha256")
    p.add_argument("--rootfs-sha256-a")
    p.add_argument("--rootfs-sha256-b")
    p.add_argument("--report",type=Path,required=True)
    opt=p.parse_args()
    try:
        broker_report=json.loads(opt.broker_report.read_bytes())
        if opt.rootfs_sha256:
            rootfs=opt.rootfs_sha256
        elif opt.rootfs_sha256_a and opt.rootfs_sha256_b:
            rootfs={"a":opt.rootfs_sha256_a,"b":opt.rootfs_sha256_b}
        else:
            raise GuestWasiError("both per-tenant rootfs digests required")
        proof=verify_brokered_pair(broker_report,opt.jailer_base,rootfs)
        with opt.report.open("x") as fp:
            json.dump(proof,fp,sort_keys=True,indent=2)
            fp.write("\n")
    except (OSError,ValueError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":proof["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
