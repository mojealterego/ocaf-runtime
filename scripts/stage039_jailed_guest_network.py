#!/usr/bin/env python3
"""Stage 0.39.12: ACTUAL jailed dual-tenant guest-originated UDP lab.

Real two simultaneous Firecracker jailed Linux guests, each isolated by a
dedicated cgroup-v2 inode and unique dropped uid/gid, send UDP via separate
TAPs into local host endpoints. nftables blocks A, B survives, A recovers.
A is SIGKILLed and B still communicates. Artifacts from this test do not
establish production approval, two-host failover, or Aya enforcement of
guest-originated traffic.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
import subprocess
import tempfile

import stage039_guest_network as gn
import stage039_guest_tc as guest_tc
import stage039_live_firecracker as fc
import stage039_vm_resources as vm_resources
import stage039_vm_crash as vm_crash
import stage039_microvm_gate as gate


class JailedNetError(ValueError):
    pass


def bound_nonce(nonce: str) -> str:
    if not isinstance(nonce, str) or gn.NONCE.fullmatch(nonce) is None:
        raise JailedNetError("checkpoint nonce must be 16 lowercase hexadecimal bytes")
    return nonce


def tap_setup_command(tap: str, ip: str, uid: int) -> list[str]:
    if (not isinstance(tap, str) or gn.TAP.fullmatch(tap) is None
            or ip not in {gn.TARGET["A"]["host"], gn.TARGET["B"]["host"]}
            or type(uid) is not int or uid not in (1002, 1003)):
        raise JailedNetError("untrusted dedicated TAP and jailed UID")
    return ["ip", "tuntap", "add", "dev", tap, "mode", "tap", "user", str(uid)]


def jailed_network_config(tenant: str, tap: str) -> dict:
    if tenant not in ("A","B") or gn.TAP.fullmatch(tap) is None or tap[4].lower()!=tenant.lower():
        raise JailedNetError("wrong tenant or malformed bounded TAP")
    return gn.build_lab_config(tenant,tap,
                               "/guest/rootfs.ext4","/guest/vmlinux.bin")


def verify_pair(a:dict,b:dict)->dict:
    if (not isinstance(a,dict) or not isinstance(b,dict)
            or a.get("vm_id") is None or b.get("vm_id") is None
            or a["vm_id"]==b["vm_id"]
            or a.get("tenant_id")!="tenant-a" or b.get("tenant_id")!="tenant-b"
            or not isinstance(a.get("cgroup_inode"),int)
            or not isinstance(b.get("cgroup_inode"),int)
            or a["cgroup_inode"]==b["cgroup_inode"]
            or any(x.get("firecracker_api_running") is not True
                   or x.get("guest_kernel_console_seen") is not True
                   or x.get("process_in_expected_cgroup") is not True
                   for x in (a,b))):
        raise JailedNetError("live two-tenant jailed guest kernel identity invalid")
    return {
        "two_live_jailer_tenants":True,
        "different_vm_ids":True,
        "different_kernel_cgroup_inodes":True,
        "guest_network_egress_tested":True,
        "production_approved":False,
    }


def assert_live(proc:subprocess.Popen, vm_id:str, cgroup:Path, expected_inode:int)->None:
    try:
        actual=fc.vm_egress.require_vm_running(proc,vm_id,cgroup)
    except (ValueError,OSError) as exc:
        raise JailedNetError("jailed VM died or changed kernel cgroup") from exc
    if type(expected_inode) is not int or actual!=expected_inode:
        raise JailedNetError("different running VM cgroup inode")


def _live_evidence(proc,vm_id,cgroup,log,sock,tenant)->dict:
    if not gn._serial_proven(log,tenant):
        raise JailedNetError("no authentic guest kernel + PID1 UDP signature")
    api=fc._api_state(sock)
    try:
        current=Path(f"/proc/{proc.pid}/cgroup").read_text(encoding="ascii")
        result=fc.validate_running(api,fc._bounded_console_evidence(log),
                                   current,vm_id,cgroup.name)
    except (OSError,fc.LiveFirecrackerError) as exc:
        raise JailedNetError("no validated jailed Firecracker kernel boot") from exc
    inode=cgroup.stat().st_ino
    assert_live(proc,vm_id,cgroup,inode)
    result.update({
        "tenant_id":"tenant-"+tenant.lower(),
        "vm_id":vm_id,
        "cgroup_inode":inode,
        "cgroup_name":cgroup.name,
        "firecracker_pid":proc.pid,
        "jailer_unprivileged_uid":1002 if tenant=="A" else 1003,
    })
    return result


def execute(*, firecracker:Path, firecracker_sha:str, jailer:Path, jailer_sha:str,
            jailer_base:Path, kernel:Path, kernel_sha:str, rootfs:Path,
            rootfs_sha:str, probe_source:Path, report_path:Path,
            nonce_override:str|None=None,
            tc_bpf_source:Path|None=None)->dict:
    if os.geteuid()!=0 or os.uname().machine!="x86_64":
        raise JailedNetError("privileged ephemeral KVM host required")
    fc._verified_binary(firecracker,firecracker_sha)
    fc._verified_binary(jailer,jailer_sha)
    if fc._matching_release(jailer,firecracker)!="1.17.0":
        raise JailedNetError("expected pinned Firecracker/jailer 1.17.0")
    fc._private_base(jailer_base)
    gn._check_pinned(kernel,kernel_sha)
    gn._check_pinned(rootfs,rootfs_sha)
    if probe_source.is_symlink() or not probe_source.is_file():
        raise JailedNetError("untrusted guest PID1 source")
    nonce=secrets.token_hex(8) if nonce_override is None else bound_nonce(nonce_override)
    suffix=secrets.token_hex(5)[:5]
    taps={"A":"oc39a"+suffix,"B":"oc39b"+suffix}
    nft="ocaf39"+secrets.token_hex(4)
    lab=Path(tempfile.mkdtemp(prefix="ocaf39-jguest-",dir="/tmp"))
    processes={}
    sockets={}
    cgroups={}
    devices=[]
    firewall=False
    tc_attached=False
    outputs={}
    observed={}
    try:
        for tenant in ("A","B"):
            uid=1002 if tenant=="A" else 1003
            init=lab/("init-"+tenant)
            image=lab/("rootfs-"+tenant+".ext4")
            gn._compile_guest(probe_source,init,tenant,nonce)
            digest=gn._stage_guest_rootfs(rootfs,image,init)
            image.chmod(0o400)
            vm_id="ocaf-"+secrets.token_hex(12)
            name="ocaf-stage039-vm-"+secrets.token_hex(8)+".service"
            cg=Path("/sys/fs/cgroup")/name
            root=jailer_base/firecracker.name/vm_id/"root"
            fc.validate_api_socket_path(jailer_base,firecracker.name,vm_id)
            cg.mkdir(mode=0o700)
            cgroups[tenant]=cg
            resource=vm_resources.apply_resource_limits(cg,256)
            fc._secure_staging_directories(root)
            fc._copy_pinned(kernel,root/"guest"/"vmlinux.bin",kernel_sha)
            fc._copy_pinned(image,root/"guest"/"rootfs.ext4",digest)
            config=jailed_network_config(tenant,taps[tenant])
            config_file=root/"vmconfig.json"
            fd=os.open(config_file,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o444)
            with os.fdopen(fd,"w",encoding="utf-8") as sink:
                json.dump(config,sink,separators=(",",":"))
                sink.flush();os.fsync(sink.fileno());os.fchmod(sink.fileno(),0o444)
            # TAP ownership is intentionally restricted to the VMM's dropped
            # UID; it is not an internet bridge or all-host routing.
            gn._run(tap_setup_command(taps[tenant],gn.TARGET[tenant]["host"],uid))
            devices.append(taps[tenant])
            gn._run(["ip","addr","add",gn.TARGET[tenant]["host"]+"/30",
                     "dev",taps[tenant]])
            gn._run(["ip","link","set","dev",taps[tenant],"up"])
            listener=socket.socket(socket.AF_INET,socket.SOCK_DGRAM)
            listener.bind((gn.TARGET[tenant]["host"],gn.TARGET[tenant]["port"]))
            sockets[tenant]=listener
            log=root.parent/"serial-console.log"
            f=os.open(log,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
            try:
                proc=subprocess.Popen(fc.build_jailer_command(
                    jailer,firecracker,vm_id,uid,uid,jailer_base,name),
                    stdout=f,stderr=subprocess.STDOUT,close_fds=True)
            finally:
                os.close(f)
            processes[tenant]=(proc,vm_id,log,root)
            outputs[tenant]={
                "tenant_id":"tenant-"+tenant.lower(),
                "tap":taps[tenant],
                "jailer_uid":uid,
                "vm_id":vm_id,
                "cgroup_inode":resource["vm_cgroup_inode"],
                "rootfs_sha256":digest,
                "guest_init_sha256":hashlib.sha256(init.read_bytes()).hexdigest(),
                "configured_vm_sha256":hashlib.sha256(config_file.read_bytes()).hexdigest(),
                "resources":resource,
            }

        baseline={}
        for tenant in ("A","B"):
            baseline[tenant]=gn._wait_new(sockets[tenant],tenant,nonce,-1,30)
            proc,vm_id,log,root=processes[tenant]
            observed[tenant]=_live_evidence(proc,vm_id,cgroups[tenant],
                                              log,root/"firecracker.socket",tenant)
        pair=verify_pair(observed["A"],observed["B"])
        for tenant in ("A","B"):
            proc,vm_id,log,root=processes[tenant]
            assert_live(proc,vm_id,cgroups[tenant],outputs[tenant]["cgroup_inode"])

        # Dual independent VMMs remain alive during the actual guest deny.
        firewall=True
        gn._nft_filter(nft,taps["A"])
        gn._drain(sockets["A"]);gn._drain(sockets["B"])
        during_b=gn._wait_new(sockets["B"],"B",nonce,baseline["B"],3)
        sockets["A"].settimeout(1.8)
        try:
            payload,address=sockets["A"].recvfrom(1024)
        except socket.timeout:
            pass
        else:
            gn.verify_guest_datagram(payload,address[0],"A",nonce)
            raise JailedNetError("jailed guest A bypassed nft rule")
        counter_text=gn._run(["nft","list","table","inet",nft])
        counters=[int(v) for v in re.findall(r"counter packets (\d+) bytes",counter_text)]
        if not counters or max(counters)==0:
            raise JailedNetError("zero denied packets from jailed A")
        gn._run(["nft","delete","table","inet",nft])
        firewall=False
        gn._drain(sockets["A"]);gn._drain(sockets["B"])
        rec_a=gn._wait_new(sockets["A"],"A",nonce,baseline["A"],6)
        rec_b=gn._wait_new(sockets["B"],"B",nonce,during_b,6)
        bpf_proof=None
        bpf_phase=None
        if tc_bpf_source is not None:
            # Proper host path for guest-originated packets is TC ingress
            # on the guest's TAP, not CgroupSkb of the host VMM process.
            obj=lab/"ocaf039-guest-deny.o"
            elf_sha=guest_tc.compile_bpf(source=tc_bpf_source,elf=obj)
            bpf_proof=guest_tc.attach_on_guest_a_tap(tap=taps["A"],elf=obj)
            tc_attached=True
            gn._drain(sockets["A"]);gn._drain(sockets["B"])
            b_during_tc=gn._wait_new(sockets["B"],"B",nonce,rec_b,5)
            sockets["A"].settimeout(1.8)
            a_received=False
            try:
                payload,origin=sockets["A"].recvfrom(1024)
            except socket.timeout:
                pass
            else:
                gn.verify_guest_datagram(payload,origin[0],"A",nonce)
                a_received=True
            guest_tc.detach_from_guest_a_tap(taps["A"])
            tc_attached=False
            gn._drain(sockets["A"]);gn._drain(sockets["B"])
            a_tc_recovered=gn._wait_new(sockets["A"],"A",nonce,rec_a,6)
            b_tc_recovered=gn._wait_new(sockets["B"],"B",nonce,b_during_tc,6)
            bpf_phase=guest_tc.validate_guest_tc_phases(
                before_a=rec_a,before_b=rec_b,
                during_a_received=a_received,during_b=b_during_tc,
                after_a=a_tc_recovered,after_b=b_tc_recovered)
            bpf_proof.update(bpf_phase)
            bpf_proof["bpf_elf_sha256"]=elf_sha
            rec_a=a_tc_recovered
            rec_b=b_tc_recovered
        a_proc,a_id,_,_=processes["A"]
        b_proc,b_id,_,_=processes["B"]
        a_kill=vm_crash.kill_and_verify(a_proc,a_id,cgroups["A"])
        assert_live(b_proc,b_id,cgroups["B"],outputs["B"]["cgroup_inode"])
        final_b=gn._wait_new(sockets["B"],"B",nonce,rec_b,6)
        b_kill=vm_crash.kill_and_verify(b_proc,b_id,cgroups["B"])
        result=gn.validate_phases(baseline["A"],baseline["B"],during_b,
                                  rec_a,rec_b,final_b)
        result.update({
            "status":"TWO_JAILED_GUEST_ORIGINATED_UDP_TENANTS_ISOLATED_AND_FAULTED",
            "schema_version":1,
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "checkpoint_nonce":nonce,
            "checkpoint_nonce_sha256":hashlib.sha256(nonce.encode("ascii")).hexdigest(),
            "host_boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "scope":"two simultaneous real jailed Firecracker guests, host nftables TAP deny/recovery, one ephemeral runner",
            "jailer_used_for_both_guests":True,
            "two_live_tenants_validated":pair,
            "tenant_a":outputs["A"],"tenant_b":outputs["B"],
            "live_jailed_guest_a":observed["A"],"live_jailed_guest_b":observed["B"],
            "tenant_a_vm_sigkill":a_kill,"tenant_b_vm_sigkill":b_kill,
            "nft_dropped_guest_a_packets":max(counters),
            "tc_ebpf_guest_tap":bpf_proof,
            "tc_kernel_ebpf_guest_network_tested":bpf_proof is not None,
            "aya_cgroup_guest_network_tested":False,
            "two_independent_hosts":False,
            "multi_host_failover_tested":False,
            "production_approved":False,
            "stage_039_complete":False,
        })
        gate.save_report(report_path,result)
        return result
    finally:
        for proc,vm_id,log,root in processes.values():
            if proc.poll() is None:
                proc.kill()
                try:proc.wait(timeout=5)
                except subprocess.TimeoutExpired:pass
        for listener in sockets.values():listener.close()
        if tc_attached:
            try:guest_tc.detach_from_guest_a_tap(taps["A"])
            except (OSError,ValueError):pass
        if firewall:
            subprocess.run(["nft","delete","table","inet",nft],
                           capture_output=True,check=False)
        for dev in reversed(devices):
            subprocess.run(["ip","link","del","dev",dev],capture_output=True,check=False)
        for cg in cgroups.values():
            try:cg.rmdir()
            except OSError:pass
        shutil.rmtree(lab,ignore_errors=True)


def main()->int:
    ap=argparse.ArgumentParser(description=__doc__)
    for arg in ("firecracker","jailer","jailer-base","kernel","rootfs",
                "probe-source","report"):
        ap.add_argument("--"+arg,type=Path,required=True)
    for arg in ("firecracker-sha256","jailer-sha256","kernel-sha256","rootfs-sha256"):
        ap.add_argument("--"+arg,required=True)
    ap.add_argument("--ack-privileged-dual-jailed-guest-network-lab",action="store_true")
    ap.add_argument("--checkpoint-nonce",help="public run-scoped nonce from separately verified handoff state")
    ap.add_argument("--tc-bpf-source",type=Path,
                    help="optionally compile real TC ingress kernel BPF classifier from checked-in C")
    a=ap.parse_args()
    if not a.ack_privileged_dual_jailed_guest_network_lab:
        ap.error("explicit privileged dual jailed guest network acknowledgement required")
    try:
        result=execute(firecracker=a.firecracker,
                       firecracker_sha=a.firecracker_sha256,
                       jailer=a.jailer,jailer_sha=a.jailer_sha256,
                       jailer_base=a.jailer_base,
                       kernel=a.kernel,kernel_sha=a.kernel_sha256,
                       rootfs=a.rootfs,rootfs_sha=a.rootfs_sha256,
                       probe_source=a.probe_source,report_path=a.report,
                       nonce_override=a.checkpoint_nonce,
                       tc_bpf_source=a.tc_bpf_source)
    except (OSError,ValueError,subprocess.SubprocessError) as exc:
        ap.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
