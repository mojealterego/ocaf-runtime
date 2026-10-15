#!/usr/bin/env python3
"""0.39.14: kernel TC direct-action eBPF enforcement on a dedicated guest TAP.

BPF_PROG_TYPE_SCHED_CLS ingress is the correct hook for packets sent by
Firecracker guest virtio-net into the host TAP. CgroupSkb on the host VMM
process alone does not filter those packets. Never attach outside a unique
OCAF stage039 ephemeral TAP, never to the machine's physical devices.
"""
from __future__ import annotations
import hashlib
import re
from pathlib import Path

import stage039_guest_network as gn

class TcGuestError(ValueError):
    pass

def validate_tc_evidence(output:str,tap:str,program_id:int)->dict:
    if (not isinstance(output,str) or len(output)>8192 or
        not isinstance(tap,str) or gn.TAP.fullmatch(tap) is None or
        not tap.startswith("oc39a") or
        type(program_id) is not int or program_id<=0 or
        re.search(r"\bbpf\b",output) is None or
        re.search(r"\bid\s+"+str(program_id)+r"\b",output) is None):
        raise TcGuestError("no verified kernel BPF classifier on tenant-A guest TAP")
    return {
        "tc_bpf_program_attached":True,
        "kernel_bpf_program_id":program_id,
        "aya_cgroup_guest_network_tested":False,
        "kernel_guest_tap_egress_tested":True,
        "production_approved":False,
    }


def validate_guest_tc_phases(*,before_a:int,before_b:int,during_a_received:bool,
                             during_b:int,after_a:int,after_b:int)->dict:
    if (not all(type(x) is int and 0<=x<500 for x in
                (before_a,before_b,during_b,after_a,after_b)) or
        during_a_received is not False or
        not during_b>before_b or not after_a>before_a or
        not after_b>during_b):
        raise TcGuestError("guest-originated TC negative, peer or recovery checks failed")
    return {
        "guest_bpf_deny_recovery_proved":True,
        "tenant_a_guest_udp_denied_by_kernel_tc_ebpf":True,
        "tenant_b_guest_udp_remained_reachable":True,
        "tenant_a_guest_udp_recovered_after_tc_detach":True,
        "production_approved":False,
    }


def compile_bpf(*, source:Path, elf:Path)->str:
    if (not isinstance(source,Path) or not source.is_absolute()
        or source.is_symlink() or not source.is_file()
        or not isinstance(elf,Path) or not elf.is_absolute()
        or elf.exists() or source == elf):
        raise TcGuestError("untrusted eBPF source or destination")
    gn._run(["clang","-O2","-Wall","-Wextra","-Werror","-target","bpf",
             "-c",str(source),"-o",str(elf)],timeout=45)
    if not elf.is_file() or not 100<elf.stat().st_size<1048576:
        raise TcGuestError("BPF ELF build failed")
    digest=hashlib.sha256(elf.read_bytes()).hexdigest()
    return digest


def attach_on_guest_a_tap(*,tap:str,elf:Path)->dict:
    if (gn.TAP.fullmatch(tap) is None or not tap.startswith("oc39a")
        or not isinstance(elf,Path) or not elf.is_absolute()
        or not elf.is_file() or elf.is_symlink()):
        raise TcGuestError("TC eBPF attachment only permitted on authorized A TAP")
    # Always permit only the ephemeral TAP; use clsact for ingress from guest.
    gn._run(["tc","qdisc","add","dev",tap,"clsact"])
    try:
        gn._run(["tc","filter","add","dev",tap,"ingress",
                 "bpf","da","obj",str(elf),"sec","classifier"])
        record=gn._run(["tc","filter","show","dev",tap,"ingress"])
        hits=re.findall(r"\bid\s+([0-9]+)\b",record)
        if len(hits)!=1:
            raise TcGuestError("no unique kernel-loaded TC BPF program ID")
        identity=int(hits[0])
        result=validate_tc_evidence(record,tap,identity)
        result["bpf_elf_sha256"]=hashlib.sha256(elf.read_bytes()).hexdigest()
        return result
    except Exception:
        # Never leave a deny-all tc rule on test failure.
        detach_from_guest_a_tap(tap)
        raise


def detach_from_guest_a_tap(tap:str)->None:
    if not isinstance(tap,str) or gn.TAP.fullmatch(tap) is None or not tap.startswith("oc39a"):
        raise TcGuestError("unsafe BPF detach target")
    gn._run(["tc","qdisc","del","dev",tap,"clsact"])
    state=gn._run(["tc","filter","show","dev",tap,"ingress"])
    if re.search(r"\bbpf\b",state):
        raise TcGuestError("guest deny-all TC BPF detached without verified cleanup")
