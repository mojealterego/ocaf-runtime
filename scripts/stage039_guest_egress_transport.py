#!/usr/bin/env python3
"""One-VM Firecracker vsock backend to a signed HTTPS egress broker.

Firecracker connects to ``uds_path_<guest_port>`` when its guest connects to
host CID 2.  This listener identifies the *VMM process* through kernel
SO_PEERCRED and a live PID/start-time/cgroup binding before it reads a guest
request.  Tenant identity is never accepted from the guest payload.

The existing EgressBroker still decides the destination, pins DNS to one
literal IP, verifies HTTPS identity, and rejects redirects/metadata IPs.
This module only provides the guest-to-host boundary; full KVM integration
requires provisioning a per-VM Firecracker vsock device and host listener.
"""
from __future__ import annotations

from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import socket
import stat
import struct
import time
from typing import Protocol

MAX_PERMIT = 64 * 1024
MAX_RESPONSE = 1024 * 1024
PORT = 5000
_PEER_SIZE = struct.calcsize("3i")


class TransportDenied(ValueError):
    pass


class SignedFetch(Protocol):
    def fetch(self, *, signed_permit: bytes, authenticated_tenant: str) -> bytes: ...


def _trusted_parent_owner(owner_uid: int, peer: "VmPeer") -> bool:
    # Jailer chowns the private chroot to the VMM UID before this listener
    # binds. The peer comes from the live jailed PID and is rechecked for
    # every accepted connection.
    return owner_uid in (0, os.geteuid(), peer.uid)


def _proc_start_ticks(pid: int) -> int:
    raw = Path(f"/proc/{pid}/stat").read_text(encoding="ascii")
    end = raw.rfind(")")
    if end < 0:
        raise TransportDenied("unreadable VM process identity")
    # Fields after comm start at proc stat field 3; starttime is field 22.
    value = int(raw[end + 2:].split()[19])
    if value <= 0:
        raise TransportDenied("invalid VM process start time")
    return value


@dataclass(frozen=True)
class VmPeer:
    tenant_id: str
    uid: int
    pid: int
    proc_start_ticks: int
    cgroup: str
    host_boot_id: str

    @classmethod
    def capture(cls, *, tenant_id: str, uid: int, pid: int) -> "VmPeer":
        if (tenant_id not in ("tenant-a", "tenant-b")
                or type(uid) is not int or uid < 0
                or type(pid) is not int or pid <= 1):
            raise TransportDenied("bounded live VM principal required")
        try:
            current_uid = Path(f"/proc/{pid}").stat().st_uid
            cgroup = Path(f"/proc/{pid}/cgroup").read_text(encoding="ascii")
            boot_id = Path("/proc/sys/kernel/random/boot_id").read_text(encoding="ascii").strip()
            start = _proc_start_ticks(pid)
        except (OSError, ValueError, IndexError) as exc:
            raise TransportDenied("live VM kernel identity unavailable") from exc
        if current_uid != uid or not cgroup or not boot_id:
            raise TransportDenied("VM UID, cgroup or host boot identity mismatch")
        return cls(tenant_id, uid, pid, start, cgroup, boot_id)

    def verify(self, connection: socket.socket) -> None:
        if not hasattr(socket, "SO_PEERCRED"):
            raise TransportDenied("Linux SO_PEERCRED required")
        raw = connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, _PEER_SIZE)
        pid, uid, _gid = struct.unpack("3i", raw)
        if pid != self.pid or uid != self.uid:
            raise TransportDenied("vsock backend peer is not the reserved VM process")
        try:
            observed = VmPeer.capture(tenant_id=self.tenant_id, uid=uid, pid=pid)
        except TransportDenied as exc:
            raise TransportDenied("reserved VM process no longer live") from exc
        if observed != self:
            raise TransportDenied("VM PID reused, moved cgroup or host rebooted")


def _read_exact(conn: socket.socket, size: int, deadline: float) -> bytes:
    result = bytearray()
    while len(result) < size:
        left = deadline - time.monotonic()
        if left <= 0:
            raise TransportDenied("guest frame deadline expired")
        conn.settimeout(min(1.0, left))
        try:
            chunk = conn.recv(size - len(result))
        except socket.timeout as exc:
            raise TransportDenied("guest frame deadline expired") from exc
        if not chunk:
            raise TransportDenied("truncated guest frame")
        result.extend(chunk)
    return bytes(result)


def handle_one(conn: socket.socket, *, peer: VmPeer, broker: SignedFetch) -> dict:
    """Handle one bounded request; any auth/fetch failure yields a DENY frame."""
    decision = {"status": "DENIED", "tenant_id": peer.tenant_id,
                "vm_pid": peer.pid, "guest_response_bytes": 0}
    try:
        peer.verify(conn)
        deadline = time.monotonic() + 3
        length = struct.unpack("!I", _read_exact(conn, 4, deadline))[0]
        if not 0 < length <= MAX_PERMIT:
            raise TransportDenied("signed permit frame exceeds bound")
        permit = _read_exact(conn, length, deadline)
        body = broker.fetch(signed_permit=permit, authenticated_tenant=peer.tenant_id)
        if not isinstance(body, bytes) or len(body) > MAX_RESPONSE:
            raise TransportDenied("upstream response outside bound")
        conn.settimeout(3)
        conn.sendall(struct.pack("!BI", 1, len(body)) + body)
        decision.update(status="GRANTED", guest_response_bytes=len(body),
                        response_sha256=hashlib.sha256(body).hexdigest())
    except Exception:
        # Any failed broker, DNS, TLS, or policy dependency fails closed.
        try:
            conn.sendall(struct.pack("!BI", 0, 0))
        except OSError:
            pass
    return decision


def listen_many(path: Path, *, peer: VmPeer, broker: SignedFetch,
                requests: int) -> list[dict]:
    """Create one per-VM Firecracker vsock endpoint for bounded requests."""
    if type(requests) is not int or not 1 <= requests <= 4:
        raise TransportDenied("bounded guest request count required")
    if (not isinstance(path, Path) or not path.is_absolute() or path.is_symlink()
            or path.resolve(strict=False) != path or path.parent.is_symlink()):
        raise TransportDenied("absolute non-symlinked vsock UDS path required")
    parent = path.parent
    pstat = parent.stat()
    if (not stat.S_ISDIR(pstat.st_mode) or pstat.st_mode & 0o022
            or not _trusted_parent_owner(pstat.st_uid, peer)
            or len(os.fsencode(path)) > 100 or path.exists()):
        raise TransportDenied("private fresh per-VM socket parent required")
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        listener.bind(str(path))
        os.chown(path, peer.uid, -1)
        os.chmod(path, 0o600)
        listener.listen(1)
        listener.settimeout(45)
        decisions=[]
        for _ in range(requests):
            conn, _address = listener.accept()
            with conn:
                decisions.append(handle_one(conn, peer=peer, broker=broker))
        return decisions
    finally:
        listener.close()
        try:
            path.unlink()
        except FileNotFoundError:
            pass


def listen_once(path: Path, *, peer: VmPeer, broker: SignedFetch) -> dict:
    """Serve one guest request and remove the per-VM UDS endpoint."""
    return listen_many(path, peer=peer, broker=broker, requests=1)[0]
