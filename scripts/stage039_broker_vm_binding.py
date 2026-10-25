#!/usr/bin/env python3
"""Stage 0.39.18: actual AF_UNIX SO_PEERCRED broker reservation -> jailed VM boot.

The broker *reserves* two pinned, netless tenant workloads through real
Linux kernel SO_PEERCRED, backed by a durable SQLite lease. ONLY AFTER
both reservations are verified does stage039_dual_tenant launch two
simultaneous jailed Firecracker guest kernels with Aya host-cgroup tests,
kill A and verify B remains running. Boot identities, guest/rootfs digests,
actual kernel cgroup inodes and SIGKILL receipts are bound back to the
exact reservations, and both leases are finally released.

This is not signed WASI execution inside Firecracker; no WASI runtime is
present in the test rootfs. It is also not production admission authority.
"""
from __future__ import annotations

import argparse
import asyncio
import base64
from datetime import datetime,timezone
import hashlib
import json
import os
import re
from pathlib import Path
import secrets
import socket
import stat
import struct
import time
import hmac

import stage039_broker_ipc as ipc
import stage039_broker_ledger as broker
import stage039_dual_tenant as dual
import stage039_microvm_gate as gate
import stage039_signed_manifest as signed_manifest
import stage039_traceparent as traceparent


class BindingError(ValueError):
    pass


def _operator_file(path:Path, maximum:int)->bytes:
    """Read a root-owned operator input once through an O_NOFOLLOW fd."""
    if (not isinstance(path,Path) or not path.is_absolute() or ".." in path.parts
        or path.is_symlink()):
        raise BindingError("operator signer path must be absolute and non-symlinked")
    parent=path.parent
    while parent != parent.parent:
        info=parent.stat()
        sticky_root_tmp=(info.st_uid==0 and info.st_mode & stat.S_ISVTX
                         and info.st_mode & 0o777==0o777)
        if (parent.is_symlink() or not stat.S_ISDIR(info.st_mode)
            or info.st_uid not in (0,os.geteuid()) and not sticky_root_tmp
            or info.st_mode & 0o022 and not sticky_root_tmp):
            raise BindingError("operator signer parent path is mutable")
        parent=parent.parent
    try:
        fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_CLOEXEC)
    except OSError as exc:
        raise BindingError("operator signer input cannot be opened") from exc
    try:
        info=os.fstat(fd)
        if (not stat.S_ISREG(info.st_mode) or info.st_nlink!=1
            or info.st_uid!=os.geteuid() or info.st_mode & 0o022
            or not 0<info.st_size<=maximum):
            raise BindingError("operator signer input owner, mode or size denied")
        data=b""
        while len(data)<=maximum:
            chunk=os.read(fd,min(65536,maximum+1-len(data)))
            if not chunk:
                break
            data+=chunk
        if len(data)!=info.st_size:
            raise BindingError("operator signer input changed during read")
        return data
    finally:
        os.close(fd)


def load_signer_inputs(*,signed_manifest_a:Path|None,
                       signed_manifest_b:Path|None,
                       trusted_public_key:Path|None,
                       trusted_key_id:str,
                       signer_mode:str="lab",
                       trusted_public_key_sha256:str|None=None)->tuple[dict|None,dict|None]:
    """Load signed inputs before a broker lease or VM is created."""
    if signer_mode not in ("lab","operator"):
        raise BindingError("unknown signer mode")
    if all(item is None for item in (signed_manifest_a,signed_manifest_b,trusted_public_key)):
        if signer_mode=="operator" or trusted_public_key_sha256 is not None:
            raise BindingError("operator signer requires two envelopes and trust anchor")
        return None,None
    if any(item is None for item in (signed_manifest_a,signed_manifest_b,trusted_public_key)):
        raise BindingError("both signed guest manifests and trusted key are required")
    if signed_manifest.KEY_ID.fullmatch(trusted_key_id) is None:
        raise BindingError("trusted key ID is invalid")
    if signer_mode=="operator":
        if (trusted_key_id=="stage039-ci" or
            not isinstance(trusted_public_key_sha256,str) or
            gate.HEX_SHA256.fullmatch(trusted_public_key_sha256) is None):
            raise BindingError("operator signer needs a distinct ID and pinned public key digest")
        public=_operator_file(trusted_public_key,32)
        signed_a=_operator_file(signed_manifest_a,256*1024)
        signed_b=_operator_file(signed_manifest_b,256*1024)
        if not hmac.compare_digest(hashlib.sha256(public).hexdigest(),trusted_public_key_sha256):
            raise BindingError("operator trust anchor digest mismatch")
    else:
        if trusted_public_key_sha256 is not None:
            raise BindingError("public key pin belongs to operator signer mode")
        if (trusted_public_key.is_symlink() or not trusted_public_key.is_file()
            or trusted_public_key.stat().st_size!=32):
            raise BindingError("trusted public key is invalid")
        public=trusted_public_key.read_bytes()
        signed_a=signed_manifest_a.read_bytes()
        signed_b=signed_manifest_b.read_bytes()
    if len(public)!=32:
        raise BindingError("trusted public key length invalid")
    keys={trusted_key_id:public}
    envelopes={"a":signed_a,"b":signed_b}
    try:
        for value in envelopes.values():
            signed_manifest.verify(value,keys)
    except signed_manifest.SignedManifestError as exc:
        raise BindingError("signed guest manifest rejected before admission") from exc
    return envelopes,keys


