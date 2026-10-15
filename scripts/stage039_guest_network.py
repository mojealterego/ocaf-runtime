#!/usr/bin/env python3
"""0.39.11 REAL guest-originated UDP over separate Firecracker virtio-net TAPs.

Ephemeral CI only, root with KVM and CAP_NET_ADMIN, Firecracker launched in
dedicated lab mode WITHOUT jailer for this experiment (jailer separately
covered by stage039_live_firecracker.py). No NAT, no internet forwarding.
Kernel nftables blocks only guest A's TAP ingress to host sink; guest B
continues. Fault inject SIGKILL of VM A and prove VM B continues sending.
Never conflate this with Aya cgroup SKB, inter-host tests, or production GO.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import select
import shutil
import socket
import subprocess
import tempfile
import time

import stage039_live_firecracker as fc
import stage039_microvm_gate as gate

TARGET={
 "A":{"host":"198.19.38.1","guest":"198.19.38.2","port":23011,
      "mac":"06:00:c6:13:26:02"},
 "B":{"host":"198.19.38.5","guest":"198.19.38.6","port":23012,
      "mac":"06:00:c6:13:26:06"},
}
TAP=re.compile(r"oc39[ab][0-9a-f]{5}\Z")
NONCE=re.compile(r"[0-9a-f]{16}\Z")


class GuestNetworkError(ValueError):
    pass


def verify_guest_datagram(payload:bytes, source_ip:str, tenant:str, nonce:str)->int:
    if tenant not in TARGET or not isinstance(nonce,str) or NONCE.fullmatch(nonce) is None:
        raise GuestNetworkError("bad guest probe identity")
    if source_ip != TARGET[tenant]["guest"]:
        raise GuestNetworkError("unexpected guest IPv4 source address")
    if not isinstance(payload,bytes) or len(payload)>100:
        raise GuestNetworkError("invalid bounded guest UDP payload")
    try:
        parts=payload.decode("ascii").split(":")
    except UnicodeError as exc:
        raise GuestNetworkError("guest probe malformed ASCII") from exc
    if (len(parts)!=4 or parts[:3]!=["OCAF039",tenant,nonce]
            or re.fullmatch(r"(?:0|[1-9][0-9]{0,8})",parts[3]) is None):
        raise GuestNetworkError("incorrect tenant or nonce in guest packet")
    n=int(parts[3])
    if n>499:
        raise GuestNetworkError("guest packet sequence outside bounded test interval")
    return n


def build_lab_config(tenant:str,tap:str,rootfs:str,kernel:str)->dict:
    if (tenant not in TARGET or not isinstance(tap,str)
            or TAP.fullmatch(tap) is None or tap[4].lower()!=tenant.lower()
            or not all(isinstance(p,str) and Path(p).is_absolute()
                       and ".." not in Path(p).parts for p in (rootfs,kernel))):
        raise GuestNetworkError("untrusted TAP or guest image path")
    return {
      "boot-source":{
         "kernel_image_path":kernel,
         "boot_args":"console=ttyS0 reboot=k panic=1 pci=off root=/dev/vda ro init=/ocaf_probe_init",
      },
      "drives":[{
         "drive_id":"rootfs","path_on_host":rootfs,
         "is_root_device":True,"is_read_only":True,
      }],
      "machine-config":{"vcpu_count":1,"mem_size_mib":256,"smt":False},
      "network-interfaces":[{
         "iface_id":"eth0","host_dev_name":tap,
         "guest_mac":TARGET[tenant]["mac"],
      }],
    }


def validate_phases(baseline_a:int,baseline_b:int,during_b:int,
                    recovered_a:int,recovered_b:int,after_fault_b:int)->dict:
    if not (all(type(x) is int and 0<=x<500 for x in
            (baseline_a,baseline_b,during_b,recovered_a,recovered_b,after_fault_b))
            and during_b>baseline_b
            and recovered_a>baseline_a
            and recovered_b>during_b
            and after_fault_b>recovered_b):
        raise GuestNetworkError("live guest UDP negative, recovery or crash contract failed")
    return {
      "status":"REAL_GUEST_UDP_TAP_DENY_RECOVERY_AND_PEER_FAULT_PASSED",
      "guest_originated_network_tested":True,
      "tenant_a_denied_by_nftables_ingress":True,
      "tenant_b_uninterrupted":True,
      "tenant_a_recovered_after_firewall_removal":True,
      "tenant_b_survived_a_vm_sigkill":True,
      "aya_cgroup_guest_network_tested":False,
      "two_independent_hosts":False,
      "stage_039_complete":False,
      "production_approved":False,
    }


def _run(argv:list[str],timeout:int=20)->str:
    try:
        p=subprocess.run(argv,check=True,stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT,text=True,timeout=timeout)
    except (OSError,subprocess.SubprocessError) as exc:
        raise GuestNetworkError("authorized lab command failed: "+argv[0]) from exc
    return p.stdout


def _check_pinned(path:Path,expected_sha:str)->None:
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise GuestNetworkError("lab uses absolute regular pinned inputs only")
    if not isinstance(expected_sha,str) or re.fullmatch("[0-9a-f]{64}",expected_sha) is None:
        raise GuestNetworkError("invalid SHA-256 pin")
    sha=hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda:f.read(1<<20),b""):
            sha.update(chunk)
    if sha.hexdigest()!=expected_sha:
        raise GuestNetworkError("guest lab input SHA-256 mismatch")


def _compile_guest(source:Path,dst:Path,tenant:str,nonce:str)->None:
    if not source.is_file() or tenant not in TARGET or NONCE.fullmatch(nonce) is None:
        raise GuestNetworkError("invalid guest C source or test identity")
    _run(["gcc","-static","-Os","-s","-Wall","-Wextra","-Werror",
          "-DTENANT_NO="+("1" if tenant=="A" else "2"),
          '-DPROBE_NONCE="'+nonce+'"',
          str(source),"-o",str(dst)],timeout=60)
    if not dst.is_file() or dst.stat().st_size>4*1024*1024:
        raise GuestNetworkError("guest static init binary absent or oversized")


def _stage_guest_rootfs(source:Path,rootfs:Path,init:Path)->str:
    shutil.copyfile(source,rootfs)
    # Expand the PRIVATE copy to guarantee space for the static C executable.
    with rootfs.open("r+b") as f:
        f.truncate(f.seek(0,os.SEEK_END)+16*1024*1024)
    _run(["e2fsck","-pf",str(rootfs)],timeout=30)
    _run(["resize2fs",str(rootfs)],timeout=30)
    _run(["debugfs","-w","-R",f"write {init} /ocaf_probe_init",str(rootfs)])
    _run(["debugfs","-w","-R","set_inode_field /ocaf_probe_init mode 0100755",str(rootfs)])
    stat_result=_run(["debugfs","-R","stat /ocaf_probe_init",str(rootfs)])
    if not re.search(r"Mode:\s*0755",stat_result):
        raise GuestNetworkError("offline-injected guest init is not executable")
    return hashlib.sha256(rootfs.read_bytes()).hexdigest()


def _new_tap(dev:str,host_ip:str)->None:
    _run(["ip","tuntap","add","dev",dev,"mode","tap"])
    _run(["ip","addr","add",host_ip+"/30","dev",dev])
    _run(["ip","link","set","dev",dev,"up"])


def _nft_filter(table:str,tap_a:str)->None:
    if not re.fullmatch(r"ocaf39[a-f0-9]{8}",table):
        raise GuestNetworkError("invalid test-only firewall table name")
    _run(["nft","add","table","inet",table])
    _run(["nft","add","chain","inet",table,"input",
          "{ type filter hook input priority -50; policy accept; }"])
    _run(["nft","add","rule","inet",table,"input",
          "iifname",tap_a,"ip","daddr",TARGET["A"]["host"],
          "udp","dport",str(TARGET["A"]["port"]),"counter","drop"])


def _drain(sock:socket.socket)->None:
    sock.setblocking(False)
    try:
        for _ in range(1000):
            try: sock.recvfrom(1024)
            except BlockingIOError: break
    finally:
        sock.setblocking(True)


def _wait_new(sock:socket.socket,tenant:str,nonce:str,
              minimum:int,timeout:float)->int:
    stop=time.monotonic()+timeout
    while time.monotonic()<stop:
        sock.settimeout(min(0.25,max(0.01,stop-time.monotonic())))
        try:
            packet,origin=sock.recvfrom(1024)
        except socket.timeout:
            continue
        seq=verify_guest_datagram(packet,origin[0],tenant,nonce)
        if seq>minimum:
            return seq
    raise GuestNetworkError("no authenticated new guest-originated UDP from "+tenant)


def _serial_proven(log:Path,tenant:str)->bool:
    if not log.is_file(): return False
    s=fc._bounded_console_evidence(log)
    return ("Linux version " in s and
            "OCAF039_GUEST_NETWORK_INIT_TENANT_"+tenant in s)


def execute(*,firecracker:Path,firecracker_sha:str,kernel:Path,kernel_sha:str,
            rootfs:Path,rootfs_sha:str,probe_source:Path,
            report_path:Path)->dict:
    if os.geteuid()!=0 or os.uname().machine not in {"x86_64","amd64"}:
        raise GuestNetworkError("root x86_64 dedicated ephemeral Linux required")
    fc._verified_binary(firecracker,firecracker_sha)
    _check_pinned(kernel,kernel_sha)
    _check_pinned(rootfs,rootfs_sha)
    if not probe_source.is_file() or probe_source.is_symlink():
        raise GuestNetworkError("guest init C source must be a regular file")
    suffix=secrets.token_hex(5)
    names={"A":"oc39a"+suffix[:5],"B":"oc39b"+suffix[:5]}
    table="ocaf39"+secrets.token_hex(4)
    nonce=secrets.token_hex(8)
    root=Path(tempfile.mkdtemp(prefix="ocaf039-guest-network-",dir="/tmp"))
    created=[]
    firewall=False
    processes={}
    sockets={}
    outputs={}
    try:
        for tenant in ("A","B"):
            img=root/("guest-"+tenant+".ext4")
            init=root/("init-"+tenant)
            _compile_guest(probe_source,init,tenant,nonce)
            digest=_stage_guest_rootfs(rootfs,img,init)
            outputs[tenant]={"rootfs_sha256":digest,"guest_init_sha256":
                hashlib.sha256(init.read_bytes()).hexdigest(),
                "tap":names[tenant],"guest_ipv4":TARGET[tenant]["guest"],
                "tenant_label":"tenant-"+tenant.lower()}
            _new_tap(names[tenant],TARGET[tenant]["host"])
            created.append(names[tenant])
            sk=socket.socket(socket.AF_INET,socket.SOCK_DGRAM)
            sk.bind((TARGET[tenant]["host"],TARGET[tenant]["port"]))
            sockets[tenant]=sk
            config=build_lab_config(tenant,names[tenant],str(img),str(kernel))
            conf=root/("config-"+tenant+".json")
            conf.write_text(json.dumps(config,separators=(",",":"))+"\n")
            outputs[tenant]["vm_config_sha256"]=hashlib.sha256(conf.read_bytes()).hexdigest()
            log=root/("serial-"+tenant+".log")
            handle=log.open("wb")
            try:
                proc=subprocess.Popen([str(firecracker),"--api-sock",
                    str(root/("fc-"+tenant+".socket")),"--config-file",str(conf)],
                    stdout=handle,stderr=subprocess.STDOUT,close_fds=True)
            finally:
                handle.close()
            processes[tenant]=(proc,log)

        baseline={}
        for tenant in ("A","B"):
            baseline[tenant]=_wait_new(sockets[tenant],tenant,nonce,-1,25)
            if not _serial_proven(processes[tenant][1],tenant):
                raise GuestNetworkError("guest PID1 serial and Linux boot signatures absent")
        if any(proc.poll() is not None for proc,_ in processes.values()):
            raise GuestNetworkError("two guests not concurrently running")
        pid_a=processes["A"][0].pid
        pid_b=processes["B"][0].pid
        if pid_a==pid_b:
            raise GuestNetworkError("guest VMM process IDs collided")
        # Deny host delivery from guest A's TAP; guest B stays allowed.
        firewall=True
        _nft_filter(table,names["A"])
        _drain(sockets["A"])
        _drain(sockets["B"])
        during_b=_wait_new(sockets["B"],"B",nonce,baseline["B"],3)
        # No guest-A delivery is permitted for 1.8s after the nft ingress rule.
        sockets["A"].settimeout(1.8)
        try:
            payload,address=sockets["A"].recvfrom(1024)
        except socket.timeout:
            pass
        else:
            verify_guest_datagram(payload,address[0],"A",nonce)
            raise GuestNetworkError("guest A UDP bypassed tenant-specific nft DROP")
        nftlog=_run(["nft","list","table","inet",table])
        counters=[int(x) for x in re.findall(r"counter packets (\d+) bytes",nftlog)]
        if not counters or max(counters)<=0:
            raise GuestNetworkError("firewall DROP did not count actual guest packets")

        _run(["nft","delete","table","inet",table])
        firewall=False
        _drain(sockets["A"]);_drain(sockets["B"])
        recovered_a=_wait_new(sockets["A"],"A",nonce,baseline["A"],5)
        recovered_b=_wait_new(sockets["B"],"B",nonce,during_b,5)
        processes["A"][0].kill()
        if processes["A"][0].wait(timeout=5)!=-9:
            raise GuestNetworkError("VM A SIGKILL did not return -9")
        if processes["B"][0].poll() is not None:
            raise GuestNetworkError("guest B died when A VMM was killed")
        after_b=_wait_new(sockets["B"],"B",nonce,recovered_b,5)
        results=validate_phases(
            baseline["A"],baseline["B"],during_b,recovered_a,recovered_b,after_b)
        results.update({
            "schema_version":1,
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "scope":"temporary root-privileged Firecracker without jailer, two local TAPs, no NAT, guest PID1 UDP",
            "host_boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "guest_tenant_a":outputs["A"],"guest_tenant_b":outputs["B"],
            "guest_a_vmm_pid":pid_a,"guest_b_vmm_pid":pid_b,
            "firecracker_sha256":firecracker_sha,"kernel_sha256":kernel_sha,
            "input_rootfs_sha256":rootfs_sha,
            "guest_init_source_sha256":hashlib.sha256(probe_source.read_bytes()).hexdigest(),
            "nft_dropped_guest_a_packets":max(counters),
            "guest_a_sigkill_confirmed":True,
            "guest_b_alive_after_a_kill":True,
            "two_guest_vms_started_concurrently":True,
        })
        gate.save_report(report_path,results)
        return results
    finally:
        for proc,_ in processes.values():
            if proc.poll() is None:
                proc.kill()
                try:proc.wait(timeout=5)
                except subprocess.TimeoutExpired:pass
        for s in sockets.values():s.close()
        if firewall:
            subprocess.run(["nft","delete","table","inet",table],
                           check=False,capture_output=True)
        for dev in reversed(created):
            subprocess.run(["ip","link","del","dev",dev],check=False,capture_output=True)
        shutil.rmtree(root,ignore_errors=True)


def main()->int:
    p=argparse.ArgumentParser(description=__doc__)
    for key in ("firecracker","kernel","rootfs","probe-source","report"):
        p.add_argument("--"+key,type=Path,required=True)
    for key in ("firecracker-sha256","kernel-sha256","rootfs-sha256"):
        p.add_argument("--"+key,required=True)
    p.add_argument("--ack-root-kvm-tap-nft-ephemeral",action="store_true")
    a=p.parse_args()
    if not a.ack_root_kvm_tap_nft_ephemeral:
        p.error("explicit acknowledgement of temporary root TAP/nft lab required")
    try:
        result=execute(firecracker=a.firecracker,
            firecracker_sha=a.firecracker_sha256,
            kernel=a.kernel,kernel_sha=a.kernel_sha256,
            rootfs=a.rootfs,rootfs_sha=a.rootfs_sha256,
            probe_source=a.probe_source,report_path=a.report)
    except (OSError,ValueError,subprocess.SubprocessError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":result["status"],"report":str(a.report),
                      "production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
