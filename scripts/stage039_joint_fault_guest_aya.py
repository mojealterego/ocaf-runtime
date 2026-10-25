#!/usr/bin/env python3
"""Real KVM negative: guest Aya missing while host Aya and another VM stay live.

The quarantine guest is gated over vsock until a kernel host Aya link denies a
host probe in its VM cgroup. Its guest Aya binary is deliberately absent from
a separately hashed, signed lab image. The expected guest result is DENY,
without a quarantine permit. This is only one fault slice of 0.39.25.
"""
from __future__ import annotations

import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import multiprocessing
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

import stage039_guest_egress_transport as transport
import stage039_live_firecracker as live
import stage039_microvm_gate as gate
import stage039_preboot_aya as preboot
import stage039_signed_manifest as signed_manifest
import stage039_vm_egress as vm_egress
import stage039_real_guest_quarantine as quarantine

ROOT = Path(__file__).resolve().parents[1]
HOST_BPF_SHA = "992cd7a365067f52bce5afe5ca9758dad0e15ad9291b71991914040faaa55c95"


class JointFaultDenied(ValueError):
    pass


def _broker_signer_worker(conn) -> None:
    """Separate signer: accepts a qualified challenge, signs only on ISSUE."""
    key = Ed25519PrivateKey.generate()
    try:
        conn.send("READY")
        raw, host_id = conn.recv()
        challenge = quarantine.parse_challenge(raw)
        if type(host_id) is not int or host_id <= 0:
            raise JointFaultDenied("host BPF evidence missing")
        conn.send(("CHALLENGE_ACCEPTED", challenge["guest_bpf_id"]))
        if conn.recv() == "ISSUE":
            proof_hash = hashlib.sha256(raw + str(host_id).encode()).hexdigest()
            conn.send(quarantine.signed_permit(
                challenge, host_id, proof_hash, key, int(time.time())))
    finally:
        conn.close()


def vm_cgroup_from_peer(peer: transport.VmPeer) -> Path:
    names = [component for line in peer.cgroup.splitlines()
             if line.startswith("0::/")
             for component in line[4:].split("/")
             if vm_egress.VM_CGROUP.fullmatch(component)]
    if len(names) != 1:
        raise JointFaultDenied("VM cgroup identity absent")
    path = Path("/sys/fs/cgroup") / names[0]
    if path.is_symlink() or not path.is_dir() or path.stat().st_uid != 0:
        raise JointFaultDenied("VM cgroup path unsafe")
    return path


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def build_fault_image(approved: Path, work: Path, *, broker_fault: bool = False) -> Path:
    source = approved / "guest_rootfs_quarantine_a"
    if not source.is_file() or source.is_symlink() or source.stat().st_uid != 0:
        raise JointFaultDenied("verified quarantine guest image missing")
    prior = gate.parse_manifest((approved / "request-quarantine-a.json").read_bytes())
    if (prior["assets"]["guest_rootfs"] != {"path": str(source), "sha256": sha(source)}
            or gate.inspect_manifest((approved / "request-quarantine-a.json").read_bytes(),
                                    approved)["status"] != "STATIC_ARTIFACTS_VERIFIED_ONLY"):
        raise JointFaultDenied("quarantine source image differs from pinned lab manifest")
    image = approved / ("guest_rootfs_broker_fault_a" if broker_fault else
                        "guest_rootfs_joint_fault_a")
    if image.exists():
        raise JointFaultDenied("fresh joint fault image required")
    shutil.copyfile(source, image)
    wrapper = work / "fault-gate-init"
    subprocess.run(["gcc", "-std=c11", "-O2", "-static", "-Wall", "-Wextra",
                    "-Werror", str(ROOT / "stage039/guest-quarantine/fault_gate_init.c"),
                    "-o", str(wrapper)], check=True)
    mount = work / "mount"
    mount.mkdir(mode=0o700)
    subprocess.run(["mount", "-o", "loop,rw", str(image), str(mount)], check=True)
    try:
        original = mount / "ocaf-q-init"
        renamed = mount / "ocaf-q-run"
        probe = mount / "usr/bin/ocaf-guest-openat-probe"
        if not original.is_file() or not probe.is_file() or renamed.exists():
            raise JointFaultDenied("fault image lacks original Aya guest setup")
        original.rename(renamed)
        original.write_bytes(wrapper.read_bytes())
        original.chmod(0o555)
        if not broker_fault:
            probe.unlink()
        subprocess.run(["sync"], check=True)
    finally:
        subprocess.run(["umount", str(mount)], check=True)
    image.chmod(0o444)
    return image