def _manifest(raw:bytes)->dict:
    try:
        obj=json.loads(raw,object_pairs_hook=gate._unique_object,
                       parse_constant=gate._reject_nonfinite)
    except (ValueError,UnicodeError,TypeError) as exc:
        raise BindingError("untrusted JSON manifest") from exc
    if not isinstance(obj,dict):
        raise BindingError("workload manifest must be an object")
    iso=obj.get("isolation")
    if (not isinstance(iso,dict)
        or iso.get("network")!="none" or iso.get("guest_egress") is not False
        or iso.get("jailer") is not True):
        raise BindingError("live broker/VM binding requires explicit netless jailed policy")
    return obj


def verify_ledger_rows(rows:dict,reports:dict)->None:
    if set(rows)!=set(("a","b")) or set(reports)!=set(("a","b")):
        raise BindingError("both independent broker leases required")
    for tenant in ("a","b"):
        row=rows[tenant]
        receipt=reports[tenant]
        if (not isinstance(row,(tuple,list)) or len(row)!=6
            or row[0]!=receipt.get("tenant_id")
            or row[1]!=receipt.get("agent_id")
            or row[2]!=receipt.get("workload_id")
            or type(row[3]) is not int or row[3]!=receipt.get("leader_epoch")
            or row[4]!=receipt.get("request_id")
            or row[5]!="reserved"):
            raise BindingError("broker reservation no longer matches durable active lease")


