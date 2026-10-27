#!/usr/bin/env python3
"""Durable majority-certificate admission gate for PostgreSQL promotion.

The older :mod:`stage040_quorum` authority is intentionally a single local
witness.  This module adds the other half of the boundary: a candidate must
present matching, authenticated votes from a strict majority of the configured
witness set.  The admitted epoch is persisted so an old or conflicting
certificate cannot become valid after process restart.

Ed25519 admission keeps only witness *public* keys on the candidate side. The
older HMAC mode remains for existing local labs, but is unsuitable when the
candidate might be compromised: its verifier holds every signing secret. Key
distribution and remote witness availability are deployment concerns. Neither
mode proves remote failure domains or PostgreSQL server-side fencing.

The persisted membership digest includes witness key fingerprints and the
threshold. Existing state written by the older name-only format fails closed
on reopen; operators must fence the old authority before controlled rebootstrap.
"""
from __future__ import annotations

from contextlib import contextmanager
from dataclasses import asdict, dataclass
import hashlib
import hmac
import json
import math
import os
from pathlib import Path
import re
import sqlite3
import stat
import time
from typing import Mapping, Sequence

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey, Ed25519PublicKey,
)


MAX_EPOCH = 2**63 - 1
MAX_LEASE_SECONDS = 300.0
MAX_CLOCK_SECONDS = 10**12
_IDENTIFIER = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}\Z")
_SIGNATURE = re.compile(r"[a-f0-9]{64}\Z")
_ED25519_SIGNATURE = re.compile(r"[a-f0-9]{128}\Z")


class CertificateError(ValueError):
    """The certificate or durable admission state is unsafe."""


def _identifier(value: object) -> bool:
    return isinstance(value, str) and _IDENTIFIER.fullmatch(value) is not None


def _clock_value(value: object) -> bool:
    return (type(value) in (int, float) and math.isfinite(value)
            and 0 <= value <= MAX_CLOCK_SECONDS)


def _key(value: object) -> bool:
    return isinstance(value, bytes) and 32 <= len(value) <= 128


@dataclass(frozen=True)
class WitnessVote:
    witness_id: str
    cluster_id: str
    candidate_id: str
    epoch: int
    lease_id: str
    issued_at: float
    expires_at: float
    signature: str


@dataclass(frozen=True)
class QuorumReceipt:
    cluster_id: str
    candidate_id: str
    epoch: int
    lease_id: str
    expires_at: float
    certificate_sha256: str
    witness_ids: tuple[str, ...]


def _payload(vote: WitnessVote, *, scheme: str = "hmac-sha256") -> bytes:
    document = {
        "candidate_id": vote.candidate_id,
        "cluster_id": vote.cluster_id,
        "epoch": vote.epoch,
        "expires_at": vote.expires_at,
        "issued_at": vote.issued_at,
        "lease_id": vote.lease_id,
        "witness_id": vote.witness_id,
    }
    domain = (b"OCAF-STAGE040-WITNESS-ED25519-V1\0" if scheme == "ed25519"
              else b"OCAF-STAGE040-WITNESS-V1\0")
    return (domain +
            json.dumps(document, sort_keys=True, separators=(",", ":"),
                       ensure_ascii=True, allow_nan=False).encode("ascii"))


def issue_vote(*, witness_id: str, key: bytes, cluster_id: str,
               candidate_id: str, epoch: int, lease_id: str,
               issued_at: float, expires_at: float) -> WitnessVote:
    """Create one authenticated vote at a witness boundary.

    Production callers should keep this function and the key in the witness
    process.  The admission gate only needs the resulting vote and its own
    configured verifier copy of that witness key.
    """
    unsigned = WitnessVote(witness_id, cluster_id, candidate_id, epoch,
                           lease_id, issued_at, expires_at, "")
    _validate_unsigned(unsigned)
    if not _key(key):
        raise CertificateError("witness key must contain 32..128 bytes")
    signature = hmac.new(key, _payload(unsigned), hashlib.sha256).hexdigest()
    return WitnessVote(**{**asdict(unsigned), "signature": signature})


def issue_vote_ed25519(*, witness_id: str, private_key: Ed25519PrivateKey,
                       cluster_id: str, candidate_id: str, epoch: int,
                       lease_id: str, issued_at: float,
                       expires_at: float) -> WitnessVote:
    """Sign inside one witness authority; never send this key to a candidate."""
    unsigned = WitnessVote(witness_id, cluster_id, candidate_id, epoch,
                           lease_id, issued_at, expires_at, "")
    _validate_unsigned(unsigned)
    if not isinstance(private_key, Ed25519PrivateKey):
        raise CertificateError("Ed25519 witness private key required")
    signature = private_key.sign(_payload(unsigned, scheme="ed25519")).hex()
    return WitnessVote(**{**asdict(unsigned), "signature": signature})