def signed_lab_manifest(manifest: dict, approved: Path,
                        *, broker_fault: bool = False) -> Path:
    key = Ed25519PrivateKey.generate()
    public = key.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw)
    body = signed_manifest.canonical_payload(manifest)
    signature = key.sign(body)
    envelope = {"key_id": "stage039-joint-fault-lab",
                "public_key": base64.b64encode(public).decode(),
                "signature": base64.b64encode(signature).decode(),
                "payload": manifest}
    signed_path = approved / ("signed-broker-fault-a.json" if broker_fault else
                              "signed-joint-fault-a.json")
    signed_path.write_text(json.dumps(envelope, sort_keys=True) + "\n")
    signed_path.chmod(0o400)
    verified = signed_manifest.verify(signed_path.read_bytes(),
                                     {envelope["key_id"]: public})
    if verified["payload"] != manifest:
        raise JointFaultDenied("fault image signed manifest mismatch")
    manifest_path = approved / ("request-broker-fault-a.json" if broker_fault else
                                "request-joint-fault-a.json")
    manifest_path.write_bytes(body)
    manifest_path.chmod(0o400)
    gate.inspect_manifest(manifest_path.read_bytes(), approved)
    return manifest_path


class AyaFaultBridge:
    guest_init_path = "/ocaf-q-init"
    needs_entropy = True

    def __init__(self, approved: Path, *, broker_fault: bool = False):
        self.approved = approved
        self.broker_fault = broker_fault
        self.path: Path | None = None
        self.thread: threading.Thread | None = None
        self.error: BaseException | None = None
        self.guard: preboot.PrebootAya | None = None
        self.proc: subprocess.Popen | None = None
        self.ready = threading.Event()
        self.proved = False
        self.receipt: dict | None = None
        self.permit_issued = False
        self.broker_sigkill_observed = False
        self.guest_challenge: dict | None = None
        self.broker_proc = None

    def config(self, root: Path) -> dict:
        if not root.is_absolute() or root.is_symlink():
            raise JointFaultDenied("untrusted jailed root")
        self.path = root / "fault_5100"
        if len(os.fsencode(self.path)) > 100:
            raise JointFaultDenied("AF_UNIX path exceeded bound")
        cid = 3 + int(hashlib.sha256(root.parent.name.encode()).hexdigest()[:6], 16)
        return {"guest_cid": cid, "uds_path": "/fault"}

    def start(self, proc: subprocess.Popen, root: Path) -> None:
        if self.path != root / "fault_5100":
            raise JointFaultDenied("vsock fault root mismatch")
        self.proc = proc
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if proc.poll() is not None:
                raise JointFaultDenied("VM exited before fault listener bound")
            try:
                peer = transport.VmPeer.capture(tenant_id="tenant-a", uid=1002,
                                                pid=proc.pid)
            except transport.TransportDenied:
                time.sleep(.02)
                continue
            if "ocaf-stage039-vm-" in peer.cgroup:
                break
            time.sleep(.02)
        else:
            raise JointFaultDenied("VM process never entered private cgroup")
        cgroup = vm_cgroup_from_peer(peer)

        def serve() -> None:
            listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try:
                assert self.path is not None
                listener.bind(str(self.path))
                os.chown(self.path, 1002, -1)
                os.chmod(self.path, 0o600)
                listener.listen(2 if self.broker_fault else 1)
                listener.settimeout(40)
                conn, _ = listener.accept()
                with conn:
                    peer.verify(conn)
                    conn.settimeout(5)
                    if conn.recv(2) != b"R":
                        raise JointFaultDenied("guest fault gate frame denied")
                    loader = self.approved / "aya-guard"
                    program = self.approved / "aya_cgroup_device"
                    self.guard = preboot.PrebootAya(
                        cgroup=cgroup, loader=loader, loader_sha=sha(loader),
                        program=program, program_sha=HOST_BPF_SHA,
                        lifetime_seconds=120)
                    self.receipt = self.guard.start()
                    self.guard.require_live()
                    conn.sendall(b"G")
                    self.ready.set()
                if self.broker_fault:
                    conn, _ = listener.accept()
                    with conn:
                        peer.verify(conn)
                        prefix = conn.recv(2, socket.MSG_WAITALL)
                        if len(prefix) != 2:
                            raise JointFaultDenied("guest Aya challenge frame absent")
                        length = struct.unpack("!H", prefix)[0]
                        if not 0 < length <= 256:
                            raise JointFaultDenied("unbounded guest Aya challenge")
                        raw = conn.recv(length, socket.MSG_WAITALL)
                        if len(raw) != length:
                            raise JointFaultDenied("truncated guest Aya challenge")
                        self.guest_challenge = quarantine.parse_challenge(raw)
                        self.guard.require_live()
                        parent, child = multiprocessing.get_context("spawn").Pipe()
                        signer = multiprocessing.get_context("spawn").Process(
                            target=_broker_signer_worker, args=(child,))
                        signer.start()
                        child.close()
                        self.broker_proc = signer
                        try:
                            if not parent.poll(10) or parent.recv() != "READY":
                                raise JointFaultDenied("broker signer process not ready")
                            parent.send((raw, self.receipt["kernel_bpf_program_id"]))
                            if (not parent.poll(10) or parent.recv() !=
                                    ("CHALLENGE_ACCEPTED", self.guest_challenge["guest_bpf_id"])):
                                raise JointFaultDenied("broker did not accept qualified guest challenge")
                            signer.kill()
                            signer.join(timeout=5)
                            if signer.exitcode is None or signer.exitcode >= 0:
                                raise JointFaultDenied("broker process SIGKILL not observed")
                            self.broker_sigkill_observed = True
                            # Close vsock without any permit frame. Guest PID1 must deny.
                        finally:
                            parent.close()
                            if signer.is_alive():
                                signer.kill()
                                signer.join(timeout=5)
                # No signed permit endpoint remains available after the fault.
            except BaseException as exc:
                self.error = exc
                self.ready.set()
            finally:
                listener.close()
                if self.path is not None:
                    self.path.unlink(missing_ok=True)

        self.thread = threading.Thread(target=serve, daemon=True)
        self.thread.start()
        deadline = time.monotonic() + 3
        while self.path is not None and not self.path.exists() and self.error is None \
                and time.monotonic() < deadline:
            time.sleep(.01)
        if self.path is None or not self.path.exists() or self.error is not None:
            raise JointFaultDenied("joint fault listener did not bind")

    def prove(self, result: dict, proc: subprocess.Popen, vm_id: str,
              cgroup: Path, log: Path, peer_b: tuple) -> None:
        if not self.ready.wait(timeout=40) or self.error is not None:
            raise JointFaultDenied("host Aya fault gate did not attach") from self.error
        if self.guard is None or self.receipt is None:
            raise JointFaultDenied("host Aya active receipt missing")
        deadline = time.monotonic() + 35
        marker = ("OCAF039_Q_GUEST_DENIED=permit_receive" if self.broker_fault else
                  "OCAF039_Q_GUEST_DENIED=aya_ready")
        while time.monotonic() < deadline:
            self.guard.require_live()
            if proc.poll() is not None:
                raise JointFaultDenied("faulted guest exited unexpectedly")
            serial = live._bounded_console_evidence(log)
            if marker in serial:
                break
            time.sleep(.05)
        else:
            raise JointFaultDenied("guest missing Aya did not fail closed")
        if serial.count(marker) != 1 or serial.count("OCAF039_FAULT_HOST_AYA_GATE_RELEASED") != 1 \
                or "OCAF039_Q_GUEST_AUTHORIZED_STOP_RELEASE_PASS" in serial:
            raise JointFaultDenied("guest Aya denial serial inconsistent")
        bproc, bid, bcgroup, binode = peer_b
        if vm_egress.require_vm_running(bproc, bid, bcgroup) != binode:
            raise JointFaultDenied("tenant B failed during guest Aya fault")
        if self.permit_issued:
            raise JointFaultDenied("permit leaked during guest Aya failure")
        if self.broker_fault:
            challenge = self.guest_challenge
            if (not self.broker_sigkill_observed or not isinstance(challenge, dict)
                    or challenge["guest_bpf_id"] <= 0
                    or challenge["event_count"] < 2
                    or serial.count(f"OCAF039_Q_AYA_ID={challenge['guest_bpf_id']} ") != 1):
                raise JointFaultDenied("guest Aya and broker SIGKILL not correlated")
        self.proved = True
        result["joint_guest_aya_fault"] = {
            "status": ("REAL_KVM_GUEST_AYA_LIVE_BROKER_SIGKILL_FAIL_CLOSED"
                       if self.broker_fault else "REAL_KVM_GUEST_AYA_MISSING_FAIL_CLOSED"),
            "guest_denial": marker, "host_aya_program_id": self.receipt["kernel_bpf_program_id"],
            "host_aya_live_during_denial": True,
            "tenant_b_live_during_denial": True, "permit_issued": False,
            "guest_aya_program_id": (self.guest_challenge["guest_bpf_id"]
                                     if self.broker_fault else None),
            "broker_process_sigkill_observed": self.broker_sigkill_observed,
            "broker_fault_tested": self.broker_fault,
            "network_partition_tested": False,
            "stage_039_complete": False, "production_approved": False}

    def close(self) -> None:
        if self.thread is not None:
            self.thread.join(timeout=.1)
        if self.proc is not None and self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=4)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=4)
        if self.guard is not None:
            if self.proved:
                self.receipt = self.guard.release()
            else:
                self.guard.abort()
        if self.broker_proc is not None and self.broker_proc.is_alive():
            self.broker_proc.kill()
            self.broker_proc.join(timeout=5)


