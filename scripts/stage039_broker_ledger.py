#!/usr/bin/env python3
"""0.39.2: durable, fail-closed microVM broker *admission* ledger.

This library does NOT launch Firecracker, enforce Aya/eBPF or authenticate
network callers. Its only authority is a bounded, transactional local claim.
The future privileged broker MUST validate kernel SO_PEERCRED, re-pin verified
file descriptors, attach eBPF and perform additional host security checks.
"""
from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import stat
import time

import stage039_microvm_gate as microvm

REQ_ID = re.compile(r"[0-9a-f]{32}\Z")
MAX_REQUEST_BYTES = 16 * 1024
MAX_HISTORY_ROWS = 100_000
MAX_DB_FILE_BYTES = 64 * 1024 * 1024
MAX_ACTIVE_LEASES = 64
MAX_EPOCH = (1 << 63) - 1
MAX_LEASE_SECONDS = 300


class AdmissionError(ValueError):
    """Reject an untrusted request without provisioning side effects."""


def request_parse(raw: bytes, manifest: bytes, now: int) -> dict:
    if not 0 < len(raw) <= MAX_REQUEST_BYTES or not isinstance(now, int) or now <= 0:
        raise AdmissionError("request size or clock invalid")
    try:
        request = json.loads(raw, object_pairs_hook=microvm._unique_object,
                             parse_constant=microvm._reject_nonfinite)
        manifest_info = microvm.parse_manifest(manifest)
    except (UnicodeError, ValueError, TypeError) as exc:
        raise AdmissionError("invalid admission request or manifest") from exc
    required = {"schema_version", "request_id", "tenant_id", "agent_id", "workload_id",
                "leader_epoch", "issued_unix_secs", "expires_unix_secs", "manifest_sha256"}
    if not isinstance(request, dict) or set(request) != required:
        raise AdmissionError("invalid admission schema")
    if type(request["schema_version"]) is not int or request["schema_version"] != 1:
        raise AdmissionError("unsupported request version")
    if not isinstance(request["request_id"], str) or not REQ_ID.fullmatch(request["request_id"]):
        raise AdmissionError("invalid replay identifier")
    for key in ("tenant_id", "agent_id", "workload_id"):
        if request[key] != manifest_info[key]:
            raise AdmissionError("principal or workload mismatch")
    epoch = request["leader_epoch"]
    issued = request["issued_unix_secs"]
    expiry = request["expires_unix_secs"]
    if any(type(v) is not int for v in (epoch, issued, expiry)):
        raise AdmissionError("non-integer fencing or timestamp")
    if not (1 <= epoch <= MAX_EPOCH and 0 < issued <= now + 5 and
            now - issued <= 30 and 1 <= expiry - now <= MAX_LEASE_SECONDS):
        raise AdmissionError("expired, overlong or unbounded admission request")
    if (not isinstance(request["manifest_sha256"], str) or
            request["manifest_sha256"] != hashlib.sha256(manifest).hexdigest()):
        raise AdmissionError("manifest digest mismatch")
    return request


def _check_directory(db: Path) -> None:
    if not db.is_absolute() or ".." in db.parts or db.name in ("", ".", ".."):
        raise AdmissionError("ledger requires a safe absolute path")
    if db.exists() or db.is_symlink():
        st = db.lstat()
        if stat.S_ISLNK(st.st_mode) or not stat.S_ISREG(st.st_mode) or st.st_mode & 0o077:
            raise AdmissionError("ledger inode permissions invalid")
        if st.st_size > MAX_DB_FILE_BYTES or st.st_nlink != 1:
            raise AdmissionError("ledger size or link count invalid")
    parent = db.parent
    if parent.is_symlink() or not parent.is_dir() or parent.stat().st_mode & 0o077:
        raise AdmissionError("ledger directory must be private (0700)")
    if not os.access(parent, os.W_OK | os.X_OK):
        raise AdmissionError("ledger directory not writable")


