#!/usr/bin/env python3
"""Durable SINGLE-WITNESS lease authority for PostgreSQL promotion (0.40.8).

This is not majority quorum, distributed consensus, database fencing, or a
remote authentication service. Every authority process must use the SAME local
SQLite file on a filesystem with working SQLite locks/fsync. Replicating or
restoring a stale copy of the file creates another authority and is forbidden.
A remote adapter must authenticate the holder; a holder string is not a secret.

Epochs and promotion state survive process restart. The default clock is the
local monotonic clock, bound to the Linux boot ID; reboot/domain changes and
clock rollback fail closed. Operators must fence every previous database
primary before provisioning a replacement witness after loss of its state or
clock domain. An injected test clock requires an explicit common clock_domain.

A grant reserves a lease; authorize_promotion enables effects without consuming
or fencing that lease. Every effect needs a fresh online authorization, and the
database/resource adapter must enforce the epoch at its actual write boundary.
An authorization is a point-in-time decision, never a permit to continue
writing through expiry, network partitions, or witness unavailability. This
module alone cannot stop a paused/malicious PostgreSQL process from writing.
"""
from __future__ import annotations

from contextlib import contextmanager
from dataclasses import dataclass
import math
import os
from pathlib import Path
import re
import sqlite3
import stat
import time

MAX_TTL_SECONDS = 300
MAX_CLOCK_SECONDS = 10**12
MAX_EPOCH = 2**63 - 1
_IDENTIFIER = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}\Z")


class QuorumError(ValueError):
    pass


def _identifier(value) -> bool:
    return isinstance(value, str) and _IDENTIFIER.fullmatch(value) is not None


def _finite_number(value) -> bool:
    return type(value) is int or type(value) is float and math.isfinite(value)


def _valid_clock(value) -> bool:
    return _finite_number(value) and 0 <= value <= MAX_CLOCK_SECONDS - MAX_TTL_SECONDS


def _validate_holder_epoch(holder, epoch) -> None:
    if not _identifier(holder):
        raise QuorumError("invalid bounded witness holder")
    if type(epoch) is not int or not 1 <= epoch <= MAX_EPOCH:
        raise QuorumError("invalid bounded witness epoch")


def _validate_ttl(ttl_seconds) -> None:
    if not _finite_number(ttl_seconds) or not 0 < ttl_seconds <= MAX_TTL_SECONDS:
        raise QuorumError("invalid finite bounded witness lease")


@dataclass(frozen=True)
class WitnessLease:
    holder: str
    epoch: int
    expires_at: float

    def valid_for(self, holder: str, now: float) -> bool:
        """Local expiry predicate only; never substitutes for online authority."""
        return (_identifier(holder) and holder == self.holder
                and type(self.epoch) is int and 1 <= self.epoch <= MAX_EPOCH
                and _valid_clock(now) and _finite_number(self.expires_at)
                and now < self.expires_at <= MAX_CLOCK_SECONDS)


