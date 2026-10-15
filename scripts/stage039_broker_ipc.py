#!/usr/bin/env python3
"""OCAF 0.39.3: kernel-authenticated AF_UNIX admission service; no guest launch.

A local-only protocol: four-byte big-endian payload size and strict JSON.
The trusted caller UID and a single pinned (tenant, agent) principal are
supplied by the administrator, never by an unauthenticated request frame.
This is not a microVM runner, an eBPF enforcement point or production gate.
"""
from __future__ import annotations

import asyncio
import base64
from dataclasses import dataclass
import json
import os
from pathlib import Path
import socket
import stat
import struct
import sys
import time
from typing import Callable

import stage039_broker_ledger as broker

MAX_FRAME_BYTES = 128 * 1024
MAX_RESPONSE_BYTES = 16 * 1024
DEADLINE_SECONDS = 3
SO_PEERCRED_LENGTH = struct.calcsize("3i")


class IPCSecurityError(ValueError):
    """A fail-closed socket/peer security precondition did not hold."""


@dataclass(frozen=True)
class PeerPolicy:
    allowed_uids: frozenset[int]
    allowed_pairs: frozenset[tuple[str, str]]

    def __post_init__(self):
        if not self.allowed_uids or not self.allowed_pairs:
            raise IPCSecurityError("empty local peer policy")
        if any(type(uid) is not int or not 0 <= uid <= (1 << 32)-1 for uid in self.allowed_uids):
            raise IPCSecurityError("invalid authorized kernel UID")
        # One principal per broker socket prevents cross-tenant release
        # confusion: peer credentials identify the local service account.
        if len(self.allowed_pairs) != 1:
            raise IPCSecurityError("one tenant:agent principal per socket required")
        for tenant, agent in self.allowed_pairs:
            if (not isinstance(tenant, str) or not isinstance(agent, str) or
                    broker.microvm.IDENTITY.fullmatch(tenant) is None or
                    broker.microvm.IDENTITY.fullmatch(agent) is None):
                raise IPCSecurityError("invalid authorized principal")


def _decode_pinned_base64(value: object, limit: int) -> bytes:
    if not isinstance(value, str) or len(value) > (limit + 2) // 3 * 4 + 4:
        raise IPCSecurityError("invalid bounded encoded data")
    try:
        data = base64.b64decode(value, validate=True)
    except (ValueError, base64.binascii.Error) as exc:
        raise IPCSecurityError("invalid encoded data") from exc
    if not 0 < len(data) <= limit:
        raise IPCSecurityError("decoded data length invalid")
    return data


def _request_json(raw: bytes) -> dict:
    if not 0 < len(raw) <= MAX_FRAME_BYTES:
        raise IPCSecurityError("invalid frame size")
    try:
        data = json.loads(raw, object_pairs_hook=broker.microvm._unique_object,
                          parse_constant=broker.microvm._reject_nonfinite)
    except (ValueError, UnicodeError, TypeError) as exc:
        raise IPCSecurityError("invalid JSON frame") from exc
    if not isinstance(data, dict) or type(data.get("version")) is not int or data["version"] != 1:
        raise IPCSecurityError("invalid frame version")
    op = data.get("op")
    expected = {
        "health": {"version", "op"},
        "reserve": {"version", "op", "request_b64", "manifest_b64"},
        "release": {"version", "op", "workload_id", "request_id", "leader_epoch"},
    }
    if op not in expected or set(data) != expected[op]:
        raise IPCSecurityError("unsupported request or unknown property")
    return data


