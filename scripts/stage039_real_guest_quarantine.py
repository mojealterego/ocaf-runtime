#!/usr/bin/env python3
"""One-host KVM lab: signed host authorization stops an exact guest Wasmtime PID.

The guest Aya probe and host Aya cgroup probe must both observe real kernels.
An operator key generated for this one lab signs a bounded permit only after
both identities are checked. A second jailed tenant stays alive while A stops.
This is not an independent authority, durable-host or production acceptance.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

import stage039_guest_egress_transport as transport
import stage039_live_firecracker as live
import stage039_microvm_gate as gate
import stage039_vm_egress as vm_egress
from stage039_signed_manifest import canonical_payload
import stage039_signed_manifest as signed_manifest

ROOT = Path(__file__).resolve().parents[1]
CHALLENGE = re.compile(
    rb"OCAF039QC1\|tenant-a\|([0-9a-f]{32})\|"
    rb"([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\|"
    rb"([1-9][0-9]{0,8})\|([1-9][0-9]{0,19})\|"
    rb"([1-9][0-9]{0,9})\|([1-9][0-9]{0,9})\n\Z")
HOST_BPF_SHA = "992cd7a365067f52bce5afe5ca9758dad0e15ad9291b71991914040faaa55c95"


class QuarantineDenied(ValueError):
    pass


def parse_challenge(raw: bytes) -> dict:
    if not isinstance(raw, bytes) or len(raw) > 256:
        raise QuarantineDenied("unbounded guest quarantine challenge")
    match = CHALLENGE.fullmatch(raw)
    if match is None:
        raise QuarantineDenied("invalid guest quarantine challenge")
    nonce, boot, pid, ticks, program, count = match.groups()
    result = {"nonce": nonce.decode(), "guest_boot_id": boot.decode(),
              "pid": int(pid), "start_ticks": int(ticks),
              "guest_bpf_id": int(program), "event_count": int(count)}
    if result["pid"] <= 1 or result["event_count"] < 2:
        raise QuarantineDenied("unqualified target or syscall evidence")
    return result


def signed_permit(challenge: dict, host_bpf_id: int, evidence_sha: str,
                  key: Ed25519PrivateKey, now: int) -> tuple[bytes, bytes]:
    if (type(host_bpf_id) is not int or host_bpf_id <= 0
            or not re.fullmatch(r"[0-9a-f]{64}", evidence_sha)
            or type(now) is not int or now <= 0):
        raise QuarantineDenied("host policy evidence absent")
    payload = (f"OCAF039Q1|tenant-a|{challenge['nonce']}|{challenge['guest_boot_id']}|"
               f"{challenge['pid']}|{challenge['start_ticks']}|{challenge['guest_bpf_id']}|"
               f"{host_bpf_id}|{challenge['event_count']}|1500|{now + 120}|"
               f"{evidence_sha}\n").encode("ascii")
    if len(payload) >= 512:
        raise QuarantineDenied("signed permit exceeds guest bound")
    return payload, key.sign(payload)


def _recv(sock: socket.socket, size: int) -> bytes:
    data = bytearray()
    sock.settimeout(50)
    while len(data) < size:
        chunk = sock.recv(size - len(data))
        if not chunk:
            raise QuarantineDenied("truncated guest quarantine frame")
        data.extend(chunk)
    return bytes(data)


def _sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


class QuarantineBridge:
    guest_init_path = "/ocaf-q-init"
    needs_entropy = True

    def __init__(self, key: Ed25519PrivateKey, uid: int = 1002):
        self.key, self.uid = key, uid
        self.path: Path | None = None
        self.thread: threading.Thread | None = None
        self.error: BaseException | None = None
        self.challenge: dict | None = None
        self.permit_sha: str | None = None
        self.host_proof: dict | None = None
        self.host_ready = threading.Event()

    def config(self, root: Path) -> dict:
        if not root.is_absolute() or root.is_symlink():
            raise QuarantineDenied("untrusted jailed root")
        self.path = root / "q_5100"
        if len(os.fsencode(self.path)) > 100:
            raise QuarantineDenied("AF_UNIX path exceeded bound")
        # CID only routes traffic; SO_PEERCRED binds the VM identity.
        cid = 3 + int(hashlib.sha256(root.parent.name.encode()).hexdigest()[:6], 16)
        return {"guest_cid": cid, "uds_path": "/q"}

    def start(self, proc: subprocess.Popen, root: Path) -> None:
        if self.path != root / "q_5100":
            raise QuarantineDenied("vsock bridge root mismatch")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if proc.poll() is not None:
                raise QuarantineDenied("VM exited before quarantine listener bound")
            try:
                peer = transport.VmPeer.capture(tenant_id="tenant-a", uid=self.uid, pid=proc.pid)
            except transport.TransportDenied:
                time.sleep(.02)
                continue
            if "ocaf-stage039-vm-" in peer.cgroup:
                break
            time.sleep(.02)
        else:
            raise QuarantineDenied("VM process never entered private cgroup")

        def serve() -> None:
            listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try:
                assert self.path is not None
                listener.bind(str(self.path))
                os.chown(self.path, self.uid, -1)
                os.chmod(self.path, 0o600)
                listener.listen(1)
                listener.settimeout(40)
                conn, _ = listener.accept()
                with conn:
                    peer.verify(conn)
                    size = struct.unpack("!H", _recv(conn, 2))[0]
                    if not 0 < size <= 256:
                        raise QuarantineDenied("guest challenge size denied")
                    raw = _recv(conn, size)
                    self.challenge = parse_challenge(raw)
                    if not self.host_ready.wait(timeout=40) or self.host_proof is None:
                        raise QuarantineDenied("host Aya evidence unavailable")
                    h = hashlib.sha256()
                    h.update(raw)
                    h.update(canonical_payload(self.host_proof))
                    h.update(str(proc.pid).encode())
                    permit, sig = signed_permit(
                        self.challenge, self.host_proof["kernel_bpf_program_id"],
                        h.hexdigest(), self.key, int(time.time()))
                    conn.sendall(struct.pack("!H", len(permit)) + permit + sig)
                    self.permit_sha = hashlib.sha256(permit).hexdigest()
            except BaseException as exc:
                self.error = exc
            finally:
                listener.close()
                if self.path is not None:
                    self.path.unlink(missing_ok=True)

        self.thread = threading.Thread(target=serve, daemon=True)
        self.thread.start()
        deadline = time.monotonic() + 3
        while (self.path is not None and not self.path.exists()
               and self.error is None and time.monotonic() < deadline):
            time.sleep(.01)
        if self.path is None or not self.path.exists() or self.error is not None:
            raise QuarantineDenied("vsock listener did not bind")

    def prove(self, result: dict, proc: subprocess.Popen, vm_id: str,
              cgroup: Path, log: Path, peer_b: tuple) -> None:
        self.host_proof = vm_egress.prove_vm_host_cgroup_egress(
            proc=proc, vm_id=vm_id, cgroup=cgroup,
            loader=Path(result["approved_aya_loader"]),
            loader_sha=result["approved_aya_loader_sha"],
            program=Path(result["approved_aya_program"]),
            program_sha=HOST_BPF_SHA, seconds=12)
        self.host_ready.set()
        if self.thread is None:
            raise QuarantineDenied("quarantine listener missing")
        deadline = time.monotonic() + 35
        stopped = released = observed_during_stop = False
        while time.monotonic() < deadline:
            if self.error is not None:
                raise QuarantineDenied("vsock signed permit exchange failed") from self.error
            if proc.poll() is not None:
                raise QuarantineDenied("guest VM exited during quarantine")
            serial = live._bounded_console_evidence(log)
            if "OCAF039_Q_GUEST_DENIED=" in serial:
                raise QuarantineDenied("guest rejected authorized quarantine")
            stopped = "OCAF039_GUEST_QUARANTINE_STOPPED" in serial
            released = "OCAF039_GUEST_QUARANTINE_RELEASED" in serial
            if stopped:
                bproc, bid, bcgroup, binode = peer_b
                if vm_egress.require_vm_running(bproc, bid, bcgroup) != binode:
                    raise QuarantineDenied("tenant B did not survive guest A stop")
                if not released:
                    observed_during_stop = True
            if stopped and released and "OCAF039_Q_GUEST_AUTHORIZED_STOP_RELEASE_PASS" in serial:
                break
            time.sleep(.05)
        else:
            raise QuarantineDenied("guest signed quarantine did not complete")
        if not observed_during_stop or not self.challenge or not self.permit_sha or not self.host_proof:
            raise QuarantineDenied("missing correlated guest and host proof")
        challenge = self.challenge
        marker = (f"OCAF039_Q_AYA_ID={challenge['guest_bpf_id']} "
                  f"COUNT={challenge['event_count']} PID={challenge['pid']}")
        if serial.count(marker) != 1 or serial.count("OCAF039_GUEST_QUARANTINE_STOPPED") != 1 \
                or serial.count("OCAF039_GUEST_QUARANTINE_RELEASED") != 1:
            raise QuarantineDenied("guest Aya or stop/release serial mismatch")
        result["guest_quarantine"] = {
            "status": "REAL_KVM_SIGNED_GUEST_PID_STOP_RELEASE_WITH_PEER_SURVIVAL",
            "tenant_id": "tenant-a", "vm_id": vm_id,
            "guest_bpf_program_id": challenge["guest_bpf_id"],
            "guest_openat_event_count": challenge["event_count"],
            "guest_pid": challenge["pid"],
            "guest_boot_id": challenge["guest_boot_id"],
            "host_bpf_program_id": self.host_proof["kernel_bpf_program_id"],
            "signed_permit_sha256": self.permit_sha,
            "tenant_b_survived_stop": True,
            "one_host_only": True, "operator_key_provisioned_independently": False,
            "stage_039_complete": False, "production_approved": False,
        }

    def close(self) -> None:
        self.host_ready.set()
        if self.thread is not None:
            self.thread.join(timeout=.1)


def build_image(approved: Path, work: Path, public_key: bytes) -> Path:
    source = approved / "guest_rootfs_wasi_a"
    image = approved / "guest_rootfs_quarantine_a"
    shutil.copyfile(source, image)
    subprocess.run(["truncate", "-s", "+64M", str(image)], check=True)
    fsck = subprocess.run(["e2fsck", "-f", "-y", str(image)],
                          stdout=subprocess.DEVNULL)
    if fsck.returncode not in (0, 1):
        raise QuarantineDenied("copied guest ext4 check failed")
    subprocess.run(["resize2fs", str(image)], check=True, stdout=subprocess.DEVNULL)
    init = work / "ocaf-q-init"
    helper = work / "ocaf-guest-quarantine"
    for source_file, target, libraries in (
        ("guest_quarantine_init.c", init, []),
        ("quarantine.c", helper, ["-lcrypto", "-lz", "-lzstd", "-ldl", "-lpthread"]),
    ):
        subprocess.run(["gcc", "-std=c11", "-O2", "-static", "-Wall", "-Wextra",
                        "-Werror", str(ROOT / "stage039/guest-quarantine" / source_file),
                        "-o", str(target), *libraries], check=True)
    spin = work / "quarantine-spin.wasm"
    # Reuse the probe already built and separately pinned by the signed
    # tenant-A WASI manifest. Rebuilding here could silently change its ELF.
    probe = approved / "guest_openat_probe"
    prior = gate.parse_manifest((approved / "request-wasi-a.json").read_bytes())
    record = prior["assets"]["guest_openat_probe"]
    if (probe.is_symlink() or not probe.is_file()
            or probe.stat().st_uid != 0 or probe.stat().st_mode & 0o022
            or record["path"] != str(probe)
            or _sha(probe) != record["sha256"]):
        raise QuarantineDenied("quarantine Aya loader differs from signed tenant asset")
    subprocess.run(["wat2wasm", str(ROOT / "stage039/guest-quarantine/spin.wat"),
                    "-o", str(spin)], check=True)
    mount = work / "mount"
    mount.mkdir(mode=0o700)
    subprocess.run(["mount", "-o", "loop,rw", str(image), str(mount)], check=True)
    try:
        (mount / "run").mkdir(exist_ok=True)
        (mount / "opt/ocaf").mkdir(parents=True, exist_ok=True)
        (mount / "usr/bin").mkdir(parents=True, exist_ok=True)
        for path, data, mode in (
            (mount / "ocaf-q-init", init.read_bytes(), 0o555),
            (mount / "usr/bin/ocaf-guest-openat-probe",
             probe.read_bytes(), 0o555),
            (mount / "usr/bin/ocaf-guest-quarantine", helper.read_bytes(), 0o555),
            (mount / "opt/ocaf/quarantine-spin.wasm", spin.read_bytes(), 0o444),
            (mount / "opt/ocaf/quarantine-authority.pub", public_key, 0o444),
        ):
            path.write_bytes(data)
            path.chmod(mode)
        subprocess.run(["sync"], check=True)
    finally:
        subprocess.run(["umount", str(mount)], check=True)
    image.chmod(0o444)
    return image


def run(prefix: Path, report: Path) -> dict:
    if os.geteuid() != 0 or not str(prefix).startswith("/opt/ocaf039-real-") \
            or prefix.is_symlink() or not prefix.is_dir() or report.exists():
        raise QuarantineDenied("fresh privileged KVM prefix and report required")
    approved = prefix / "approved"
    if approved.stat().st_uid != 0 or approved.stat().st_mode & 0o077:
        raise QuarantineDenied("approved registry not private")
    policy_key = Ed25519PrivateKey.generate()
    pub = policy_key.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw)
    with tempfile.TemporaryDirectory(prefix="ocaf039-q-", dir="/tmp") as directory:
        work = Path(directory)
        image = build_image(approved, work, pub)
        base = json.loads((approved / "request.json").read_text())
        manifest_a = dict(base)
        manifest_a["workload_id"] = "ocaf-q-a"
        manifest_a["isolation"] = {**base["isolation"], "mem_mib": 1024}
        manifest_a["assets"] = dict(base["assets"])
        manifest_a["assets"]["guest_rootfs"] = {"path": str(image), "sha256": _sha(image)}
        manifest_b = dict(base)
        manifest_b["tenant_id"] = "tenant-b"
        manifest_b["agent_id"] = "agent-b"
        manifest_b["workload_id"] = "ocaf-q-b"
        path_a = approved / "request-quarantine-a.json"
        path_b = approved / "request-quarantine-b.json"
        for path, manifest in ((path_a, manifest_a), (path_b, manifest_b)):
            path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
            path.chmod(0o600)
            gate.inspect_manifest(path.read_bytes(), approved)
        # Distinct manifest-signer and guest policy keys; both are ephemeral lab keys.
        image_signer = Ed25519PrivateKey.generate()
        signed = []
        signer_public = image_signer.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        for key, manifest in (("a", manifest_a), ("b", manifest_b)):
            data = canonical_payload(manifest)
            signature = image_signer.sign(data)
            envelope = {"key_id": "stage039-q-lab", "payload": manifest,
                        "public_key": base64.b64encode(signer_public).decode(),
                        "signature": base64.b64encode(signature).decode()}
            signed_path = approved / f"signed-quarantine-{key}.json"
            signed_path.write_text(json.dumps(envelope, sort_keys=True) + "\n")
            signed_path.chmod(0o400)
            verified = signed_manifest.verify(
                signed_path.read_bytes(), {"stage039-q-lab": signer_public})
            if verified["payload"] != gate.parse_manifest(
                    (path_a if key == "a" else path_b).read_bytes()):
                raise QuarantineDenied("signed image manifest diverged")
            signed.append(hashlib.sha256(signature).hexdigest())
        bridge = QuarantineBridge(policy_key)
        b_ready = threading.Event()
        b_release = threading.Event()
        b_state: dict = {}

        def hook_b(result, proc, vm_id, cgroup):
            b_state["peer"] = (proc, vm_id, cgroup, result["cgroup_inode"])
            b_ready.set()
            if not b_release.wait(timeout=110):
                raise QuarantineDenied("tenant B live guard timed out")
            if vm_egress.require_vm_running(proc, vm_id, cgroup) != result["cgroup_inode"]:
                raise QuarantineDenied("tenant B VM no longer alive")

        def launch_b():
            return live.run(manifest=path_b, approved_root=approved,
                jailer=approved / "jailer", jailer_sha=_sha(approved / "jailer"),
                firecracker=approved / "firecracker",
                firecracker_sha=manifest_b["assets"]["firecracker"]["sha256"],
                chroot_base=prefix / "jailer", uid=1003, gid=1003, timeout=120,
                boot_hook=hook_b)

        with ThreadPoolExecutor(max_workers=1) as pool:
            future_b = pool.submit(launch_b)
            try:
                if not b_ready.wait(timeout=40):
                    raise QuarantineDenied("tenant B never entered live KVM guard")
                def hook_a(result, proc, vm_id, cgroup):
                    result["approved_aya_loader"] = str(approved / "aya-guard")
                    result["approved_aya_loader_sha"] = _sha(approved / "aya-guard")
                    result["approved_aya_program"] = str(approved / "aya_cgroup_device")
                    log = prefix / "jailer" / "firecracker" / vm_id / "serial-console.log"
                    bridge.prove(result, proc, vm_id, cgroup, log, b_state["peer"])
                a = live.run(manifest=path_a, approved_root=approved,
                    jailer=approved / "jailer", jailer_sha=_sha(approved / "jailer"),
                    firecracker=approved / "firecracker",
                    firecracker_sha=manifest_a["assets"]["firecracker"]["sha256"],
                    chroot_base=prefix / "jailer", uid=1002, gid=1002, timeout=110,
                    boot_hook=hook_a, vsock_bridge=bridge)
            finally:
                b_release.set()
            b = future_b.result(timeout=40)
        qa = a.get("guest_quarantine")
        if (not isinstance(qa, dict) or qa.get("tenant_b_survived_stop") is not True
                or a["vm_id"] == b["vm_id"] or a["cgroup_inode"] == b["cgroup_inode"]):
            raise QuarantineDenied("two independent VM quarantine proof absent")
        result = {"status": "ONE_HOST_TWO_JAILED_VMS_SIGNED_GUEST_QUARANTINE_LAB",
                  "tenant_a": qa, "tenant_b_vm_id": b["vm_id"],
                  "tenant_b_survived_a_quarantine": True,
                  "image_signature_sha256": signed, "guest_rootfs_sha256": _sha(image),
                  "operator_key_provisioned_independently": False,
                  "stage_039_complete": False, "production_approved": False}
        report.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
        return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--ack-real-kvm-quarantine", action="store_true")
    args = parser.parse_args()
    if not args.ack_real_kvm_quarantine:
        parser.error("explicit privileged KVM lab acknowledgment required")
    try:
        print(run(args.prefix, args.report)["status"])
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        detail = f"{type(exc).__name__}: {exc}"
        detail = detail.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
        print(f"::error title=stage039-quarantine::{detail[:500]}", file=sys.stderr)
        parser.error(str(exc))


if __name__ == "__main__":
    main()