class AdmissionLedger:
    """Single-host serialized SQLite lease/fencing state; not production consensus."""

    def __init__(self, db: Path):
        self.path = Path(db)
        _check_directory(self.path)
        # SQLite normally creates the database using the process umask, which
        # may briefly expose the uninitialized inode as 0644. Create it
        # exclusively with 0600 *before* sqlite3.connect touches the path.
        if not self.path.exists():
            flags = os.O_CREAT | os.O_EXCL | os.O_RDWR | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
            fd = os.open(self.path, flags, 0o600)
            os.close(fd)
        _check_directory(self.path)
        self.conn = sqlite3.connect(str(self.path), isolation_level=None, timeout=5)
        try:
            self.conn.execute("PRAGMA journal_mode=WAL")
            self.conn.execute("PRAGMA synchronous=FULL")
            self.conn.execute("PRAGMA busy_timeout=5000")
            self.conn.execute("PRAGMA trusted_schema=OFF")
            self.conn.executescript("""
                CREATE TABLE IF NOT EXISTS lease_history (
                    request_id TEXT PRIMARY KEY,
                    workload_id TEXT NOT NULL,
                    tenant_id TEXT NOT NULL,
                    agent_id TEXT NOT NULL,
                    leader_epoch INTEGER NOT NULL,
                    admitted_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS workload_state (
                    workload_id TEXT PRIMARY KEY,
                    tenant_id TEXT NOT NULL,
                    agent_id TEXT NOT NULL,
                    epoch INTEGER NOT NULL,
                    request_id TEXT NOT NULL,
                    expires_at INTEGER NOT NULL,
                    status TEXT NOT NULL CHECK(status IN ('reserved','released'))
                );
            """)
            os.chmod(self.path, 0o600)
            _check_directory(self.path)
        except Exception:
            self.conn.close()
            raise

    def close(self) -> None:
        self.conn.close()

    def __enter__(self) -> "AdmissionLedger":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    def reserve(self, raw: bytes, manifest: bytes, approved_root: Path,
                *, now: int | None = None) -> dict:
        """Validate static artifacts + host and reserve one fenced workload.

        Does NOT prove an actor's identity, safe VM launch or real isolation.
        This method must remain callable only from an externally authenticated
        privileged broker; it MUST NOT be exposed as an unauthenticated RPC.
        """
        clock = int(time.time()) if now is None else now
        request = request_parse(raw, manifest, clock)
        # Verify real local host; a passed-in JSON claim is not trusted.
        host = microvm.preflight()
        if host["status"] != "HOST_PREREQUISITES_ONLY":
            raise AdmissionError("host KVM/Aya prerequisites absent")
        static = microvm.inspect_manifest(manifest, Path(approved_root))
        if static["status"] != "STATIC_ARTIFACTS_VERIFIED_ONLY":
            raise AdmissionError("artifact policy denied")
        if self.path.stat().st_size > MAX_DB_FILE_BYTES:
            raise AdmissionError("ledger storage bound exceeded")
        try:
            self.conn.execute("BEGIN IMMEDIATE")
            if self.conn.execute(
                "SELECT 1 FROM lease_history WHERE request_id=?", (request["request_id"],)
            ).fetchone():
                raise AdmissionError("request replay")
            if self.conn.execute("SELECT COUNT(*) FROM lease_history").fetchone()[0] >= MAX_HISTORY_ROWS:
                raise AdmissionError("ledger history capacity exhausted")
            row = self.conn.execute(
                "SELECT tenant_id,agent_id,epoch,expires_at,status FROM workload_state WHERE workload_id=?",
                (request["workload_id"],)
            ).fetchone()
            if row:
                if row[0] != request["tenant_id"] or row[1] != request["agent_id"]:
                    raise AdmissionError("workload ownership cannot change")
                if request["leader_epoch"] <= row[2]:
                    raise AdmissionError("stale or reused leader fencing epoch")
                if row[4] == "reserved" and row[3] > clock:
                    raise AdmissionError("active workload cannot be overwritten")
            active = self.conn.execute(
                "SELECT COUNT(*) FROM workload_state WHERE status='reserved' AND expires_at>?",
                (clock,)
            ).fetchone()[0]
            if active >= MAX_ACTIVE_LEASES:
                raise AdmissionError("broker capacity exhausted")
            self.conn.execute(
                "INSERT INTO lease_history "
                "(request_id, workload_id, tenant_id, agent_id, leader_epoch, admitted_at) "
                "VALUES (?,?,?,?,?,?)",
                (request["request_id"], request["workload_id"], request["tenant_id"],
                 request["agent_id"], request["leader_epoch"], clock))
            self.conn.execute(
                "INSERT INTO workload_state "
                "(workload_id,tenant_id,agent_id,epoch,request_id,expires_at,status) "
                "VALUES (?,?,?,?,?,?,'reserved') "
                "ON CONFLICT(workload_id) DO UPDATE SET "
                "epoch=excluded.epoch,request_id=excluded.request_id,"
                "expires_at=excluded.expires_at,status='reserved'",
                (request["workload_id"], request["tenant_id"], request["agent_id"],
                 request["leader_epoch"], request["request_id"],
                 request["expires_unix_secs"]))
            self.conn.execute("COMMIT")
        except (sqlite3.Error, AdmissionError) as exc:
            if self.conn.in_transaction:
                self.conn.execute("ROLLBACK")
            if isinstance(exc, AdmissionError):
                raise
            raise AdmissionError("durable ledger unavailable") from exc
        return {"schema_version": 1, "status": "RESERVED_NOT_EXECUTED",
                "workload_id": request["workload_id"],
                "tenant_id": request["tenant_id"], "agent_id": request["agent_id"],
                "request_id": request["request_id"], "leader_epoch": request["leader_epoch"],
                "manifest_sha256": request["manifest_sha256"],
                "host_prerequisites_observed": True, "microvm_launched": False,
                "ebpf_attached": False, "production_approved": False,
                "stage_039_complete": False}

    def release(self, *, workload_id: str, request_id: str, epoch: int) -> bool:
        if (not isinstance(workload_id, str) or microvm.IDENTITY.fullmatch(workload_id) is None
                or not isinstance(request_id, str) or REQ_ID.fullmatch(request_id) is None
                or type(epoch) is not int or not 1 <= epoch <= MAX_EPOCH):
            raise AdmissionError("invalid release identity")
        try:
            self.conn.execute("BEGIN IMMEDIATE")
            changed = self.conn.execute(
                "UPDATE workload_state SET status='released' "
                "WHERE workload_id=? AND request_id=? AND epoch=? AND status='reserved'",
                (workload_id, request_id, epoch)).rowcount
            self.conn.execute("COMMIT")
            return changed == 1
        except sqlite3.Error as exc:
            if self.conn.in_transaction:
                self.conn.execute("ROLLBACK")
            raise AdmissionError("ledger release failed") from exc
