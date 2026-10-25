#!/usr/bin/env python3
"""Hold a pinned host Aya cgroup egress link across one jailed VM lifetime.

This is a host-cgroup readiness and deny test. It cannot attest guest-originated
network policy; the guest remains netless in the signed workload manifest.
"""
from __future__ import annotations

import os
from pathlib import Path
import secrets
import subprocess

import stage039_live_egress as egress
import stage039_vm_egress as vm_egress


class PrebootAyaError(ValueError):
    pass


class PrebootAya:
    def __init__(self, *, cgroup: Path, loader: Path, loader_sha: str,
                 program: Path, program_sha: str, lifetime_seconds: int):
        if not 20 <= lifetime_seconds <= 180:
            raise PrebootAyaError("preboot Aya lifetime outside 20..180 seconds")
        self.cgroup = cgroup
        self.loader = loader
        self.loader_sha = loader_sha
        self.program = program
        self.program_sha = program_sha
        self.lifetime_seconds = lifetime_seconds
        self.proc: subprocess.Popen | None = None
        self.program_id: int | None = None
        self.cgroup_inode: int | None = None

    def start(self) -> dict:
        if self.proc is not None or not egress.permitted_probe_cgroup(self.cgroup):
            raise PrebootAyaError("fresh dedicated VM cgroup required")
        egress.preflight()
        egress.verified_executable(self.loader, self.loader_sha)
        if egress.sha256_file(self.program) != self.program_sha:
            raise PrebootAyaError("preboot Aya ELF digest mismatch")
        if self.cgroup.is_symlink() or self.cgroup.stat().st_uid != 0:
            raise PrebootAyaError("unsafe VM cgroup ownership")
        self.cgroup_inode = self.cgroup.stat().st_ino
        peer = egress.CGROUP_ROOT / (
            "ocaf-stage039-b-" + secrets.token_hex(8) + ".service")
        if not egress.permitted_probe_cgroup(peer):
            raise PrebootAyaError("unsafe peer probe cgroup")
        peer_created = False
        try:
            peer.mkdir(mode=0o700)
            peer_created = True
            with egress.local_tcp_server() as port:
                if not egress.probe_connect(self.cgroup, port) or not egress.probe_connect(peer, port):
                    raise PrebootAyaError("preboot baseline network probe failed")
                self.proc = subprocess.Popen(
                    [str(self.loader), "--program", str(self.program),
                     "--sha256", self.program_sha, "--cgroup", str(self.cgroup),
                     "--seconds", str(self.lifetime_seconds), "--ack-attach",
                     "--hold-stdin"],
                    stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE, text=True,
                    env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
                )
                attached = egress.await_attach(self.proc, self.cgroup_inode)
                self.program_id = attached["program_id"]
                if (egress.probe_connect(self.cgroup, port) is not False
                        or egress.probe_connect(peer, port) is not True
                        or self.proc.poll() is not None):
                    raise PrebootAyaError("preboot Aya host cgroup deny/peer isolation absent")
            return self.receipt()
        except Exception:
            self.abort()
            raise
        finally:
            if peer_created:
                peer.rmdir()

    def require_live(self) -> None:
        if self.proc is None or self.program_id is None or self.proc.poll() is not None:
            raise PrebootAyaError("host Aya link stopped before guest proof")
        if (self.cgroup.is_symlink() or self.cgroup.stat().st_ino != self.cgroup_inode):
            raise PrebootAyaError("host Aya target cgroup inode changed")

    def receipt(self) -> dict:
        if self.program_id is None or self.cgroup_inode is None:
            raise PrebootAyaError("unattached host Aya has no receipt")
        return {"status": "HOST_AYA_ATTACHED_BEFORE_FIRECRACKER_START",
                "kernel_bpf_program_id": self.program_id,
                "vm_cgroup_inode": self.cgroup_inode,
                "aya_loader_sha256": self.loader_sha,
                "bpf_elf_sha256": self.program_sha,
                "host_probe_denied_before_vm_start": True,
                "guest_network_egress_tested": False,
                "stage_039_complete": False, "production_approved": False}

    def release(self) -> dict:
        self.require_live()
        assert self.proc is not None and self.proc.stdin is not None
        self.proc.stdin.write("R")
        self.proc.stdin.flush()
        self.proc.stdin.close()
        self.proc.stdin = None
        out, err = self.proc.communicate(timeout=5)
        if (self.proc.returncode != 0
                or not vm_egress._confirmed_detach(out, self.program_id)):
            raise PrebootAyaError("host Aya kernel detach receipt absent: " + err[-300:])
        with egress.local_tcp_server() as port:
            if egress.probe_connect(self.cgroup, port) is not True:
                raise PrebootAyaError("host Aya cgroup recovery after detach absent")
        return {**self.receipt(), "aya_detached_after_vm_stop": True,
                "host_probe_recovered_after_vm_stop": True}

    def abort(self) -> None:
        if self.proc is not None and self.proc.poll() is None:
            self.proc.kill()
            self.proc.communicate(timeout=5)