def verify_binding(reports:dict,manifests:dict,actual:dict,*,require_wasi:bool=False,
                   signed_manifests:dict|None=None,
                   trusted_keys:dict|None=None)->dict:
    if (not isinstance(reports,dict) or not isinstance(manifests,dict)
        or set(reports)!=set(manifests)!=set(("a","b"))
        or not isinstance(actual,dict)
        or actual.get("status")!="TWO_CONCURRENT_TENANTS_VM_BOOT_AND_FAULT_ISOLATION_PASSED"
        or actual.get("production_approved") is not False
        or actual.get("stage_039_complete") is not False
        or actual.get("two_independent_hosts") is not False
        or actual.get("guest_network_egress_tested") is not False
        or not isinstance(actual.get("simultaneous_guest_boot"),dict)
        or actual["simultaneous_guest_boot"].get("two_guests_simultaneously_running") is not True
        or not isinstance(actual.get("fault_isolation"),dict)
        or actual["fault_isolation"].get("tenant_b_survived_tenant_a_sigkill") is not True):
        raise BindingError("real simultaneous jailed guest + tenant crash proof absent")
    vmids=[]
    inodes=[]
    bootids=[]
    requestids=[]
    manifest_hashes=[]
    host_aya_ids=[]
    traceparents={}
    if signed_manifests is not None and set(signed_manifests) != {"a", "b"}:
        raise BindingError("signed manifest set must cover both tenants")
    if signed_manifests is not None and not trusted_keys:
        raise BindingError("trusted signing key is required")
    for tenant in ("a","b"):
        raw=manifests[tenant]
        if not isinstance(raw,bytes) or not 0<len(raw)<=gate.MAX_MANIFEST_BYTES:
            raise BindingError("missing bounded pinned manifest")
        m=_manifest(raw)
        if signed_manifests is not None:
            try:
                signed = signed_manifest.verify(signed_manifests[tenant], trusted_keys)
            except (KeyError, signed_manifest.SignedManifestError) as exc:
                raise BindingError("signed guest manifest verification failed") from exc
            if signed["payload"] != m:
                raise BindingError("signed guest manifest payload differs from admitted manifest")
        rec=reports[tenant]
        actual_vm=actual.get("tenant_"+tenant)
        if (not isinstance(rec,dict) or not isinstance(actual_vm,dict)
            or rec.get("status")!="RESERVED_NOT_EXECUTED"
            or rec.get("microvm_launched") is not False
            or rec.get("ebpf_attached") is not False
            or rec.get("production_approved") is not False
            or rec.get("stage_039_complete") is not False
            or not isinstance(rec.get("request_id"),str)
            or broker.REQ_ID.fullmatch(rec["request_id"]) is None
            or type(rec.get("leader_epoch")) is not int or rec["leader_epoch"]<=0
            or rec.get("manifest_sha256")!=hashlib.sha256(raw).hexdigest()
            or any(rec.get(k)!=m.get(k) or actual_vm.get(k)!=m.get(k)
                   for k in ("tenant_id","agent_id","workload_id"))
            or actual_vm.get("status")!="JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED"
            or any(actual_vm.get(k) is not True for k in
                   ("firecracker_api_running","guest_kernel_console_seen",
                    "process_in_expected_cgroup"))
            or actual_vm.get("production_approved") is not False
            or type(actual_vm.get("cgroup_inode")) is not int
            or actual_vm["cgroup_inode"]<=0
            or not isinstance(actual_vm.get("vm_id"),str)
            or dual.fc.VALID_VM.fullmatch(actual_vm["vm_id"]) is None
            or not isinstance(actual_vm.get("host_boot_id"),str)
            or len(actual_vm["host_boot_id"])<16):
            raise BindingError("VM boot does not match authenticated broker reservation")
        expected=m.get("assets")
        if (not isinstance(expected,dict)
            or any(not isinstance(expected.get(name),dict)
                   or expected[name].get("sha256")!=actual_vm.get(key)
                   for name,key in (("guest_kernel","guest_kernel_sha256"),
                                    ("guest_rootfs","guest_rootfs_sha256"),
                                    ("firecracker","firecracker_sha256")))):
            raise BindingError("running guest/Firecracker digest diverged from admitted inputs")
        if require_wasi:
            guest=actual_vm.get("guest_wasi_execution")
            preboot=actual_vm.get("vm_preboot_aya_egress")
            wasi=m.get("wasi")
            try:
                expected_trace=traceparent.derive(rec["request_id"], rec["manifest_sha256"],m["tenant_id"])
            except (KeyError,ValueError) as exc:
                raise BindingError("invalid broker-derived VM trace identity") from exc
            if (not isinstance(preboot,dict)
                or preboot.get("status")!="HOST_AYA_ATTACHED_BEFORE_FIRECRACKER_START"
                or preboot.get("vm_cgroup_inode")!=actual_vm["cgroup_inode"]
                or preboot.get("bpf_elf_sha256")!=m["assets"]["aya_cgroup_device"]["sha256"]
                or not isinstance(preboot.get("aya_loader_sha256"),str)
                or re.fullmatch("[a-f0-9]{64}",preboot["aya_loader_sha256"]) is None
                or type(preboot.get("kernel_bpf_program_id")) is not int
                or preboot["kernel_bpf_program_id"]<=0
                or preboot.get("host_probe_denied_before_vm_start") is not True
                or preboot.get("guest_wasi_observed_while_attached") is not True
                or preboot.get("aya_detached_after_vm_stop") is not True
                or preboot.get("host_probe_recovered_after_vm_stop") is not True
                or preboot.get("guest_network_egress_tested") is not False
                or preboot.get("production_approved") is not False
                or preboot.get("stage_039_complete") is not False
                or not isinstance(wasi,dict)
                or set(wasi)!={"task_nonce","module_sha256"}
                or actual_vm.get("wasi")!=wasi
                or not isinstance(guest,dict)
                or guest.get("guest_pid1_started") is not True
                or guest.get("wasi_preview1_fd_write_observed_inside_guest") is not True
                or guest.get("guest_wasmtime_child_exited_zero") is not True
                or guest.get("guest_bpf_socket_filter_attached") is not True
                or guest.get("guest_module_digest_checked") is not True
                or guest.get("tenant_id")!=m.get("tenant_id")
                or guest.get("task_nonce")!=wasi["task_nonce"]
                or guest.get("module_sha256")!=wasi["module_sha256"]
                or actual_vm.get("traceparent")!=expected_trace
                or guest.get("traceparent")!=expected_trace
                or type(guest.get("guest_openat_bpf_program_id")) is not int
                or guest["guest_openat_bpf_program_id"]<=0
                or type(guest.get("guest_openat_event_count")) is not int
                or guest["guest_openat_event_count"]<=0
                or type(guest.get("guest_wasmtime_tgid")) is not int
                or guest["guest_wasmtime_tgid"]<=1
                or type(guest.get("guest_openat_event_pid")) is not int
                or guest["guest_openat_event_pid"]<=0
                or type(guest.get("guest_kernel_bpf_program_id")) is not int
                or guest["guest_kernel_bpf_program_id"] <= 0
                or guest.get("observed_before_vm_sigkill",True) is not True
                or not isinstance(guest.get("serial_sha256"),str)
                or re.fullmatch("[a-f0-9]{64}",guest["serial_sha256"]) is None):
                raise BindingError("real guest WASIp1 success receipt missing before VM termination")
            traceparents[tenant]=expected_trace
            host_aya_ids.append(preboot["kernel_bpf_program_id"])
        crash=actual_vm.get("vm_sigkill_fault")
        if (not isinstance(crash,dict)
            or crash.get("status")!="VM_SIGKILL_CGROUP_SUBTREE_DRAINED"
            or crash.get("cgroup_subtree_empty") is not True
            or crash.get("sigkill_executed") is not True
            or type(crash.get("sigkill_exit_code")) is not int
            or crash["sigkill_exit_code"]!=-9
            or crash.get("vm_cgroup_inode")!=actual_vm["cgroup_inode"]):
            raise BindingError("original admitted VM SIGKILL and cgroup drain absent")
        vmids.append(actual_vm["vm_id"])
        inodes.append(actual_vm["cgroup_inode"])
        bootids.append(actual_vm["host_boot_id"])
        requestids.append(rec["request_id"])
        manifest_hashes.append(rec["manifest_sha256"])
    if len(set(vmids))!=2 or len(set(inodes))!=2 or len(set(requestids))!=2 or bootids[0]!=bootids[1]:
        raise BindingError("reused VM, cgroup, replay ID or different kernel in one-host test")
    if require_wasi:
        a,b=(_manifest(manifests[t]) for t in ("a","b"))
        if (a["wasi"]["task_nonce"]==b["wasi"]["task_nonce"]
            or a["wasi"]["module_sha256"]==b["wasi"]["module_sha256"]
            or a["assets"]["guest_rootfs"]["sha256"]==b["assets"]["guest_rootfs"]["sha256"]):
            raise BindingError("WASI guest tasks must have separate nonces, modules and rootfs images")
    if actual.get("host_boot_id")!=bootids[0]:
        raise BindingError("VM host kernel identity changed")
    if require_wasi and len(set(traceparents.values()))!=2:
        raise BindingError("tenant VM traces collided")
    if require_wasi and len(set(host_aya_ids))!=2:
        raise BindingError("two tenant VM host Aya program IDs collided")
    return {
        "status":"BROKER_SO_PEERCRED_TWO_REAL_JAILED_VM_BOOT_BOUND",
        "schema_version":1,
        "admission_before_vm_launch":True,
        "real_kernel_so_peercred_authenticated_broker":True,
        "durable_sqlite_leases_matched":True,
        "two_distinct_tenant_request_ids":requestids,
        "two_manifest_sha256":manifest_hashes,
        "vm_ids":vmids,"cgroup_inodes":inodes,
        "kernel_boot_id":bootids[0],
        "both_guest_kernel_boots_observed":True,
        "both_vm_cgroups_drained_after_sigkill":True,
        "tenant_b_survived_tenant_a_crash":True,
        "wasi_executed_inside_guest":bool(require_wasi),
        "host_aya_egress_attached_before_guest_execution":bool(require_wasi),
        "guest_traceparent_propagated":bool(require_wasi),
        "tenant_traceparents":traceparents,
        "signed_wasi_manifest_verified":signed_manifests is not None,
        "production_approved":False,
        "stage_039_complete":False,
    }