class BrokerIPC:
    """One local socket for one authorized tenant:agent principal.

    This server operates with the privileges of its parent process; a real
    deployment needs a dedicated unprivileged service account, filesystem
    confinement and independent authorization of the upstream signed RPC.
    """

    def __init__(self, path: Path, ledger: broker.AdmissionLedger,
                 approved_root: Path, policy: PeerPolicy,
                 *, clock: Callable[[], int] | None = None):
        self.path = Path(path)
        self.ledger = ledger
        self.approved_root = Path(approved_root)
        self.policy = policy
        self.clock = clock if clock is not None else lambda: int(time.time())
        self._server: asyncio.AbstractServer | None = None
        self._bound_inode: tuple[int, int] | None = None

    async def start(self) -> None:
        if sys.platform != "linux":
            raise IPCSecurityError("SO_PEERCRED on Linux required")
        if self._server is not None:
            raise IPCSecurityError("broker socket already started")
        parent = self.path.parent
        if (not self.path.is_absolute() or ".." in self.path.parts or
                len(str(self.path).encode()) >= 107 or parent.is_symlink() or
                not parent.is_dir()):
            raise IPCSecurityError("broker requires a safe AF_UNIX parent")
        p = parent.stat()
        if not stat.S_ISDIR(p.st_mode) or p.st_uid != os.geteuid() or p.st_mode & 0o077:
            raise IPCSecurityError("socket directory must be private and owned by service")
        if self.path.exists() or self.path.is_symlink():
            raise IPCSecurityError("socket path must not exist")
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            # A private directory and binding umask restrict pre-chmod access.
            # Since umask is process-global, callers must bootstrap this service
            # before starting any other worker threads.
            old_mask = os.umask(0o077)
            try:
                sock.bind(str(self.path))
            finally:
                os.umask(old_mask)
            os.chmod(self.path, 0o600)
            info = self.path.lstat()
            if not stat.S_ISSOCK(info.st_mode) or info.st_uid != os.geteuid():
                raise IPCSecurityError("incorrect socket inode owner/type")
            self._bound_inode = (info.st_dev, info.st_ino)
            sock.listen(16)
            sock.setblocking(False)
            self._server = await asyncio.start_unix_server(self._serve, sock=sock, backlog=16)
        except Exception:
            sock.close()
            self._unlink_own_socket()
            raise

    def _unlink_own_socket(self) -> None:
        if self._bound_inode is None:
            return
        try:
            st = self.path.lstat()
            if (st.st_dev, st.st_ino) == self._bound_inode and stat.S_ISSOCK(st.st_mode):
                self.path.unlink()
        except FileNotFoundError:
            pass
        self._bound_inode = None

    async def close(self) -> None:
        if self._server is not None:
            self._server.close()
            await self._server.wait_closed()
            self._server = None
        self._unlink_own_socket()

    def _get_authenticated_peer(self, writer: asyncio.StreamWriter) -> tuple[str, str]:
        sock = writer.get_extra_info("socket")
        if sock is None or sock.family != socket.AF_UNIX:
            raise IPCSecurityError("not an AF_UNIX peer")
        raw = sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, SO_PEERCRED_LENGTH)
        pid, uid, gid = struct.unpack("3i", raw)
        if pid <= 0 or uid not in self.policy.allowed_uids or gid < 0:
            raise IPCSecurityError("kernel peer credentials denied")
        return next(iter(self.policy.allowed_pairs))

    async def _serve(self, reader: asyncio.StreamReader,
                     writer: asyncio.StreamWriter) -> None:
        status: dict = {"status": "DENIED", "microvm_launched": False,
                        "ebpf_attached": False, "production_approved": False}
        try:
            principal = self._get_authenticated_peer(writer)
            async with asyncio.timeout(DEADLINE_SECONDS):
                size = struct.unpack("!I", await reader.readexactly(4))[0]
                if not 0 < size <= MAX_FRAME_BYTES:
                    raise IPCSecurityError("invalid frame size")
                packet = _request_json(await reader.readexactly(size))
                status = self._dispatch(packet, principal)
        except (IPCSecurityError, broker.AdmissionError, ValueError,
                OSError, EOFError, asyncio.TimeoutError, asyncio.IncompleteReadError):
            pass
        finally:
            try:
                raw = json.dumps(status, separators=(",", ":"), sort_keys=True).encode()
                if len(raw) > MAX_RESPONSE_BYTES:
                    raise IPCSecurityError("oversize response denied")
                writer.write(struct.pack("!I", len(raw)) + raw)
                await asyncio.wait_for(writer.drain(), timeout=1)
            except (OSError, RuntimeError, asyncio.TimeoutError, IPCSecurityError):
                pass
            writer.close()
            try:
                await writer.wait_closed()
            except (OSError, RuntimeError):
                pass

    def _dispatch(self, packet: dict, principal: tuple[str, str]) -> dict:
        if packet["op"] == "health":
            return {"status": "ADMISSION_ONLY", "microvm_launched": False,
                    "ebpf_attached": False, "production_approved": False}
        if packet["op"] == "reserve":
            req = _decode_pinned_base64(packet["request_b64"], broker.MAX_REQUEST_BYTES)
            manifest = _decode_pinned_base64(packet["manifest_b64"],
                                              broker.microvm.MAX_MANIFEST_BYTES)
            request = broker.request_parse(req, manifest, int(self.clock()))
            if (request["tenant_id"], request["agent_id"]) != principal:
                raise IPCSecurityError("cross-principal reservation denied")
            return self.ledger.reserve(req, manifest, self.approved_root, now=int(self.clock()))
        if packet["op"] == "release":
            workload_id, request_id, epoch = (packet["workload_id"],
                                              packet["request_id"],
                                              packet["leader_epoch"])
            if (not isinstance(workload_id, str) or
                    broker.microvm.IDENTITY.fullmatch(workload_id) is None or
                    not isinstance(request_id, str) or
                    broker.REQ_ID.fullmatch(request_id) is None or
                    type(epoch) is not int or not 1 <= epoch <= broker.MAX_EPOCH):
                raise IPCSecurityError("invalid release parameters")
            row = self.ledger.conn.execute(
                "SELECT tenant_id,agent_id FROM workload_state WHERE workload_id=?",
                (workload_id,)).fetchone()
            if row is None or tuple(row) != principal:
                raise IPCSecurityError("cross-principal release denied")
            if not self.ledger.release(workload_id=workload_id,
                                       request_id=request_id, epoch=epoch):
                raise IPCSecurityError("release not found")
            return {"status": "RELEASED_NOT_EXECUTED", "microvm_launched": False,
                    "ebpf_attached": False, "production_approved": False}
        raise IPCSecurityError("unexpected operation")