def _validate_unsigned(vote: WitnessVote) -> None:
    if not isinstance(vote, WitnessVote):
        raise CertificateError("typed witness vote required")
    if not all(_identifier(value) for value in (
            vote.witness_id, vote.cluster_id, vote.candidate_id, vote.lease_id)):
        raise CertificateError("invalid bounded vote identity")
    if type(vote.epoch) is not int or not 1 <= vote.epoch <= MAX_EPOCH:
        raise CertificateError("invalid bounded fencing epoch")
    if not _clock_value(vote.issued_at) or not _clock_value(vote.expires_at):
        raise CertificateError("invalid finite vote time")
    if not (vote.issued_at < vote.expires_at
            and vote.expires_at - vote.issued_at <= MAX_LEASE_SECONDS):
        raise CertificateError("invalid bounded witness lease interval")


class QuorumCertificateGate:
    """Verify strict-majority votes and persist the fencing high-water mark."""

    def __init__(self, state_path: Path | str, *, cluster_id: str,
                 witness_keys: Mapping[str, bytes] | None = None,
                 witness_public_keys: Mapping[str, bytes] | None = None,
                 allow_shared_secret_lab: bool = False,
                 threshold: int | None = None,
                 clock=time.monotonic, clock_domain: str | None = None):
        if not _identifier(cluster_id):
            raise CertificateError("invalid cluster identity")
        if (witness_keys is None) == (witness_public_keys is None):
            raise CertificateError("select exactly one witness signature scheme")
        self._scheme = "ed25519" if witness_public_keys is not None else "hmac-sha256"
        if self._scheme == "hmac-sha256" and allow_shared_secret_lab is not True:
            raise CertificateError("shared-secret HMAC mode requires explicit lab opt-in")
        if self._scheme == "ed25519" and allow_shared_secret_lab is not False:
            raise CertificateError("lab opt-in cannot accompany public-key quorum")
        keys = witness_public_keys if witness_public_keys is not None else witness_keys
        if (not isinstance(keys, Mapping) or len(keys) < 3
                or any(not _identifier(name) or not isinstance(key, bytes)
                       or (len(key) != 32 if self._scheme == "ed25519" else not _key(key))
                       for name, key in keys.items())):
            raise CertificateError("at least three bounded independent witness keys required")
        if self._scheme == "ed25519":
            try:
                for key in keys.values():
                    Ed25519PublicKey.from_public_bytes(key)
            except ValueError as exc:
                raise CertificateError("invalid Ed25519 witness public key") from exc
        # A majority counts independent authorities, not labels. Duplicate
        # key material lets one signer impersonate multiple witness IDs.
        if len(set(keys.values())) != len(keys):
            raise CertificateError("witness keys must be distinct per authority")
        majority = len(keys) // 2 + 1
        if threshold is None:
            threshold = majority
        if type(threshold) is not int or not majority <= threshold <= len(keys):
            raise CertificateError("threshold must be a strict configured majority")
        if not callable(clock):
            raise CertificateError("authoritative clock must be callable")
        if clock_domain is None:
            if clock is not time.monotonic:
                raise CertificateError("injected clock needs an explicit clock domain")
            try:
                clock_domain = "linux-boot:" + Path(
                    "/proc/sys/kernel/random/boot_id").read_text(encoding="ascii").strip()
            except (OSError, UnicodeError) as exc:
                raise CertificateError("cannot establish authoritative clock domain") from exc
        if not _identifier(clock_domain):
            raise CertificateError("invalid bounded clock domain")

        self._cluster_id = cluster_id
        self._keys = dict(keys)
        self._threshold = threshold
        self._clock = clock
        self._clock_domain = clock_domain
        # The durable authority must not silently change its trust roots or
        # voting rule after restart. A digest of witness names alone would
        # accept same-ID key rotation, and a separate threshold change could
        # weaken a previously admitted certificate's authority domain.
        membership = {
            "threshold": threshold,
            "witnesses": [(name, hashlib.sha256(self._keys[name]).hexdigest())
                          for name in sorted(self._keys)],
        }
        membership_domain = (b"OCAF-STAGE040-WITNESS-MEMBERSHIP-ED25519-V1\0"
                             if self._scheme == "ed25519"
                             else b"OCAF-STAGE040-WITNESS-MEMBERSHIP-V2\0")
        self._membership_sha256 = hashlib.sha256(
            membership_domain +
            json.dumps(membership, sort_keys=True, separators=(",", ":"),
                       ensure_ascii=True).encode("ascii")).hexdigest()
        self.path = self._private_path(state_path)
        self._conn: sqlite3.Connection | None = None
        try:
            self._conn = sqlite3.connect(str(self.path), isolation_level=None, timeout=5)
            self._conn.execute("PRAGMA trusted_schema=OFF")
            self._conn.execute("PRAGMA journal_mode=WAL")
            self._conn.execute("PRAGMA synchronous=FULL")
            self._conn.execute("PRAGMA busy_timeout=5000")
            with self._transaction() as db:
                version = db.execute("PRAGMA user_version").fetchone()[0]
                if version not in (0, 1):
                    raise CertificateError("unsupported quorum database version")
                if version == 0:
                    if db.execute("SELECT name FROM sqlite_master").fetchone() is not None:
                        raise CertificateError("refusing unrelated quorum database")
                    db.execute("""CREATE TABLE quorum_state (
                        id INTEGER PRIMARY KEY CHECK(id=1),
                        high_epoch INTEGER NOT NULL CHECK(high_epoch>=0),
                        certificate_sha256 TEXT,
                        candidate_id TEXT,
                        lease_id TEXT,
                        expires_at REAL NOT NULL,
                        last_clock REAL NOT NULL,
                        clock_domain TEXT NOT NULL,
                        cluster_id TEXT NOT NULL,
                        membership_sha256 TEXT NOT NULL)""")
                    db.execute("INSERT INTO quorum_state VALUES(1,0,NULL,NULL,NULL,0,0,?,?,?)",
                               (clock_domain, cluster_id, self._membership_sha256))
                    db.execute("PRAGMA user_version=1")
                self._state(db)
        except (sqlite3.Error, CertificateError) as exc:
            self.close()
            raise CertificateError(f"quorum state unavailable: {exc}") from exc

    @staticmethod
    def _private_path(value: Path | str) -> Path:
        if not isinstance(value, (str, Path)):
            raise CertificateError("quorum storage path required")
        path = Path(value)
        if not path.is_absolute() or ".." in path.parts:
            raise CertificateError("absolute quorum storage path required")
        try:
            parent = path.parent
            info = parent.stat()
            if (not parent.is_dir() or parent.is_symlink() or info.st_uid != os.geteuid()
                    or info.st_mode & 0o077 or parent.resolve(strict=True) != parent):
                raise CertificateError("quorum storage needs an owned private directory")
            # SQLite may open existing WAL/SHM siblings before creating its
            # own files. Reject aliases to another authority file.
            for sibling in (path, Path(str(path) + "-wal"), Path(str(path) + "-shm")):
                try:
                    sidecar = sibling.lstat()
                except FileNotFoundError:
                    continue
                if (not stat.S_ISREG(sidecar.st_mode) or sidecar.st_uid != os.geteuid()
                        or sidecar.st_mode & 0o077 or sidecar.st_nlink != 1):
                    raise CertificateError("untrusted quorum SQLite inode")
            try:
                fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_EXCL
                             | getattr(os, "O_NOFOLLOW", 0), 0o600)
            except FileExistsError:
                pass
            else:
                os.close(fd)
            inode = path.lstat()
            if (not stat.S_ISREG(inode.st_mode) or inode.st_uid != os.geteuid()
                    or inode.st_mode & 0o077 or inode.st_nlink != 1):
                raise CertificateError("untrusted quorum storage inode")
        except OSError as exc:
            raise CertificateError("quorum storage unavailable") from exc
        return path

    @contextmanager
    def _transaction(self):
        if self._conn is None:
            raise CertificateError("quorum gate is closed")
        try:
            self._conn.execute("BEGIN IMMEDIATE")
            yield self._conn
            self._conn.execute("COMMIT")
        except BaseException as exc:
            if self._conn is not None and self._conn.in_transaction:
                self._conn.execute("ROLLBACK")
            if isinstance(exc, sqlite3.Error):
                raise CertificateError("quorum storage transaction failed") from exc
            raise

    def _state(self, db: sqlite3.Connection):
        row = db.execute("""SELECT high_epoch,certificate_sha256,candidate_id,lease_id,
                            expires_at,last_clock,clock_domain,cluster_id,membership_sha256
                            FROM quorum_state WHERE id=1""").fetchone()
        if row is None:
            raise CertificateError("missing durable quorum state")
        if (type(row[0]) is not int or not 0 <= row[0] <= MAX_EPOCH
                or not _clock_value(row[4]) or not _clock_value(row[5])
                or row[6] != self._clock_domain or row[7] != self._cluster_id
                or row[8] != self._membership_sha256
                or (row[0] == 0 and any(value is not None for value in row[1:4]))
                or (row[0] > 0 and (not isinstance(row[1], str)
                                    or not _identifier(row[2]) or not _identifier(row[3])))):
            raise CertificateError("invalid quorum state or changed authority domain")
        return row

    def _now(self, db: sqlite3.Connection, last_clock: float) -> float:
        try:
            now = self._clock()
        except Exception as exc:
            raise CertificateError("authoritative clock unavailable") from exc
        if not _clock_value(now) or now < last_clock:
            raise CertificateError("invalid or regressed authoritative clock")
        db.execute("UPDATE quorum_state SET last_clock=? WHERE id=1", (now,))
        return now

    def admit(self, votes: Sequence[WitnessVote]) -> QuorumReceipt:
        if not isinstance(votes, Sequence) or isinstance(votes, (str, bytes)):
            raise CertificateError("witness vote sequence required")
        votes = tuple(votes)
        if not self._threshold <= len(votes) <= len(self._keys):
            raise CertificateError("certificate lacks a strict majority")
        witness_ids: set[str] = set()
        reference = None
        for vote in votes:
            _validate_unsigned(vote)
            if vote.cluster_id != self._cluster_id:
                raise CertificateError("vote belongs to another cluster")
            if vote.witness_id in witness_ids:
                raise CertificateError("duplicate witness vote")
            witness_ids.add(vote.witness_id)
            key = self._keys.get(vote.witness_id)
            signature_pattern = (_ED25519_SIGNATURE if self._scheme == "ed25519"
                                 else _SIGNATURE)
            if key is None or not isinstance(vote.signature, str) or not signature_pattern.fullmatch(vote.signature):
                raise CertificateError("unknown witness or malformed signature")
            if self._scheme == "ed25519":
                try:
                    Ed25519PublicKey.from_public_bytes(key).verify(
                        bytes.fromhex(vote.signature), _payload(vote, scheme="ed25519"))
                except (InvalidSignature, ValueError) as exc:
                    raise CertificateError("invalid witness signature") from exc
            else:
                expected = hmac.new(key, _payload(vote), hashlib.sha256).hexdigest()
                if not hmac.compare_digest(expected, vote.signature):
                    raise CertificateError("invalid witness signature")
            common = (vote.cluster_id, vote.candidate_id, vote.epoch, vote.lease_id,
                      vote.issued_at, vote.expires_at)
            if reference is None:
                reference = common
            elif common != reference:
                raise CertificateError("witnesses did not sign one identical lease")

        ordered = sorted(votes, key=lambda vote: vote.witness_id)
        certificate_bytes = json.dumps(
            [asdict(vote) for vote in ordered], sort_keys=True, separators=(",", ":"),
            ensure_ascii=True, allow_nan=False).encode("ascii")
        certificate_sha256 = hashlib.sha256(certificate_bytes).hexdigest()
        candidate = ordered[0].candidate_id
        epoch = ordered[0].epoch
        lease_id = ordered[0].lease_id
        issued_at = ordered[0].issued_at
        expires_at = ordered[0].expires_at

        rejection: CertificateError | None = None
        with self._transaction() as db:
            state = self._state(db)
            now = self._now(db, state[5])
            db.execute("SAVEPOINT admission")
            try:
                if not issued_at <= now < expires_at:
                    raise CertificateError("quorum certificate is not currently live")
                if epoch < state[0]:
                    raise CertificateError("stale quorum fencing epoch")
                if epoch == state[0]:
                    if certificate_sha256 != state[1]:
                        raise CertificateError("conflicting certificate at admitted epoch")
                else:
                    db.execute("""UPDATE quorum_state
                                  SET high_epoch=?,certificate_sha256=?,candidate_id=?,
                                      lease_id=?,expires_at=? WHERE id=1""",
                               (epoch, certificate_sha256, candidate, lease_id, expires_at))
            except CertificateError as exc:
                db.execute("ROLLBACK TO admission")
                rejection = exc
            db.execute("RELEASE admission")
        if rejection is not None:
            raise rejection
        return QuorumReceipt(self._cluster_id, candidate, epoch, lease_id, expires_at,
                             "sha256:" + certificate_sha256, tuple(sorted(witness_ids)))

    def close(self) -> None:
        if self._conn is not None:
            self._conn.close()
            self._conn = None

    def __enter__(self):
        if self._conn is None:
            raise CertificateError("quorum gate is closed")
        return self

    def __exit__(self, *_exc):
        self.close()
