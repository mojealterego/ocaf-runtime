#!/usr/bin/env python3
"""Real KVM guest AF_VSOCK -> kernel-bound VMM -> signed local TLS lab.

This lab is intentionally scoped to one jailed VM and one local TLS origin.
It proves the Firecracker guest/host boundary and replay denial, not a
production egress authorization or multi-host recovery claim.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import http.server
import ipaddress
import json
import multiprocessing
import os
from pathlib import Path
import shutil
import ssl
import subprocess
import sys
import tempfile
import threading
import time

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

import stage039_egress_broker as eg
import stage039_firecracker_spec as spec
import stage039_guest_egress_transport as transport
import stage039_live_firecracker as live
from stage039_signed_manifest import canonical_payload

ROOT = Path(__file__).resolve().parents[1]
BODY = b"ocaf039-real-guest-vsock-local-tls"


class LabHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        if self.path != "/approved":
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(BODY)))
        self.end_headers()
        self.wfile.write(BODY)

    def log_message(self, *_args):
        pass


def _broker_worker(conn, store_path: Path, cert: Path, public: bytes, port: int) -> None:
    """Separate process: the VMM-bound bridge has no direct TLS authority."""
    store = None
    try:
        store = eg.DurablePermitReplay.reopen(store_path)
        context = ssl.create_default_context(cafile=str(cert))
        broker = eg.EgressBroker(
            trusted_keys={"stage039-vsock-lab": public}, ssl_context=context,
            allowed_ports=frozenset({port}),
            allowed_networks=(ipaddress.ip_network("127.0.0.1/32"),),
            resolver=lambda _host, checked_port, **kwargs:
                __import__("socket").getaddrinfo("127.0.0.1", checked_port, **kwargs),
            replay_store=store)
        conn.send(("READY", b""))
        while True:
            request = conn.recv()
            if request is None:
                break
            permit, tenant = request
            try:
                body = broker.fetch(signed_permit=permit,
                                    authenticated_tenant=tenant)
                conn.send(("GRANTED", body))
            except (eg.EgressDenied, OSError) as exc:
                conn.send(("DENIED", f"{type(exc).__name__}: {exc}"))
    except (EOFError, BrokenPipeError):
        pass
    finally:
        if store is not None:
            store.close()
        conn.close()


class ProcessFaultBroker:
    """Crash a real broker process after grant, deny outage, then restart it."""

    def __init__(self, store_path: Path, cert: Path, public: bytes, port: int):
        self.store_path, self.cert, self.public, self.port = store_path, cert, public, port
        self.ctx = multiprocessing.get_context("spawn")
        self.proc = None
        self.conn = None
        self.calls = 0
        self.process_sigkill_observed = False
        self.start()

    def start(self) -> None:
        parent, child = self.ctx.Pipe()
        proc = self.ctx.Process(target=_broker_worker,
                                args=(child, self.store_path, self.cert, self.public, self.port))
        proc.start()
        child.close()
        if not parent.poll(10) or parent.recv() != ("READY", b"") or not proc.is_alive():
            proc.kill()
            proc.join(timeout=5)
            parent.close()
            raise ValueError("separate broker worker failed to become ready")
        self.proc, self.conn = proc, parent

    def fetch(self, *, signed_permit: bytes, authenticated_tenant: str) -> bytes:
        self.calls += 1
        if self.calls == 2:
            if self.proc is None or self.proc.is_alive() or not self.process_sigkill_observed:
                raise ValueError("broker process fault not observed")
            raise eg.EgressDenied("broker process unavailable after SIGKILL")
        if self.calls == 3:
            self.start()
        if self.calls not in (1, 3) or self.proc is None or self.conn is None \
                or not self.proc.is_alive():
            raise eg.EgressDenied("no live broker process for bounded request")
        self.conn.send((signed_permit, authenticated_tenant))
        if not self.conn.poll(12):
            raise eg.EgressDenied("separate broker process response timed out")
        status, response = self.conn.recv()
        if self.calls == 1:
            if status != "GRANTED" or not isinstance(response, bytes):
                raise eg.EgressDenied("initial broker grant absent")
            self.proc.kill()
            self.proc.join(timeout=5)
            if self.proc.exitcode is None or self.proc.exitcode >= 0:
                raise eg.EgressDenied("broker SIGKILL did not terminate worker")
            self.process_sigkill_observed = True
            self.conn.close()
            self.conn = None
            return response
        if status != "DENIED" or "replayed destination permit" not in str(response):
            raise eg.EgressDenied("durable replay denial absent after process restart")
        raise eg.EgressDenied("replayed destination permit after process restart")

    def close(self) -> None:
        if self.proc is not None and self.proc.is_alive():
            self.proc.kill()
            self.proc.join(timeout=5)
        if self.conn is not None:
            self.conn.close()


class VsockBridge:
    def __init__(self, broker: ProcessFaultBroker, tenant: str, uid: int):
        self.broker, self.tenant, self.uid = broker, tenant, uid
        self.thread: threading.Thread | None = None
        self.decisions: list[dict] | None = None
        self.failure: BaseException | None = None

    def config(self, root: Path) -> dict:
        if (not root.is_absolute() or root.is_symlink()
                or len(os.fsencode(root / "e_5000")) > 100):
            raise ValueError("private jailed vsock path exceeds AF_UNIX bound")
        self.path = root / "e_5000"
        # CID is VM scoped and not a security identity. The VMM kernel peer is.
        cid = 3 + int(hashlib.sha256(root.parent.name.encode()).hexdigest()[:6], 16)
        self.cid = cid
        return {"guest_cid": cid, "uds_path": "/e"}

    def start(self, proc, root: Path) -> None:
        if root / "e_5000" != self.path or proc.poll() is not None:
            raise ValueError("vsock listener cannot bind untrusted VM")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if proc.poll() is not None:
                raise ValueError("Firecracker exited before entering jailed UID and cgroup")
            try:
                peer = transport.VmPeer.capture(
                    tenant_id=self.tenant, uid=self.uid, pid=proc.pid)
            except transport.TransportDenied:
                time.sleep(.02)
                continue
            if "ocaf-stage039-vm-" in peer.cgroup:
                break
            time.sleep(.02)
        else:
            raise ValueError("Firecracker did not enter dedicated cgroup before vsock bind")

        def serve():
            try:
                self.decisions = transport.listen_many(
                    self.path, peer=peer, broker=self.broker, requests=3)
            except BaseException as exc:
                self.failure = exc

        self.thread = threading.Thread(target=serve, daemon=True)
        self.thread.start()
        deadline = time.monotonic() + 3
        while not self.path.exists() and self.failure is None and time.monotonic() < deadline:
            time.sleep(.01)
        if not self.path.exists() or self.failure is not None:
            raise ValueError("private Firecracker vsock listener failed to bind: "
                             + repr(self.failure))

    def prove(self, log: Path, proc) -> dict:
        deadline = time.monotonic() + 40
        serial = ""
        while time.monotonic() < deadline:
            if self.failure is not None:
                raise ValueError("vsock listener failed: " + repr(self.failure))
            if proc.poll() is not None:
                raise ValueError("Firecracker exited during guest vsock exchange")
            serial = live._bounded_console_evidence(log)
            if "OCAF039_GUEST_VSOCK_FAILURE" in serial:
                markers = [line[-120:] for line in serial.splitlines()
                           if "OCAF039_" in line][-10:]
                raise ValueError("guest vsock PID1 rejected the exchange; "
                                 f"guest_markers={markers!r} "
                                 f"decisions={self.decisions!r} bridge_failure={self.failure!r}")
            if "OCAF039_GUEST_VSOCK_GRANT_CRASH_DENY_RESTART_REPLAY_DENY_PASS" in serial:
                break
            time.sleep(.1)
        else:
            markers = [line[-120:] for line in serial.splitlines()
                       if "OCAF039_" in line][-10:]
            raise ValueError("real guest vsock grant/replay deadline expired; "
                             f"guest_markers={markers!r}")
        if self.thread is None:
            raise ValueError("no vsock listener thread")
        self.thread.join(timeout=2)
        if self.thread.is_alive() or self.decisions is None:
            raise ValueError("vsock broker did not complete all three requests")
        if (serial.count("OCAF039_GUEST_EGRESS_BROKERED_BYTES=") != 1
                or f"OCAF039_GUEST_EGRESS_BROKERED_BYTES={len(BODY)}" not in serial
                or serial.count("OCAF039_GUEST_EGRESS_DENIED") != 2
                or [d["status"] for d in self.decisions] != ["GRANTED", "DENIED", "DENIED"]
                or "broker process unavailable" not in self.decisions[1].get("deny_reason", "")
                or "replayed destination permit" not in self.decisions[2].get("deny_reason", "")
                or not self.broker.process_sigkill_observed
                or self.decisions[0].get("response_sha256") != hashlib.sha256(BODY).hexdigest()
                or any(d["tenant_id"] != self.tenant or d["vm_pid"] != proc.pid
                       for d in self.decisions)):
            raise ValueError("real guest serial and kernel-bound egress decisions disagree")
        return {"status": "REAL_GUEST_VSOCK_SIGNED_TLS_GRANT_PROCESS_CRASH_REPLAY_DENY",
                "guest_cid": self.cid, "guest_serial_sha256": hashlib.sha256(serial.encode()).hexdigest(),
                "decisions": self.decisions, "broker_process_sigkill_while_guest_live": True,
                "no_guest_grant_while_broker_dead": True,
                "durable_replay_denied_after_broker_process_restart": True,
                "guest_direct_network": False,
                "guest_kvm_tested": True, "stage_039_complete": False,
                "production_approved": False}

    def close(self) -> None:
        # The successful path has already consumed both bounded connections.
        if self.thread is not None:
            self.thread.join(timeout=.1)


def build_guest_image(approved: Path, permit: bytes, work: Path) -> Path:
    image = approved / "egress_guest_rootfs"
    shutil.copyfile(approved / "guest_rootfs", image)
    subprocess.run(["truncate", "-s", "+64M", str(image)], check=True)
    checked = subprocess.run(["e2fsck", "-f", "-y", str(image)], check=False,
                             stdout=subprocess.DEVNULL)
    if checked.returncode not in (0, 1):
        raise ValueError("copied guest ext4 image failed filesystem check")
    subprocess.run(["resize2fs", str(image)], check=True, stdout=subprocess.DEVNULL)
    client = work / "ocaf-guest-vsock-egress"
    init = work / "ocaf-egress-init"
    for source, target in (("guest_vsock_egress.c", client),
                           ("guest_egress_init.c", init)):
        subprocess.run(["gcc", "-std=c11", "-O2", "-static", "-Wall", "-Wextra",
                        "-Werror", str(ROOT / "stage039/guest-egress-broker" / source),
                        "-o", str(target)], check=True)
    mount = work / "mount"
    mount.mkdir(mode=0o700)
    subprocess.run(["mount", "-o", "loop,rw", str(image), str(mount)], check=True)
    try:
        (mount / "usr/bin").mkdir(parents=True, exist_ok=True)
        (mount / "opt/ocaf").mkdir(parents=True, exist_ok=True)
        shutil.copyfile(client, mount / "usr/bin/ocaf-guest-vsock-egress")
        shutil.copyfile(init, mount / "ocaf-egress-init")
        (mount / "opt/ocaf/egress-permit.json").write_bytes(permit)
        for path in (mount / "usr/bin/ocaf-guest-vsock-egress", mount / "ocaf-egress-init"):
            path.chmod(0o555)
        (mount / "opt/ocaf/egress-permit.json").chmod(0o444)
        subprocess.run(["sync"], check=True)
    finally:
        subprocess.run(["umount", str(mount)], check=True)
    image.chmod(0o444)
    return image


def run(prefix: Path, report: Path) -> dict:
    if (os.geteuid() != 0 or not str(prefix).startswith("/opt/ocaf039-real-")
            or prefix.is_symlink() or not prefix.is_dir() or report.exists()):
        raise ValueError("fresh root-owned real KVM lab paths required")
    approved = prefix / "approved"
    if approved.stat().st_uid != 0 or approved.stat().st_mode & 0o077:
        raise ValueError("untrusted approved asset directory")
    with tempfile.TemporaryDirectory(prefix="ocaf039-vsock-", dir="/tmp") as temporary:
        work = Path(temporary)
        cert, keyfile = work / "cert.pem", work / "key.pem"
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048",
                        "-keyout", str(keyfile), "-out", str(cert), "-days", "1",
                        "-nodes", "-subj", "/CN=localhost",
                        "-addext", "subjectAltName=DNS:localhost"], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), LabHandler)
        tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        tls.load_cert_chain(cert, keyfile)
        server.socket = tls.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            port = server.server_address[1]
            signing = Ed25519PrivateKey.generate()
            public = signing.public_key().public_bytes(
                serialization.Encoding.Raw, serialization.PublicFormat.Raw)
            payload = {"schema_version": 1, "tenant_id": "tenant-a",
                       "nonce": os.urandom(16).hex(),
                       "url": f"https://localhost:{port}/approved",
                       "expires_unix": int(time.time()) + 180, "max_bytes": len(BODY)}
            permit = json.dumps({"key_id": "stage039-vsock-lab",
                                 "public_key": base64.b64encode(public).decode(),
                                 "signature": base64.b64encode(signing.sign(canonical_payload(payload))).decode(),
                                 "payload": payload}, sort_keys=True).encode()
            image = build_guest_image(approved, permit, work)
            manifest = json.loads((approved / "request.json").read_text())
            manifest["workload_id"] = "ocaf-fc-vsock-lab"
            manifest["assets"]["guest_rootfs"] = {
                "path": str(image), "sha256": hashlib.sha256(image.read_bytes()).hexdigest()}
            manifest_path = approved / "request-egress.json"
            manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
            manifest_path.chmod(0o600)
            replay_path = approved / "vsock_replay.sqlite"
            eg.DurablePermitReplay(replay_path).close()
            process_broker = ProcessFaultBroker(replay_path, cert, public, port)
            bridge = VsockBridge(process_broker, "tenant-a", 1002)
            def hooked(result, proc, vm_id, _cgroup):
                log = prefix / "jailer" / "firecracker" / vm_id / "serial-console.log"
                result["guest_vsock_egress"] = bridge.prove(log, proc)
            proof = live.run(
                manifest=manifest_path, approved_root=approved,
                jailer=approved / "jailer",
                jailer_sha=hashlib.sha256((approved / "jailer").read_bytes()).hexdigest(),
                firecracker=approved / "firecracker",
                firecracker_sha=manifest["assets"]["firecracker"]["sha256"],
                chroot_base=prefix / "jailer", uid=1002, gid=1002, timeout=100,
                boot_hook=hooked, vsock_bridge=bridge)
            if proof["guest_vsock_egress"]["status"] != "REAL_GUEST_VSOCK_SIGNED_TLS_GRANT_PROCESS_CRASH_REPLAY_DENY":
                raise ValueError("real guest vsock proof absent")
            report.write_text(json.dumps(proof, sort_keys=True, indent=2) + "\n")
            return proof
        finally:
            if "process_broker" in locals():
                process_broker.close()
            server.shutdown()
            server.server_close()
            thread.join(timeout=3)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--ack-real-kvm-egress", action="store_true")
    args = parser.parse_args()
    if not args.ack_real_kvm_egress:
        parser.error("real privileged KVM egress lab acknowledgment required")
    try:
        proof = run(args.prefix, args.report)
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        # GitHub's check-run annotations remain queryable through its REST API
        # when the external artifact/log storage is unavailable. Bound and
        # escape the diagnostic so guest serial cannot inject workflow syntax.
        detail = f"{type(exc).__name__}: {exc}"[:900]
        detail = detail.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
        print(f"::error title=Stage039 guest vsock::{detail}", file=sys.stderr)
        parser.error(str(exc))
    print(proof["guest_vsock_egress"]["status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