class QuorumGate:
    """One durable local witness, serializing claims through BEGIN IMMEDIATE."""

    def __init__(self, state_path: Path | str, *, clock=time.monotonic,
                 clock_domain: str | None = None):
        self._conn = None
        if not callable(clock):
            raise QuorumError("authoritative clock must be callable")
        if clock_domain is None:
            if clock is not time.monotonic:
                raise QuorumError("injected clock needs an explicit clock domain")
            try:
                clock_domain = "linux-boot:" + Path(
                    "/proc/sys/kernel/random/boot_id").read_text(encoding="ascii").strip()
            except (OSError, UnicodeError) as exc:
                raise QuorumError("cannot establish authoritative boot clock domain") from exc
        if not _identifier(clock_domain):
            raise QuorumError("invalid bounded witness clock domain")
        self._clock = clock
        self._clock_domain = clock_domain
        self.path = self._private_path(state_path)
        try:
            self._conn = sqlite3.connect(str(self.path), isolation_level=None, timeout=5)
            self._conn.execute("PRAGMA trusted_schema=OFF")
            self._conn.execute("PRAGMA journal_mode=WAL")
            self._conn.execute("PRAGMA synchronous=FULL")
            self._conn.execute("PRAGMA busy_timeout=5000")
            with self._transaction() as db:
                version = db.execute("PRAGMA user_version").fetchone()[0]
                if version not in (0, 1):
                    raise QuorumError("unsupported witness database version")
                if version == 0:
                    if db.execute("SELECT name FROM sqlite_master").fetchone() is not None:
                        raise QuorumError("refusing an unrelated witness database")
                    db.execute("""CREATE TABLE witness_state (
                        id INTEGER PRIMARY KEY CHECK(id=1),
                        epoch INTEGER NOT NULL CHECK(epoch>=0),
                        holder TEXT,
                        expires_at REAL NOT NULL,
                        promoted INTEGER NOT NULL CHECK(promoted IN (0,1)),
                        last_clock REAL NOT NULL,
                        clock_domain TEXT NOT NULL)""")
                    db.execute("INSERT INTO witness_state VALUES(1,0,NULL,0,0,0,?)",
                               (clock_domain,))
                    db.execute("PRAGMA user_version=1")
                self._state(db)
        except (sqlite3.Error, QuorumError) as exc:
            self.close()
            raise QuorumError(f"witness state unavailable: {exc}") from exc

    @staticmethod
    def _private_path(value) -> Path:
        if not isinstance(value, (str, Path)):
            raise QuorumError("witness storage path required")
        path = Path(value)
        if not path.is_absolute() or ".." in path.parts:
            raise QuorumError("absolute witness storage path required")
        try:
            parent = path.parent
            parent_stat = parent.stat()
            if (any(part.is_symlink() for part in (parent, *parent.parents))
                    or not parent.is_dir() or parent_stat.st_mode & 0o077
                    or parent_stat.st_uid != os.geteuid()):
                raise QuorumError("witness storage needs an owned private directory")
            try:
                fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_EXCL
                             | getattr(os, "O_NOFOLLOW", 0), 0o600)
            except FileExistsError:
                pass
            else:
                os.close(fd)
            inode = path.lstat()
            if (not stat.S_ISREG(inode.st_mode) or inode.st_mode & 0o077
                    or inode.st_uid != os.geteuid() or inode.st_nlink != 1):
                raise QuorumError("untrusted witness storage inode")
        except OSError as exc:
            raise QuorumError("witness storage unavailable") from exc
        return path

    def close(self) -> None:
        if self._conn is not None:
            self._conn.close()
            self._conn = None

    def __enter__(self):
        if self._conn is None:
            raise QuorumError("witness authority is closed")
        return self

    def __exit__(self, *exc):
        self.close()

    @contextmanager
    def _transaction(self):
        db = self._conn
        if db is None:
            raise QuorumError("witness authority is closed")
        try:
            db.execute("BEGIN IMMEDIATE")
            yield db
            db.execute("COMMIT")
        except BaseException as exc:
            if db.in_transaction:
                db.execute("ROLLBACK")
            if isinstance(exc, sqlite3.Error):
                raise QuorumError("witness storage transaction failed") from exc
            raise

    @contextmanager
    def _operation(self):
        # Persist the clock high-water mark even when a lease decision is denied.
        # Otherwise an observed expiry followed by clock rollback could revive
        # the expired holder. Roll back the action, never the observed clock.
        rejection = None
        with self._transaction() as db:
            row = self._state(db)
            now = self._now(db, row[4])
            db.execute("SAVEPOINT lease_action")
            try:
                yield db, row, now
            except QuorumError as exc:
                db.execute("ROLLBACK TO lease_action")
                rejection = exc
            db.execute("RELEASE lease_action")
        if rejection is not None:
            raise rejection

    def _state(self, db):
        row = db.execute("""SELECT epoch,holder,expires_at,promoted,last_clock,clock_domain
                            FROM witness_state WHERE id=1""").fetchone()
        if row is None:
            raise QuorumError("missing durable witness state")
        epoch, holder, expires_at, promoted, last_clock, domain = row
        if (type(epoch) is not int or not 0 <= epoch <= MAX_EPOCH
                or holder is not None and (not _identifier(holder) or epoch == 0)
                or not _finite_number(expires_at) or not 0 <= expires_at <= MAX_CLOCK_SECONDS
                or type(promoted) is not int or promoted not in (0, 1)
                or holder is None and (promoted != 0 or expires_at != 0)
                or not _valid_clock(last_clock) or domain != self._clock_domain):
            raise QuorumError("invalid witness state or changed clock domain")
        return row

    def _now(self, db, last_clock) -> float:
        # Sample only after acquiring the database writer lock: waiting for a
        # competing process must not let us decide using a stale timestamp.
        try:
            now = self._clock()
        except Exception as exc:
            raise QuorumError("authoritative witness clock unavailable") from exc
        if not _valid_clock(now) or now < last_clock:
            raise QuorumError("invalid or regressed authoritative witness clock")
        db.execute("UPDATE witness_state SET last_clock=? WHERE id=1", (now,))
        return now

    @staticmethod
    def _require_live(row, holder, epoch, now) -> WitnessLease:
        if row[0] != epoch or row[1] != holder or now >= row[2]:
            raise QuorumError("absent, stale, foreign or expired witness lease")
        return WitnessLease(holder, epoch, row[2])

    def grant(self, holder: str, *, ttl_seconds: float) -> WitnessLease:
        if not _identifier(holder):
            raise QuorumError("invalid bounded witness holder")
        _validate_ttl(ttl_seconds)
        with self._operation() as (db, row, now):
            if row[1] is not None and now < row[2]:
                raise QuorumError("a live witness lease already exists; renew or release it")
            if row[0] == MAX_EPOCH:
                raise QuorumError("witness epoch exhausted")
            expiry = now + ttl_seconds
            if expiry <= now:
                raise QuorumError("lease TTL is below authoritative clock resolution")
            lease = WitnessLease(holder, row[0] + 1, expiry)
            db.execute("""UPDATE witness_state SET epoch=?,holder=?,expires_at=?,promoted=0
                          WHERE id=1""", (lease.epoch, holder, expiry))
        return lease

    def renew(self, holder: str, epoch: int, *, ttl_seconds: float) -> WitnessLease:
        _validate_holder_epoch(holder, epoch)
        _validate_ttl(ttl_seconds)
        with self._operation() as (db, row, now):
            self._require_live(row, holder, epoch, now)
            if now + ttl_seconds <= now:
                raise QuorumError("lease TTL is below authoritative clock resolution")
            expiry = max(row[2], now + ttl_seconds)
            db.execute("UPDATE witness_state SET expires_at=? WHERE id=1", (expiry,))
        return WitnessLease(holder, epoch, expiry)

    def release(self, holder: str, epoch: int) -> None:
        _validate_holder_epoch(holder, epoch)
        with self._operation() as (db, row, now):
            self._require_live(row, holder, epoch, now)
            db.execute("""UPDATE witness_state SET holder=NULL,expires_at=0,promoted=0
                          WHERE id=1""")

    def authorize_promotion(self, holder: str, epoch: int) -> WitnessLease:
        _validate_holder_epoch(holder, epoch)
        with self._operation() as (db, row, now):
            lease = self._require_live(row, holder, epoch, now)
            db.execute("UPDATE witness_state SET promoted=1 WHERE id=1")
        return lease

    def authorize_effect(self, holder: str, epoch: int) -> None:
        _validate_holder_epoch(holder, epoch)
        with self._operation() as (db, row, now):
            self._require_live(row, holder, epoch, now)
            if row[3] != 1:
                raise QuorumError("witness holder has not been authorized for promotion")
