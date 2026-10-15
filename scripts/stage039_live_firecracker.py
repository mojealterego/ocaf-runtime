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
import time

import stage039_firecracker_spec as spec
import stage039_kvm_ioctl as kvm
import stage039_microvm_gate as gate

VALID_VM = re.compile(r"[a-z0-9][a-z0-9-]{0,63}\Z")
VALID_CGROUP = re.compile(r"ocaf-stage039-vm-[0-9a-f]{16}\.service\Z")
VERSION = re.compile(r"v([0-9]+\.[0-9]+\.[0-9]+)")
MAX_BINARY = 256 * 1024 * 1024


class LiveFirecrackerError(ValueError):
    pass


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
                      getattr(os, "O_NOFOLLOW", 0), 0o444)
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
        source.seek(max(0, size - 8192), os.SEEK_SET)
        tail = source.read(8192)
    return head.decode("utf-8", errors="replace") + "\n" + tail.decode("utf-8", errors="replace")


def _api_state(sockpath: Path) -> dict:
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(1)
        try:
            sock.connect(str(sockpath))
            sock.sendall(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            result = b""
            while len(result) < 32768:
                b = sock.recv(8192)
                if not b:
                    break
                result += b
        finally:
            sock.close()
        header, data = result.split(b"\r\n\r\n", 1)
        if b" 200 " not in header.split(b"\r\n", 1)[0]:
            return {}
        return json.loads(data)
    except (OSError, ValueError, UnicodeError):
        return {}


def run(*, manifest: Path, approved_root: Path, jailer: Path,
        jailer_sha: str, firecracker: Path, firecracker_sha: str,
        chroot_base: Path, uid: int, gid: int, timeout: int) -> dict:
    import secrets
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
    if _verified_binary(firecracker, parsed["assets"]["firecracker"]["sha256"]) != firecracker_sha:
        raise LiveFirecrackerError("Firecracker version is not the manifest's pinned binary")
    vm_id = "ocaf-" + secrets.token_hex(12)
    cgroup_name = "ocaf-stage039-vm-" + secrets.token_hex(8) + ".service"
    cgroup = Path("/sys/fs/cgroup") / cgroup_name
    root = chroot_base / firecracker.name / vm_id / "root"
    proc = None
    created = False
    try:
        cgroup.mkdir(mode=0o700)
        created = True
        _secure_staging_directories(root)
        # The jailer itself creates its chroot at this conventional location;
        # pre-staging is supported by Firecracker's jailer documentation.
        for asset, file in (("guest_kernel", "vmlinux.bin"),
                            ("guest_rootfs", "rootfs.ext4")):
            _copy_pinned(Path(parsed["assets"][asset]["path"]),
                         root / "guest" / file,
                         parsed["assets"][asset]["sha256"])
        staging = spec.build(raw, approved_root, root)
        config = root / "vmconfig.json"
        with os.fdopen(os.open(config, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o444),
                       "w", encoding="utf-8") as f:
            json.dump(staging["firecracker_config"], f, separators=(",", ":"))
            f.flush()
            os.fsync(f.fileno())
            os.fchmod(f.fileno(), 0o444)
        cmd = build_jailer_command(jailer, firecracker, vm_id, uid, gid,
                                   chroot_base, cgroup_name)
        log = root.parent / "serial-console.log"
        outfd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        try:
            proc = subprocess.Popen(cmd, stdout=outfd, stderr=subprocess.STDOUT,
                                    close_fds=True)
        finally:
            os.close(outfd)
        sockpath = root / "firecracker.socket"
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
                result.update({
                    "schema_version": 1,
                    "observed_at_utc": datetime.now(timezone.utc).isoformat(),
                    "kernel": os.uname().release,
                    "vm_id": vm_id, "guest_kernel_sha256": parsed["assets"]["guest_kernel"]["sha256"],
                    "guest_rootfs_sha256": parsed["assets"]["guest_rootfs"]["sha256"],
                    "firecracker_sha256": firecracker_sha, "jailer_sha256": jailer_sha,
                    "firecracker_release": release, "cgroup_inode": cgroup.stat().st_ino,
                    "serial_tail_sha256": hashlib.sha256(seen.encode()).hexdigest(),
                    "limit_note": "This confirms kernel boot console/API only. Aya egress must be proven separately.",
                })
                return result
            except LiveFirecrackerError as exc:
                last_rejection = str(exc)
                time.sleep(0.25)
        raise LiveFirecrackerError(
            "no matching real jailed Firecracker API+guest kernel+cgroup proof "
            f"within timeout; last check: {last_rejection}")
    finally:
        if proc is not None and proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=4)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=4)
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
    p.add_argument("--ack-real-kvm-jailer-boot", action="store_true")
    a = p.parse_args()
    if not a.ack_real_kvm_jailer_boot:
        p.error("acknowledge real jailed KVM guest launch with --ack-real-kvm-jailer-boot")
    try:
        result = run(manifest=a.manifest, approved_root=a.approved_root,
                     jailer=a.jailer, jailer_sha=a.jailer_sha256,
                     firecracker=a.firecracker, firecracker_sha=a.firecracker_sha256,
                     chroot_base=a.chroot_base, uid=a.uid, gid=a.gid,
                     timeout=a.timeout)
        gate.save_report(a.report, result)
    except (OSError, ValueError, LiveFirecrackerError) as exc:
        p.error(str(exc))
    print(json.dumps({"status": result["status"], "report": str(a.report),
                      "production_approved": False, "ebpf_attached": False}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