def run(prefix: Path, report: Path, *, broker_fault: bool = False) -> dict:
    if os.geteuid() != 0 or not str(prefix).startswith("/opt/ocaf039-real-") \
            or prefix.is_symlink() or not prefix.is_dir() or report.exists():
        raise JointFaultDenied("fresh privileged KVM prefix and report required")
    approved = prefix / "approved"
    if approved.stat().st_uid != 0 or approved.stat().st_mode & 0o077:
        raise JointFaultDenied("approved registry not private")
    with tempfile.TemporaryDirectory(prefix="ocaf039-joint-", dir="/tmp") as directory:
        image = build_fault_image(approved, Path(directory), broker_fault=broker_fault)
        base = json.loads((approved / "request.json").read_text())
        manifest_a = dict(base)
        manifest_a["workload_id"] = ("ocaf-broker-fault-a" if broker_fault else
                                      "ocaf-joint-fault-a")
        manifest_a["isolation"] = {**base["isolation"], "mem_mib": 1024}
        manifest_a["assets"] = dict(base["assets"])
        manifest_a["assets"]["guest_rootfs"] = {"path": str(image),
                                                     "sha256": sha(image)}
        path_a = signed_lab_manifest(manifest_a, approved, broker_fault=broker_fault)
        manifest_b = dict(base)
        manifest_b["tenant_id"] = "tenant-b"
        manifest_b["agent_id"] = "agent-b"
        manifest_b["workload_id"] = ("ocaf-broker-fault-b" if broker_fault else
                                      "ocaf-joint-fault-b")
        path_b = approved / ("request-broker-fault-b.json" if broker_fault else
                             "request-joint-fault-b.json")
        path_b.write_text(json.dumps(manifest_b, sort_keys=True) + "\n")
        path_b.chmod(0o600)
        gate.inspect_manifest(path_b.read_bytes(), approved)
        bridge = AyaFaultBridge(approved, broker_fault=broker_fault)
        b_ready = threading.Event()
        b_release = threading.Event()
        b_state: dict = {}

        def hook_b(result, proc, vm_id, cgroup):
            b_state["peer"] = (proc, vm_id, cgroup, result["cgroup_inode"])
            b_ready.set()
            if not b_release.wait(timeout=110):
                raise JointFaultDenied("tenant B live guard timed out")
            if vm_egress.require_vm_running(proc, vm_id, cgroup) != result["cgroup_inode"]:
                raise JointFaultDenied("tenant B VM no longer alive")

        def launch_b():
            return live.run(manifest=path_b, approved_root=approved,
                jailer=approved / "jailer", jailer_sha=sha(approved / "jailer"),
                firecracker=approved / "firecracker",
                firecracker_sha=manifest_b["assets"]["firecracker"]["sha256"],
                chroot_base=prefix / "jailer", uid=1003, gid=1003,
                timeout=120, boot_hook=hook_b)

        with ThreadPoolExecutor(max_workers=1) as pool:
            future_b = pool.submit(launch_b)
            try:
                if not b_ready.wait(timeout=40):
                    raise JointFaultDenied("tenant B never entered live KVM guard")

                def hook_a(result, proc, vm_id, cgroup):
                    log = prefix / "jailer" / "firecracker" / vm_id / "serial-console.log"
                    bridge.prove(result, proc, vm_id, cgroup, log, b_state["peer"])

                a = live.run(manifest=path_a, approved_root=approved,
                    jailer=approved / "jailer", jailer_sha=sha(approved / "jailer"),
                    firecracker=approved / "firecracker",
                    firecracker_sha=manifest_a["assets"]["firecracker"]["sha256"],
                    chroot_base=prefix / "jailer", uid=1002, gid=1002,
                    timeout=110, boot_hook=hook_a, vsock_bridge=bridge)
            finally:
                b_release.set()
            b = future_b.result(timeout=40)
        fault = a.get("joint_guest_aya_fault")
        if (not isinstance(fault, dict) or fault.get("permit_issued") is not False
                or not bridge.receipt or not bridge.receipt.get("host_probe_recovered_after_vm_stop")
                or a["vm_id"] == b["vm_id"] or a["cgroup_inode"] == b["cgroup_inode"]):
            raise JointFaultDenied("joint guest Aya denial and host recovery proof absent")
        result = {"status": ("ONE_HOST_GUEST_AYA_LIVE_BROKER_SIGKILL_DENIED"
                             if broker_fault else
                             "ONE_HOST_GUEST_AYA_ABSENT_HOST_AYA_LIVE_PEER_SURVIVED"),
                  "tenant_a": fault, "tenant_b_vm_id": b["vm_id"],
                  "host_aya_detach": bridge.receipt,
                  "fault_image_sha256": sha(image), "operator_key_provisioned_independently": False,
                  "broker_fault_tested": broker_fault,
                  "network_partition_tested": False,
                  "stage_039_complete": False, "production_approved": False}
        report.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
        return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--ack-real-kvm-joint-fault", action="store_true")
    parser.add_argument("--fault-mode", choices=("guest-aya-missing", "broker-sigkill"),
                        default="guest-aya-missing")
    args = parser.parse_args()
    if not args.ack_real_kvm_joint_fault:
        parser.error("explicit privileged KVM lab acknowledgment required")
    try:
        print(run(args.prefix, args.report,
                  broker_fault=args.fault_mode == "broker-sigkill")["status"])
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        detail = f"{type(exc).__name__}: {exc}"
        detail = detail.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
        print(f"::error title=stage039-joint-fault::{detail[:500]}", file=sys.stderr)
        parser.error(str(exc))


if __name__ == "__main__":
    main()