async def _admit(ledger:broker.AdmissionLedger, approved_root:Path,
                 raw:dict[str,bytes], socket_dir:Path, now:int)->dict:
    receipts={}
    for tenant in ("a","b"):
        m=gate.parse_manifest(raw[tenant])
        service=ipc.BrokerIPC(socket_dir/("broker-"+tenant+".sock"),
            ledger,approved_root,
            ipc.PeerPolicy(
                allowed_uids=frozenset({os.geteuid()}),
                allowed_pairs=frozenset({(m["tenant_id"],m["agent_id"])})),
            clock=lambda:now)
        await service.start()
        try:
            req={
                "schema_version":1,"request_id":secrets.token_hex(16),
                "tenant_id":m["tenant_id"],"agent_id":m["agent_id"],
                "workload_id":m["workload_id"],"leader_epoch":1,
                "issued_unix_secs":now,"expires_unix_secs":now+300,
                "manifest_sha256":hashlib.sha256(raw[tenant]).hexdigest(),
            }
            msg={"version":1,"op":"reserve",
                 "request_b64":base64.b64encode(json.dumps(req).encode()).decode(),
                 "manifest_b64":base64.b64encode(raw[tenant]).decode()}
            payload=json.dumps(msg).encode()
            reader,writer=await asyncio.open_unix_connection(str(service.path))
            try:
                writer.write(struct.pack("!I",len(payload))+payload)
                await writer.drain()
                size=struct.unpack("!I",await asyncio.wait_for(reader.readexactly(4),timeout=3))[0]
                if not 0<size<=ipc.MAX_RESPONSE_BYTES:
                    raise BindingError("oversized local broker response")
                data=json.loads(await asyncio.wait_for(reader.readexactly(size),timeout=3))
            finally:
                writer.close();await writer.wait_closed()
            if data.get("status")!="RESERVED_NOT_EXECUTED":
                raise BindingError("kernel authenticated local broker denied reservation")
            receipts[tenant]=data
        finally:
            await service.close()
    return receipts


