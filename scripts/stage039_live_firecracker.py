#!/usr/bin/env python3
"""0.39.6: ACTUAL Firecracker jailer kernel-boot smoke on a Linux/KVM host.

No synthetic PASS: requires KVM_CREATE_VM, jailer, Firecracker API Running,
actual console kernel boot signature and expected cgroup v2 membership.
Never runs on the GitHub runner without KVM. Egress enforcement/guest
application attestation MUST be verified separately before release.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import socket
import stat
import subprocess
import sys
import time

import stage039_firecracker_spec as spec
import stage039_kvm_ioctl as kvm
import stage039_vm_egress as vm_egress
import stage039_preboot_aya as preboot_aya
import stage039_vm_resources as vm_resources
import stage039_vm_crash as vm_crash
import stage039_microvm_gate as gate
import stage039_guest_wasi as guest_wasi

VALID_VM = re.compile(r"[a-z0-9][a-z0-9-]{0,63}\Z")
VALID_CGROUP = re.compile(r"ocaf-stage039-vm-[0-9a-f]{16}\.service\Z")
VERSION = re.compile(r"v([0-9]+\.[0-9]+\.[0-9]+)")
MAX_BINARY = 256 * 1024 * 1024


class LiveFirecrackerError(ValueError):
    pass


def validate_api_socket_path(chroot_base: Path, binary_name: str,
                             vm_id: str) -> Path:
    """Linux sockaddr_un.sun_path allows at most 107 pathname bytes + NUL.

    The jailer exposes the guest's AF_UNIX API socket via a long host path.
    Oversized paths silently lose Firecracker API evidence if not rejected.
    """
    if (not isinstance(chroot_base, Path) or not chroot_base.is_absolute()
            or ".." in chroot_base.parts or chroot_base.is_symlink()
            or not isinstance(binary_name, str)
            or re.fullmatch(r"[a-z0-9][a-z0-9-]{0,63}", binary_name) is None
            or not isinstance(vm_id, str) or VALID_VM.fullmatch(vm_id) is None):
        raise LiveFirecrackerError("invalid jailed Firecracker API socket scope")
    path = chroot_base / binary_name / vm_id / "root" / "firecracker.socket"
    if len(os.fsencode(path)) > 107:
        raise LiveFirecrackerError("jailed Firecracker API sockaddr_un path exceeds 107 bytes")
    return path


def build_jailer_command(jailer: Path, firecracker: Path, vm_id: str,
                         uid: int, gid: int, chroot_base: Path,
                         cgroup_name: str) -> list[str]:
    if (not isinstance(vm_id, str) or VALID_VM.fullmatch(vm_id) is None or
            type(uid) is not int or type(gid) is not int or
            not (100 <= uid <= 65534 and 100 <= gid <= 65534)):
        raise LiveFirecrackerError("jailer requires valid VM ID and nonprivileged UID/GID")
    if (not isinstance(cgroup_name, str) or
            VALID_CGROUP.fullmatch(cgroup_name) is None):
        raise LiveFirecrackerError("jailer cgroup must be an OCAF dedicated scope")
    for path in (jailer, firecracker, chroot_base):
        if not path.is_absolute() or ".." in path.parts or path.is_symlink():
            raise LiveFirecrackerError("absolute trusted non-symlink paths required")
    return [
        str(jailer), "--id", vm_id, "--exec-file", str(firecracker),
        "--uid", str(uid), "--gid", str(gid),
        "--chroot-base-dir", str(chroot_base),
        "--cgroup-version", "2", "--parent-cgroup", cgroup_name,
        "--", "--api-sock", "/firecracker.socket",
        "--config-file", "/vmconfig.json",
    ]


def validate_running(api: dict, serial: str, cgroups: str, vm_id: str,
                     cgroup_name: str) -> dict:
    if (not isinstance(api, dict) or api.get("state") != "Running" or
            api.get("id") != vm_id):
        raise LiveFirecrackerError("Firecracker API did not report the expected running VM")
    if not re.search(r"(?:^|\n)(?:\[[ .0-9]+\]\s*)?Linux version [0-9]", serial):
        raise LiveFirecrackerError("no authentic guest kernel boot message on serial")
    if not isinstance(cgroups, str) or not any(
        x.startswith("0::/") and cgroup_name in x[4:].split("/")
        for x in cgroups.splitlines()):
        raise LiveFirecrackerError("Firecracker PID does not belong to dedicated cgroup")
    return {
        "status": "JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED",
        "firecracker_api_running": True,
        "guest_kernel_console_seen": True,
        "process_in_expected_cgroup": True,
        "ebpf_attached": False,
        "production_approved": False,
        "stage_039_complete": False,
        "guest_application_attested": False,
    }


def attest_running_rootfs_fd(proc: subprocess.Popen, staged: Path,
                             expected_sha256: str) -> dict:
    """Hash the actual open VMM rootfs inode, not only its pathname."""
    if proc.poll() is not None or type(proc.pid) is not int or proc.pid <= 1:
        raise LiveFirecrackerError("running VMM PID required for rootfs FD attestation")
    if (not staged.is_absolute() or staged.is_symlink() or not staged.is_file()
            or not re.fullmatch(r"[0-9a-f]{64}", expected_sha256)):
        raise LiveFirecrackerError("unsafe pinned staged rootfs")
    info = staged.stat()
    if info.st_nlink != 1 or info.st_mode & 0o022:
        raise LiveFirecrackerError("staged rootfs inode is mutable or multiply linked")
    fd_dir = Path(f"/proc/{proc.pid}/fd")
    try:
        descriptors = list(fd_dir.iterdir())
    except OSError as exc:
        raise LiveFirecrackerError("VMM descriptor directory unavailable") from exc
    if len(descriptors) > 4096:
        raise LiveFirecrackerError("VMM descriptor scan exceeded bound")
    matches = []
    verified_digest = None
    for descriptor in descriptors:
        if not descriptor.name.isdecimal():
            continue
        try:
            opened = descriptor.stat()
        except OSError:
            continue
        if opened.st_dev == info.st_dev and opened.st_ino == info.st_ino:
            # /proc/PID/fd/N resolves to the inode held by the live VMM.
            # Reopen it so the bytes we hash are the bytes the VMM can read.
            try:
                with descriptor.open("rb") as stream:
                    before = os.fstat(stream.fileno())
                    if ((before.st_dev, before.st_ino) != (info.st_dev, info.st_ino)
                            or not stat.S_ISREG(before.st_mode)
                            or before.st_nlink != 1 or before.st_mode & 0o022
                            or not 0 < before.st_size <= gate.ASSET_MAXIMUM_BYTES["guest_rootfs"]):
                        raise LiveFirecrackerError("VMM rootfs FD changed or is mutable")
                    digest = hashlib.file_digest(stream, "sha256").hexdigest()
                    after = os.fstat(stream.fileno())
                named = staged.stat(follow_symlinks=False)
                still_open = descriptor.stat()
            except OSError as exc:
                raise LiveFirecrackerError("VMM rootfs FD disappeared during attestation") from exc
            identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
            if (identity(before) != identity(after)
                    or identity(after) != identity(named)
                    or identity(after) != identity(still_open)):
                raise LiveFirecrackerError("VMM rootfs inode changed during attestation")
            if digest != expected_sha256:
                raise LiveFirecrackerError("running VMM rootfs FD digest differs from manifest")
            verified_digest = digest
            matches.append(int(descriptor.name))
    if not matches or proc.poll() is not None:
        raise LiveFirecrackerError("running VMM has no FD for pinned staged rootfs")
    return {"status": "RUNNING_VMM_ROOTFS_FD_MATCHES_PINNED_INODE",
            "rootfs_sha256": verified_digest, "rootfs_inode": info.st_ino,
            "rootfs_device": info.st_dev, "vmm_pid": proc.pid,
            "matching_fd_count": len(matches),
            "stage_039_complete": False, "production_approved": False}


def _verified_binary(path: Path, digest: str) -> str:
    if (not path.is_absolute() or ".." in path.parts or path.is_symlink()
            or not isinstance(digest, str) or
            re.fullmatch(r"[a-f0-9]{64}", digest) is None):
        raise LiveFirecrackerError("absolute pinned executable required")
    fd = None
    try:
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        st = os.fstat(fd)
        if (not stat.S_ISREG(st.st_mode) or st.st_nlink != 1 or
                st.st_mode & 0o022 or not st.st_mode & 0o111 or
                st.st_uid not in {0, os.geteuid()} or
                not 0 < st.st_size <= MAX_BINARY):
            raise LiveFirecrackerError("untrusted executable ownership, mode or size")
        h = hashlib.sha256()
        while chunk := os.read(fd, 1 << 20):
            h.update(chunk)
    except OSError as exc:
        raise LiveFirecrackerError("unreadable pinned executable") from exc
    finally:
        if fd is not None:
            os.close(fd)
    if h.hexdigest() != digest:
        raise LiveFirecrackerError("executable SHA-256 differs from pinned input")
    return digest


def _matching_release(jailer: Path, firecracker: Path) -> str:
    values = []
    for binary in (jailer, firecracker):
        try:
            p = subprocess.run([str(binary), "--version"], text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               check=False, timeout=5)
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise LiveFirecrackerError("cannot identify Firecracker release") from exc
        m = VERSION.search(p.stdout)
        if p.returncode != 0 or m is None:
            raise LiveFirecrackerError("Firecracker/jailer version unreadable")
        values.append(m.group(1))
    if values[0] != values[1]:
        raise LiveFirecrackerError("jailer and Firecracker release versions mismatch")
    return values[0]


def _private_base(path: Path) -> None:
    if (not path.is_absolute() or path.is_symlink() or not path.is_dir()
            or path.stat().st_uid != 0 or path.stat().st_mode & 0o077):
        raise LiveFirecrackerError("operator jailer base must be root-owned and 0700")



def _secure_staging_directories(root: Path) -> None:
    """Prevent the runner's umask from making the chroot mutable or unreadable.

    A 0002 umask makes intermediate mkdir(parents=True) directories 0775,
    which breaks fail-closed stage039_firecracker_spec.build(); a 0077 umask
    can make guest assets invisible to Firecracker's dropped UID.
    """
    if not root.is_absolute() or ".." in root.parts:
        raise LiveFirecrackerError("untrusted jailer staging path")
    guest = root / "guest"
    guest.mkdir(parents=True, mode=0o755)
    for directory, mode in ((root.parent.parent, 0o700),
                            (root.parent, 0o700), (root, 0o755),
                            (guest, 0o755)):
        if directory.is_symlink() or directory.stat().st_uid not in {0, os.geteuid()}:
            raise LiveFirecrackerError("untrusted jailer staging directory inode")
        directory.chmod(mode)


def _copy_pinned(src: Path, dest: Path, digest: str) -> None:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
    sfd = dfd = None
    try:
        sfd = os.open(src, flags)
        st = os.fstat(sfd)
        if (not stat.S_ISREG(st.st_mode) or st.st_mode & 0o022 or
                not 0 < st.st_size <= 8 * 1024 * 1024 * 1024):
            raise LiveFirecrackerError("untrusted pinned guest input")
        dfd = os.open(dest, os.O_WRONLY | os.O_CREAT | os.O_EXCL |
                      getattr(os, "O_NOFOLLOW", 0), 0o600)
        h = hashlib.sha256()
        while data := os.read(sfd, 1 << 20):
            h.update(data)
            offset = 0
            while offset < len(data):
                offset += os.write(dfd, data[offset:])
        os.fsync(dfd)
        if h.hexdigest() != digest:
            raise LiveFirecrackerError("guest input changed during secure staging")
        # umask 077 would silently turn 0444 into 0400. Once jailed,
        # Firecracker runs as the explicitly unprivileged UID, not root.
        os.fchmod(dfd, 0o444)
    except OSError as exc:
        raise LiveFirecrackerError("failed to pin guest input") from exc
    finally:
        for fd in (dfd, sfd):
            if fd is not None:
                os.close(fd)



def _bounded_console_evidence(log: Path) -> str:
    """Retain guest's early boot signature AND latest output in bounded RAM.

    Previous last-64KiB-only inspection lost the Linux version string
    after chatty Firecracker API logs appended tens of thousands of bytes.
    """
    with log.open("rb") as source:
        head = source.read(65536)
        source.seek(0, os.SEEK_END)
        size = source.tell()
        if size <= len(head):
            # Never count guest PID1/WASIp1 markers twice on short console logs.
            return head.decode("utf-8", errors="replace")
        source.seek(max(len(head), size - 8192), os.SEEK_SET)
        tail = source.read(8192)
    return head.decode("utf-8", errors="replace") + "\n" + tail.decode("utf-8", errors="replace")


def _api_state(sockpath: Path) -> dict:
    """Read a bounded, Content-Length-framed Firecracker HTTP response.

    Firecracker may keep the Unix socket open after sending its reply.
    Waiting for EOF would turn a valid Running response into a timeout.
    """
    max_header = 8192
    max_body = 24576
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
            sock.settimeout(1)
            sock.connect(str(sockpath))
            sock.sendall(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")

            received = bytearray()
            while b"\r\n\r\n" not in received:
                if len(received) > max_header:
                    return {}
                chunk = sock.recv(min(4096, max_header + 4 - len(received)))
                if not chunk:
                    return {}
                received.extend(chunk)

            header, body = bytes(received).split(b"\r\n\r\n", 1)
            if len(header) > max_header:
                return {}
            lines = header.split(b"\r\n")
            if not re.fullmatch(rb"HTTP/1\.[01] 200(?: [^\r\n]*)?", lines[0]):
                return {}

            lengths = []
            for line in lines[1:]:
                name, separator, value = line.partition(b":")
                if not separator or not name:
                    return {}
                key = name.strip().lower()
                if key == b"transfer-encoding":
                    return {}
                if key == b"content-length":
                    raw_length = value.strip()
                    if not raw_length or not raw_length.isdigit():
                        return {}
                    lengths.append(int(raw_length))
            if len(lengths) != 1 or lengths[0] > max_body or len(body) > lengths[0]:
                return {}

            while len(body) < lengths[0]:
                chunk = sock.recv(min(8192, lengths[0] - len(body)))
                if not chunk:
                    return {}
                body += chunk
            decoded = json.loads(body)
            return decoded if isinstance(decoded, dict) else {}
    except (OSError, ValueError, UnicodeError):
        return {}


def await_real_guest_wasi_execution(proc,serial_path:Path,*,seconds:int,
                                    expected_wasi:dict|None=None,tenant_id:str|None=None,
                                    expected_traceparent:str|None=None)->dict:
    """Wait for a REAL Linux guest PID1->Wasmtime process result BEFORE VM SIGKILL.

    Successful VM boot is not evidence of guest WASI execution. Guest output
    must include exactly one PID1 marker, one WASIp1 fd_write payload, and
    one zero-exit marker; fail closed on guest errors or a live guest timeout.
    """
    if (type(seconds) is not int or not 1<=seconds<=110
        or not isinstance(serial_path,Path) or not serial_path.is_absolute()
        or serial_path.is_symlink()):
        raise LiveFirecrackerError("bounded actual guest WASI wait required")
    deadline=time.monotonic()+seconds
    tail=""
    while time.monotonic()<deadline:
        if proc.poll() is not None:
            raise LiveFirecrackerError("jailed Firecracker exited before real guest WASI completion")
        try:
            tail=_bounded_console_evidence(serial_path)
        except OSError as exc:
            raise LiveFirecrackerError("real guest serial inaccessible") from exc
        if "OCAF039_GUEST_WASI_EXECUTION_FAILED" in tail:
            # Keep failure bounded: kernel logs are evidence, not an executable.
            raise LiveFirecrackerError("real guest Wasmtime child failed: "+repr(tail[-1200:]))
        if "OCAF039_GUEST_WASI_EXECUTION_PASS_EXIT_0" in tail:
            try:
                return guest_wasi.verify_console(tail.encode("utf-8"),
                                                 expected_wasi=expected_wasi,
                                                 tenant_id=tenant_id,
                                                 expected_traceparent=expected_traceparent)
            except guest_wasi.GuestWasiError as exc:
                raise LiveFirecrackerError("invalid real guest WASI zero-exit proof: " + str(exc)) from exc
        time.sleep(.25)
    raise LiveFirecrackerError("real guest WASI fd_write deadline expired; console tail: "+repr(tail[-800:]))


def run(*, manifest: Path, approved_root: Path, jailer: Path,
        jailer_sha: str, firecracker: Path, firecracker_sha: str,
        chroot_base: Path, uid: int, gid: int, timeout: int,
        aya_loader: Path | None = None, aya_loader_sha: str | None = None,
        aya_program: Path | None = None, aya_program_sha: str | None = None,
        aya_seconds: int = 15, crash_after_proof: bool = False,
        boot_hook=None, require_guest_wasi: bool = False,
        traceparent: str | None = None, vsock_bridge=None,
        preboot_host_aya: bool = False) -> dict:
    import secrets
    inputs = (aya_loader, aya_loader_sha, aya_program, aya_program_sha)
    if any(value is not None for value in inputs) and any(value is None for value in inputs):
        raise LiveFirecrackerError("Aya host-cgroup probe requires all four pinned inputs")
    if aya_loader is not None and not 12 <= aya_seconds <= 60:
        raise LiveFirecrackerError("Aya attach must use a bounded 12..60 second interval")
    if preboot_host_aya and (aya_loader is None or not require_guest_wasi or not crash_after_proof):
        raise LiveFirecrackerError("preboot host Aya requires pinned loader, guest WASI and VM teardown")
    if os.geteuid() != 0:
        raise LiveFirecrackerError("operator must run privileged jailer on an authorized KVM host")
    if not 10 <= timeout <= 120:
        raise LiveFirecrackerError("bounded live test timeout 10..120 required")
    host = kvm.collect()
    if host["status"] != "KVM_IOCTL_VM_CREATED_NOT_BOOTED":
        raise LiveFirecrackerError("real KVM_CREATE_VM or host prerequisites unavailable")
    _private_base(chroot_base)
    for binpath, digest in ((jailer, jailer_sha), (firecracker, firecracker_sha)):
        _verified_binary(binpath, digest)
    release = _matching_release(jailer, firecracker)
    raw = manifest.read_bytes()
    checked = gate.inspect_manifest(raw, approved_root)
    parsed = gate.parse_manifest(raw)
    if checked["status"] != "STATIC_ARTIFACTS_VERIFIED_ONLY":
        raise LiveFirecrackerError("unverified signed/pinned assets")
    if require_guest_wasi and "wasi" not in parsed:
        raise LiveFirecrackerError("signed tenant WASI workload binding required")
    if require_guest_wasi and (
        not guest_wasi.traceparent.is_valid(traceparent)):
        raise LiveFirecrackerError("broker-derived VM traceparent required")
    if _verified_binary(firecracker, parsed["assets"]["firecracker"]["sha256"]) != firecracker_sha:
        raise LiveFirecrackerError("Firecracker version is not the manifest's pinned binary")
    vm_id = "ocaf-" + secrets.token_hex(12)
    cgroup_name = "ocaf-stage039-vm-" + secrets.token_hex(8) + ".service"
    cgroup = Path("/sys/fs/cgroup") / cgroup_name
    root = chroot_base / firecracker.name / vm_id / "root"
    sockpath = validate_api_socket_path(chroot_base, firecracker.name, vm_id)
    proc = None
    aya_guard = None
    completed_result = None
    created = False
    try:
        cgroup.mkdir(mode=0o700)
        created = True
        # Host-side CPU/RSS/task bounds are kernel cgroup-v2 limits,
        # independent of Firecracker's guest-memory configuration.
        guest_mem_mib = parsed["isolation"]["mem_mib"]
        resource_plan = vm_resources.apply_resource_limits(cgroup, guest_mem_mib)
        _secure_staging_directories(root)
        # The jailer itself creates its chroot at this conventional location;
        # pre-staging is supported by Firecracker's jailer documentation.
        for asset, file in (("guest_kernel", "vmlinux.bin"),
                            ("guest_rootfs", "rootfs.ext4")):
            _copy_pinned(Path(parsed["assets"][asset]["path"]),
                         root / "guest" / file,
                         parsed["assets"][asset]["sha256"])
        staging = spec.build(raw, approved_root, root)
        if vsock_bridge is not None:
            staging["firecracker_config"]["vsock"] = vsock_bridge.config(root)
            boot_source = staging["firecracker_config"]["boot-source"]
            if boot_source["boot_args"] != spec.BASE_BOOT_ARGS:
                raise LiveFirecrackerError("vsock lab requires reviewed netless guest boot args")
            init_path = getattr(vsock_bridge, "guest_init_path", "/ocaf-egress-init")
            if init_path not in ("/ocaf-egress-init", "/ocaf-q-init"):
                raise LiveFirecrackerError("unreviewed vsock guest init path")
            boot_source["boot_args"] += " init=" + init_path
            if getattr(vsock_bridge, "needs_entropy", False):
                staging["firecracker_config"]["entropy"] = {}
        if require_guest_wasi:
            # Old minimal 4.14 guest has no early entropy and Wasmtime
            # may block indefinitely in getrandom(); attach real virtio-rng.
            staging["firecracker_config"] = spec.enable_guest_entropy_for_wasi(
                staging["firecracker_config"],traceparent)
        config = root / "vmconfig.json"
        with os.fdopen(os.open(config, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o444),
                       "w", encoding="utf-8") as f:
            json.dump(staging["firecracker_config"], f, separators=(",", ":"))
            f.flush()
            os.fsync(f.fileno())
            os.fchmod(f.fileno(), 0o444)
        if preboot_host_aya:
            aya_guard = preboot_aya.PrebootAya(
                cgroup=cgroup, loader=aya_loader, loader_sha=aya_loader_sha,
                program=aya_program, program_sha=aya_program_sha,
                lifetime_seconds=timeout+60)
            aya_guard.start()
            aya_guard.require_live()
        cmd = build_jailer_command(jailer, firecracker, vm_id, uid, gid,
                                   chroot_base, cgroup_name)
        log = root.parent / "serial-console.log"
        outfd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        try:
            proc = subprocess.Popen(cmd, stdout=outfd, stderr=subprocess.STDOUT,
                                    close_fds=True)
        finally:
            os.close(outfd)
        if vsock_bridge is not None:
            vsock_bridge.start(proc, root)
        deadline = time.monotonic() + timeout
        api = {}
        seen = ""
        last_rejection = "no API or console evidence yet"
        while time.monotonic() < deadline:
            if proc.poll() is not None:
                break
            api = _api_state(sockpath)
            try:
                seen = _bounded_console_evidence(log)
                cgroups = Path(f"/proc/{proc.pid}/cgroup").read_text()
            except OSError:
                cgroups = ""
            try:
                result = validate_running(api, seen, cgroups, vm_id, cgroup_name)
                result["running_rootfs_fd"] = attest_running_rootfs_fd(
                    proc, root / "guest/rootfs.ext4",
                    parsed["assets"]["guest_rootfs"]["sha256"])
                result.update({
                    "schema_version": 1,
                    "observed_at_utc": datetime.now(timezone.utc).isoformat(),
                    "kernel": os.uname().release,
                    "vm_id": vm_id, "tenant_id": parsed["tenant_id"],
                    "agent_id": parsed["agent_id"],
                    "workload_id": parsed["workload_id"],
                    "host_boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                    "guest_kernel_sha256": parsed["assets"]["guest_kernel"]["sha256"],
                    "guest_rootfs_sha256": parsed["assets"]["guest_rootfs"]["sha256"],
                    "firecracker_sha256": firecracker_sha, "jailer_sha256": jailer_sha,
                    "firecracker_release": release, "cgroup_inode": cgroup.stat().st_ino,
                    "serial_tail_sha256": hashlib.sha256(seen.encode()).hexdigest(),
                    "limit_note": "This confirms kernel boot console/API only. Aya egress must be proven separately.",
                })
                if require_guest_wasi:
                    remaining=max(1,min(95,int(deadline-time.monotonic())))
                    result["guest_wasi_execution"]=await_real_guest_wasi_execution(
                        proc,log,seconds=remaining,
                        expected_wasi=parsed["wasi"],tenant_id=parsed["tenant_id"],
                        expected_traceparent=traceparent)
                    result["guest_wasi_execution"]["observed_before_vm_sigkill"]=True
                    result["wasi"]=parsed["wasi"]
                    result["traceparent"]=traceparent
                if aya_guard is not None:
                    aya_guard.require_live()
                    result["vm_preboot_aya_egress"]=aya_guard.receipt()
                    result["vm_preboot_aya_egress"]["guest_wasi_observed_while_attached"]=True
                if boot_hook is not None:
                    # Test-only synchronization callback. The still-running
                    # process and cgroup inode are independently checked by
                    # the dual-tenant acceptance harness before proceeding.
                    boot_hook(result, proc, vm_id, cgroup)
                if aya_guard is not None:
                    aya_guard.require_live()
                if aya_loader is not None and aya_guard is None:
                    # Guest boot was checked while the original jailer process lives.
                    # Probe packets originate from a HOST process in the VM cgroup.
                    # This is not a proof that guest network traffic is isolated.
                    result["vm_host_cgroup_egress"] = (
                        vm_egress.prove_vm_host_cgroup_egress(
                            proc=proc, vm_id=vm_id, cgroup=cgroup,
                            loader=aya_loader, loader_sha=aya_loader_sha,
                            program=aya_program, program_sha=aya_program_sha,
                            seconds=aya_seconds))
                # Read real cgroup counters while Firecracker is alive,
                # including any effects accumulated by the Aya host probe.
                result["vm_resource_limits"] = vm_resources.audit_running_resource_limits(
                    cgroup, guest_mem_mib, resource_plan["vm_cgroup_inode"])
                if crash_after_proof:
                    if aya_guard is not None:
                        aya_guard.require_live()
                    result["vm_sigkill_fault"] = vm_crash.kill_and_verify(
                        proc, vm_id, cgroup, deadline_seconds=5)
                completed_result=result
                return result
            except LiveFirecrackerError as exc:
                last_rejection = str(exc)
                time.sleep(0.25)
        if require_guest_wasi:
            try:
                serial = _bounded_console_evidence(log)
                markers = [line[-240:] for line in serial.splitlines()
                           if "OCAF039_" in line or "Linux version " in line][:40]
            except (OSError, LiveFirecrackerError):
                markers = ["serial unavailable"]
            print("OCAF039_WASI_FAILURE_MARKERS=" + json.dumps(markers), file=sys.stderr)
        raise LiveFirecrackerError(
            "no matching real jailed Firecracker API+guest kernel+cgroup proof "
            f"within timeout; last check: {last_rejection}")
    finally:
        if vsock_bridge is not None:
            vsock_bridge.close()
        if proc is not None and proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=4)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=4)
        try:
            if aya_guard is not None:
                if completed_result is None:
                    aya_guard.abort()
                else:
                    completed_result["vm_preboot_aya_egress"].update(aya_guard.release())
        finally:
            if created:
                try:
                    cgroup.rmdir()
                except OSError as exc:
                    raise LiveFirecrackerError("live VM cgroup cleanup incomplete") from exc
        # Staged artifacts are kept for authorized forensic inspection.


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--manifest", type=Path, required=True)
    p.add_argument("--approved-root", type=Path, required=True)
    p.add_argument("--firecracker", type=Path, required=True)
    p.add_argument("--firecracker-sha256", required=True)
    p.add_argument("--jailer", type=Path, required=True)
    p.add_argument("--jailer-sha256", required=True)
    p.add_argument("--chroot-base", type=Path, required=True)
    p.add_argument("--uid", type=int, required=True)
    p.add_argument("--gid", type=int, required=True)
    p.add_argument("--timeout", type=int, default=45)
    p.add_argument("--report", type=Path, required=True)
    p.add_argument("--aya-loader", type=Path)
    p.add_argument("--aya-loader-sha256")
    p.add_argument("--aya-program", type=Path)
    p.add_argument("--aya-program-sha256")
    p.add_argument("--aya-seconds", type=int, default=15)
    p.add_argument("--ack-privileged-vm-cgroup-egress-probe", action="store_true")
    p.add_argument("--ack-real-kvm-jailer-boot", action="store_true")
    p.add_argument("--ack-vm-sigkill-fault", action="store_true",
                   help="SIGKILL only the newly launched and verified test VM")
    a = p.parse_args()
    if not a.ack_real_kvm_jailer_boot:
        p.error("acknowledge real jailed KVM guest launch with --ack-real-kvm-jailer-boot")
    vm_egress_inputs = (a.aya_loader, a.aya_loader_sha256,
                        a.aya_program, a.aya_program_sha256)
    if any(value is not None for value in vm_egress_inputs) or a.ack_privileged_vm_cgroup_egress_probe:
        if not a.ack_privileged_vm_cgroup_egress_probe or any(
                value is None for value in vm_egress_inputs):
            p.error("the live VM host-cgroup Aya probe requires all pinned Aya inputs and explicit acknowledgement")
    try:
        result = run(manifest=a.manifest, approved_root=a.approved_root,
                     jailer=a.jailer, jailer_sha=a.jailer_sha256,
                     firecracker=a.firecracker, firecracker_sha=a.firecracker_sha256,
                     chroot_base=a.chroot_base, uid=a.uid, gid=a.gid,
                     timeout=a.timeout, aya_loader=a.aya_loader,
                     aya_loader_sha=a.aya_loader_sha256,
                     aya_program=a.aya_program,
                     aya_program_sha=a.aya_program_sha256,
                     aya_seconds=a.aya_seconds,
                     crash_after_proof=a.ack_vm_sigkill_fault)
        gate.save_report(a.report, result)
    except (OSError, ValueError, LiveFirecrackerError) as exc:
        p.error(str(exc))
    print(json.dumps({"status": result["status"], "report": str(a.report),
                      "production_approved": False, "ebpf_attached": False}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
