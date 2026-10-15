#!/usr/bin/env python3
"""Stage 0.39.7: REAL kernel Aya/eBPF cgroup egress negative test.

Runs only on an authorized root-controlled Linux cgroup-v2/BPF host.
Attaches a DENY-ALL program to a freshly created dedicated test cgroup,
and probes loopback TCP:
  tenant A and B both connect before -> A denied, B allowed during ->
  A allowed after clean program detach.
No internet traffic, no general scanning, and no production cgroups.
No Firecracker VM boot is claimed by this script.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import select
import socket
import stat
import subprocess
import sys
import threading
import time
import secrets

import stage039_microvm_gate as gate

CGROUP_ROOT = Path("/sys/fs/cgroup")
BPF_MOUNT = Path("/sys/fs/bpf")
MAX_PROGRAM = 16 * 1024 * 1024
CGROUP_NAME = re.compile(r"ocaf-stage039-[ab]-[0-9a-f]{16}\.service\Z")


class AcceptanceError(RuntimeError):
    pass


def permitted_probe_cgroup(path: Path) -> bool:
    return (path.is_absolute() and path.parent == CGROUP_ROOT and
            CGROUP_NAME.fullmatch(path.name) is not None)


def sha256_file(path: Path) -> str:
    fd = None
    try:
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) |
                     getattr(os, "O_CLOEXEC", 0))
        st = os.fstat(fd)
        if not stat.S_ISREG(st.st_mode) or not 0 < st.st_size <= MAX_PROGRAM:
            raise AcceptanceError("invalid executable or program inode")
        d = hashlib.sha256()
        while True:
            data = os.read(fd, 1024 * 1024)
            if not data:
                break
            d.update(data)
        return d.hexdigest()
    except OSError as exc:
        raise AcceptanceError("cannot read trusted pinned program") from exc
    finally:
        if fd is not None:
            os.close(fd)


def verified_executable(path: Path, digest: str) -> str:
    try:
        st = path.lstat()
    except OSError as exc:
        raise AcceptanceError("Aya executable missing") from exc
    if (not path.is_absolute() or not stat.S_ISREG(st.st_mode) or
            st.st_mode & 0o022 or not st.st_mode & 0o111 or
            st.st_nlink != 1 or st.st_uid not in {0, os.geteuid()}):
        raise AcceptanceError("Aya binary ownership, mode or type unsafe")
    actual = sha256_file(path)
    if digest != actual:
        raise AcceptanceError("Aya executable SHA-256 pin mismatch")
    return actual


def preflight() -> dict:
    if os.geteuid() != 0 or sys.platform != "linux":
        raise AcceptanceError("real eBPF probe requires authorized Linux root")
    for path in (CGROUP_ROOT / "cgroup.controllers", BPF_MOUNT,
                 Path("/sys/kernel/btf/vmlinux")):
        if not path.exists():
            raise AcceptanceError("cgroup-v2/BPF/BTF host prerequisite missing: " + str(path))
    mountpoint = False
    for line in Path("/proc/mounts").read_text().splitlines():
        cols = line.split()
        if len(cols) > 2 and cols[1] == str(CGROUP_ROOT) and cols[2] == "cgroup2":
            mountpoint = True
    if not mountpoint:
        raise AcceptanceError("not a unified cgroup-v2 mount")
    return {"kernel": os.uname().release,
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "hostname": socket.gethostname()}


def validate_aya_event(event: dict, cgroup_inode: int) -> int:
    if (not isinstance(event, dict) or event.get("status") != "AYA_CGROUP_EGRESS_ATTACHED"
            or type(event.get("program_id")) is not int or event["program_id"] <= 0
            or type(event.get("cgroup_inode")) is not int or
            event["cgroup_inode"] != cgroup_inode
            or event.get("guest_booted") is not False
            or event.get("production_approved") is not False):
        raise AcceptanceError("Aya attachment not independently identified")
    return event["program_id"]


def evaluate_probes(*, before_a: bool, before_b: bool, attached: bool,
                    during_a: bool, during_b: bool, after_a: bool) -> dict:
    if not (before_a is True and before_b is True and attached is True and
            during_a is False and during_b is True and after_a is True):
        raise AcceptanceError("kernel egress negative/recovery contract failed")
    return {
        "status": "KERNEL_EBPF_EGRESS_ISOLATION_PASSED",
        "tenant_a_denied_during_attachment": True,
        "tenant_b_not_blocked": True,
        "tenant_a_recovered_after_detach": True,
        "microvm_booted": False,
        "production_approved": False,
        "stage_039_complete": False,
    }


# Child tries the loopback connection AFTER joining the selected cgroup.
# It reports every step; a cgroup join failure must never count as an
# observed egress block. Only TCP 127.0.0.1 is used by this test.
CHILD = r"""
import json,os,socket,sys
cg=sys.argv[1]
port=int(sys.argv[2])
try:
    with open(cg+'/cgroup.procs','w',encoding='ascii') as f:
        f.write(str(os.getpid()))
    joined=True