def run_with_live_broker(*,manifest_a:Path,manifest_b:Path,
                         approved_root:Path, ledger_dir:Path,
                         dual_arguments:dict,
                         require_guest_wasi:bool=False,
                         signed_manifest_a:Path|None=None,
                         signed_manifest_b:Path|None=None,
                         trusted_public_key:Path|None=None,
                         trusted_key_id:str="stage039-ci",
                         signer_mode:str="lab",
                         trusted_public_key_sha256:str|None=None)->dict:
    if (os.geteuid()!=0 or not ledger_dir.is_absolute()
        or ".." in ledger_dir.parts or ledger_dir.is_symlink()
        or ledger_dir.exists()):
        raise BindingError("ephemeral real KVM runner requires fresh root-private broker directory")
    manifests={"a":manifest_a.read_bytes(),"b":manifest_b.read_bytes()}
    signed_manifests,trusted_keys=load_signer_inputs(
        signed_manifest_a=signed_manifest_a,signed_manifest_b=signed_manifest_b,
        trusted_public_key=trusted_public_key,trusted_key_id=trusted_key_id,
        signer_mode=signer_mode,
        trusted_public_key_sha256=trusted_public_key_sha256)
    if require_guest_wasi and signed_manifests is None:
        raise BindingError("guest WASI acceptance requires signed manifests")
    if signed_manifests is not None:
        for tenant in ("a", "b"):
            try:
                signed = signed_manifest.verify(signed_manifests[tenant], trusted_keys)
            except signed_manifest.SignedManifestError as exc:
                raise BindingError("signed guest manifest rejected before admission") from exc
            if signed["payload"] != gate.parse_manifest(manifests[tenant]):
                raise BindingError("signed guest manifest differs from admitted manifest")
    ledger_dir.mkdir(mode=0o700,parents=False)
    ledger_dir.chmod(0o700)
    for raw in manifests.values():
        m=gate.parse_manifest(raw)
        if m["isolation"]["network"]!="none" or m["isolation"]["guest_egress"] is not False:
            raise BindingError("broker VM runtime may only execute pinned netless guest")
        gate.inspect_manifest(raw,approved_root)
    with broker.AdmissionLedger(ledger_dir/"admission.sqlite") as ledger:
        now=int(time.time())
        receipts=asyncio.run(_admit(ledger,approved_root,manifests,ledger_dir,now))
        rows={}
        for tenant,rec in receipts.items():
            row=ledger.conn.execute(
                "SELECT tenant_id,agent_id,workload_id,epoch,request_id,status "
                "FROM workload_state WHERE workload_id=?",(rec["workload_id"],)).fetchone()
            rows[tenant]=row
        verify_ledger_rows(rows,receipts)
        traceparents=None
        if require_guest_wasi:
            traceparents={tenant:traceparent.derive(rec["request_id"],
                         rec["manifest_sha256"],rec["tenant_id"])
                          for tenant,rec in receipts.items()}
            if set(traceparents)!={"a","b"} or len(set(traceparents.values()))!=2:
                raise BindingError("broker trace identity collision")
        try:
            actual=dual.execute(manifest_a=manifest_a,manifest_b=manifest_b,
                                approved_root=approved_root,
                                require_guest_wasi=require_guest_wasi,
                                traceparents=traceparents,**dual_arguments)
            verification=verify_binding(
                receipts,manifests,actual,require_wasi=require_guest_wasi,
                signed_manifests=signed_manifests,trusted_keys=trusted_keys)
        finally:
            for rec in receipts.values():
                if not ledger.release(workload_id=rec["workload_id"],
                                      request_id=rec["request_id"],
                                      epoch=rec["leader_epoch"]):
                    raise BindingError("lease cleanup failed after VM test")
        verification["durable_broker_leases_released_after_vm_crashes"]=True
        verification["signer_mode"]=signer_mode
        verification["operator_trust_anchor_pinned"]=signer_mode=="operator"
        if signer_mode=="operator":
            verification["operator_public_key_sha256"]=trusted_public_key_sha256
            verification["signed_manifest_sha256"]={
                tenant:hashlib.sha256(raw).hexdigest()
                for tenant,raw in signed_manifests.items()}
        verification["observed_at_utc"]=datetime.now(timezone.utc).isoformat()
        actual["broker_vm_binding"]=verification
        return actual


