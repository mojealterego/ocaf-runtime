#!/usr/bin/env python3
"""Fail-closed Linux SO_PEERCRED binding for the local Agents Builder bridge.

This authenticates the connected Unix peer's kernel-reported process and UID.
It does not authorize project requests or replace Rust OAuth and tenant checks.
"""
from __future__ import annotations

import os
import socket
import struct
import sys
from typing import Protocol


class BridgePeerError(ValueError):
    """A bridge connection failed kernel peer-identity admission."""


class CredentialSocket(Protocol):
    def getsockopt(self, level: int, optname: int, buflen: int) -> bytes: ...


_CREDENTIALS_SIZE = struct.calcsize('3i')


def verify_bridge_peer(peer: CredentialSocket, *, expected_pid: int | None) -> int:
    """Verify kernel Unix peer UID and, when pinned, exact supervisor child PID.

    Failure is fatal for the connection before any bearer token or project bytes
    are sent. On unsupported systems, deny rather than falling back to a file
    ownership check. The returned PID is for audit-free internal control flow.
    """
    if expected_pid is not None and (type(expected_pid) is not int or expected_pid <= 0):
        raise BridgePeerError('invalid expected PID')
    if not sys.platform.startswith('linux') or not hasattr(socket, 'SO_PEERCRED'):
        raise BridgePeerError('kernel Unix peer credentials unavailable')
    try:
        raw = peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, _CREDENTIALS_SIZE)
        if len(raw) != _CREDENTIALS_SIZE:
            raise BridgePeerError('invalid kernel peer credentials')
        pid, uid, _gid = struct.unpack('3i', raw)
    except (OSError, ValueError, TypeError, struct.error) as exc:
        raise BridgePeerError('kernel peer credentials unavailable or invalid') from exc
    if pid <= 0 or uid != os.geteuid():
        raise BridgePeerError('untrusted bridge UID or PID')
    if expected_pid is not None and pid != expected_pid:
        raise BridgePeerError('bridge PID differs from supervisor child')
    return pid