except OSError as exc:
    print(json.dumps(dict(joined=False,attempted=False,connected=False,
                          join_error=exc.errno)),flush=True)
    sys.exit(2)
try:
    conn=socket.create_connection(('127.0.0.1',port),timeout=2)
    conn.close()
    ok=True
except (OSError, TimeoutError):
    ok=False
print(json.dumps(dict(joined=True,attempted=True,connected=ok)),flush=True)
"""


def probe_connect(cgroup: Path, port: int) -> bool:
    if not permitted_probe_cgroup(cgroup) or not 1 <= port <= 65535:
        raise AcceptanceError("probe arguments outside local test scope")
    try:
        proc = subprocess.run([sys.executable, "-c", CHILD, str(cgroup), str(port)],
                              check=False, capture_output=True, text=True, timeout=6,
                              env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"})
        event = json.loads(proc.stdout.strip())
        if event.get("joined") is not True or event.get("attempted") is not True or                 type(event.get("connected")) is not bool or proc.returncode != 0:
            raise AcceptanceError("probe did not actually join cgroup and attempt network")
        return event["connected"]
    except (OSError, subprocess.TimeoutExpired, ValueError) as exc:
        raise AcceptanceError("loopback probe unavailable") from exc


@contextmanager
def local_tcp_server():
    service = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    service.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    service.bind(("127.0.0.1", 0))
    service.listen(12)
    service.settimeout(0.2)
    stop = threading.Event()

    def serve():
        while not stop.is_set():
            try:
                client, _ = service.accept()
                client.close()
            except socket.timeout:
                continue
            except OSError:
                break

    worker = threading.Thread(target=serve, daemon=True)
    worker.start()
    try:
        yield service.getsockname()[1]
    finally:
        stop.set()
        service.close()
        worker.join(timeout=2)


def await_attach(proc: subprocess.Popen, inode: int, timeout: int = 10) -> dict:
    expiry = time.monotonic() + timeout
    while time.monotonic() < expiry:
        if proc.poll() is not None:
            break
        timeout_left = max(0, min(0.5, expiry - time.monotonic()))
        r, _, _ = select.select([proc.stdout], [], [], timeout_left)
        if not r:
            continue
        line = proc.stdout.readline()
        if not line:
            continue
        try:
            event = json.loads(line)
            validate_aya_event(event, inode)
            return event
        except (ValueError, AcceptanceError):
            continue
    raise AcceptanceError("Aya did not confirm a real kernel cgroup attach")


def _real_egress_proof(loader: Path, loader_sha: str, program: Path,
                       program_sha: str, seconds: int) -> dict:
    host = preflight()
    verified_executable(loader, loader_sha)
    if sha256_file(program) != program_sha:
        raise AcceptanceError("eBPF ELF SHA-256 differs from pinned input")
    if not 12 <= seconds <= 60:
        raise AcceptanceError("attach interval must be 12–60 seconds")
    marker = secrets.token_hex(8)
    a = CGROUP_ROOT / ("ocaf-stage039-a-" + marker + ".service")
    b = CGROUP_ROOT / ("ocaf-stage039-b-" + marker + ".service")
    if not permitted_probe_cgroup(a) or not permitted_probe_cgroup(b):
        raise AcceptanceError("generated cgroup outside authorized scope")
    aya: subprocess.Popen | None = None
    created = []
    try:
        for path in (a, b):
            path.mkdir(mode=0o700)
            created.append(path)
        with local_tcp_server() as port:
            before_a, before_b = probe_connect(a, port), probe_connect(b, port)
            if not before_a or not before_b:
                raise AcceptanceError("baseline TCP connectivity missing")
            aya = subprocess.Popen([
                str(loader), "--program", str(program), "--sha256", program_sha,
                "--cgroup", str(a), "--seconds", str(seconds), "--ack-attach",
            ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
               env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"})
            attach = await_attach(aya, a.stat().st_ino)
            during_a, during_b = probe_connect(a, port), probe_connect(b, port)
            try:
                tail, err = aya.communicate(timeout=seconds + 8)
            except subprocess.TimeoutExpired as exc:
                raise AcceptanceError("Aya loader did not detach on schedule") from exc
            if aya.returncode != 0:
                raise AcceptanceError("Aya loader exited unsuccessfully")
            if not any(
                isinstance(d, dict) and d.get("status") == "AYA_CGROUP_EGRESS_DETACHED"
                and d.get("program_id") == attach["program_id"]
                for line in tail.splitlines()
                for d in [json.loads(line) if line.startswith("{") else None]
            ):
                raise AcceptanceError("Aya clean detached event not recorded")
            after_a = probe_connect(a, port)
        result = evaluate_probes(before_a=before_a, before_b=before_b,
                                 attached=True, during_a=during_a,
                                 during_b=during_b, after_a=after_a)
        result.update({
            "schema_version": 1, "timestamp_utc": datetime.now(timezone.utc).isoformat(),
            "scope": "dedicated two-test-cgroup loopback 127.0.0.1 only",
            "host": host, "kernel_bpf_program_id": attach["program_id"],
            "target_cgroup_inode": attach["cgroup_inode"],
            "loader_sha256": loader_sha, "bpf_elf_sha256": program_sha,
            "elapsed_attach_seconds": seconds,
            "note": "Guest boot, Firecracker jailer and two-host tenant isolation not exercised.",
        })
        return result
    finally:
        if aya is not None and aya.poll() is None:
            aya.kill()
            aya.communicate(timeout=3)
        for path in reversed(created):
            try:
                path.rmdir()
            except OSError:
                # Record cleanup errors rather than deleting another cgroup.
                raise AcceptanceError("dedicated test cgroup still occupied")


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--loader", required=True, type=Path)
    p.add_argument("--loader-sha256", required=True)
    p.add_argument("--program", required=True, type=Path)
    p.add_argument("--program-sha256", required=True)
    p.add_argument("--seconds", type=int, default=25)
    p.add_argument("--report", required=True, type=Path)
    p.add_argument("--ack-privileged-loopback-test", action="store_true",
                   help="authorize dedicated cgroup creation and local BPF policy")
    args = p.parse_args()
    if not args.ack_privileged_loopback_test:
        p.error("explicit --ack-privileged-loopback-test required")
    try:
        report = _real_egress_proof(args.loader, args.loader_sha256,
                                    args.program, args.program_sha256, args.seconds)
        gate.save_report(args.report, report)
    except (OSError, AcceptanceError, ValueError) as exc:
        p.error(str(exc))
    print(json.dumps({"status": report["status"], "report": str(args.report),
                      "microvm_booted": False, "production_approved": False}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