def main()->int:
    p=argparse.ArgumentParser(description=__doc__)
    for name in ("manifest-a","manifest-b","approved-root","ledger-dir",
                 "firecracker","jailer","chroot-base","aya-loader",
                 "aya-program","report"):
        p.add_argument("--"+name,required=True,type=Path)
    for name in ("firecracker-sha256","jailer-sha256","aya-loader-sha256",
                 "aya-program-sha256"):
        p.add_argument("--"+name,required=True)
    p.add_argument("--aya-seconds",type=int,default=15)
    p.add_argument("--timeout",type=int,default=90)
    p.add_argument("--require-guest-wasi",action="store_true",
                   help="wait for actual guest Wasmtime exit and WASIp1 fd_write before SIGKILL")
    p.add_argument("--signed-manifest-a", type=Path)
    p.add_argument("--signed-manifest-b", type=Path)
    p.add_argument("--trusted-public-key", type=Path)
    p.add_argument("--trusted-key-id", default="stage039-ci")
    p.add_argument("--signer-mode",choices=("lab","operator"),default="lab")
    p.add_argument("--trusted-public-key-sha256")
    p.add_argument("--ack-broker-two-jailed-vm-real-kvm",action="store_true")
    a=p.parse_args()
    if not a.ack_broker_two_jailed_vm_real_kvm:
        p.error("explicit privileged KVM broker-to-VM acceptance required")
    try:
        r=run_with_live_broker(
            manifest_a=a.manifest_a,manifest_b=a.manifest_b,
            approved_root=a.approved_root,ledger_dir=a.ledger_dir,
            require_guest_wasi=a.require_guest_wasi,
            signed_manifest_a=a.signed_manifest_a,
            signed_manifest_b=a.signed_manifest_b,
            trusted_public_key=a.trusted_public_key,
            trusted_key_id=a.trusted_key_id,
            signer_mode=a.signer_mode,
            trusted_public_key_sha256=a.trusted_public_key_sha256,
            dual_arguments=dict(firecracker=a.firecracker,
                 firecracker_sha=a.firecracker_sha256,
                 jailer=a.jailer,jailer_sha=a.jailer_sha256,
                 chroot_base=a.chroot_base,
                 aya_loader=a.aya_loader,aya_loader_sha=a.aya_loader_sha256,
                 aya_program=a.aya_program,aya_program_sha=a.aya_program_sha256,
                 aya_seconds=a.aya_seconds,timeout=a.timeout))
        gate.save_report(a.report,r)
    except (OSError,ValueError,asyncio.TimeoutError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":r["status"],
         "broker_vm_binding":r["broker_vm_binding"]["status"],
         "wasi_executed_inside_guest":bool(r["broker_vm_binding"]["wasi_executed_inside_guest"]),
         "production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
