#!/usr/bin/env python3
"""0.41.6: AES-256-GCM envelope for bounded PostgreSQL backup bytes.

The caller supplies the 256-bit key through an independent key channel and
enforces its own monotonic minimum generation and revocation list. This module
does not provide immutable offsite storage or durable key distribution.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import stat

from cryptography.exceptions import InvalidTag
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

MAX_SNAPSHOT = 64 * 1024 * 1024
HEX = re.compile(r"[a-f0-9]{64}\Z")
FIELDS = {"schema_version", "algorithm", "key_id", "generation", "nonce_hex",
          "snapshot_sha256", "ciphertext_sha256", "production_approved",
          "immutable_offsite_storage_proved"}


class EncryptedBackupError(ValueError):
    pass


def _unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise EncryptedBackupError("duplicate backup manifest field")
        result[key] = value
    return result


def _regular_bytes(path: Path, limit: int) -> bytes:
    if not isinstance(path, Path) or not path.is_absolute() or path.is_symlink():
        raise EncryptedBackupError("absolute regular backup path required")
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        before = os.fstat(fd)
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or not 0 < before.st_size <= limit):
            raise EncryptedBackupError("unsafe or oversized backup inode")
        content = bytearray()
        while len(content) <= limit:
            part = os.read(fd, min(1024 * 1024, limit + 1 - len(content)))
            if not part:
                break
            content.extend(part)
        after = os.fstat(fd)
        if (len(content) != before.st_size or before.st_size != after.st_size
                or before.st_ino != after.st_ino
                or before.st_mtime_ns != after.st_mtime_ns):
            raise EncryptedBackupError("backup changed while reading")
        return bytes(content)
    finally:
        os.close(fd)


def _key_id(key: bytes) -> str:
    if not isinstance(key, bytes) or len(key) != 32:
        raise EncryptedBackupError("256-bit backup key required")
    return hashlib.sha256(b"OCAF-STAGE041-BACKUP-KEY-V1\0" + key).hexdigest()


def _aad(record: dict) -> bytes:
    return b"OCAF-STAGE041-BACKUP-AEAD-V1\0" + json.dumps(
        {name: record[name] for name in sorted(FIELDS - {"ciphertext_sha256"})},
        sort_keys=True, separators=(",", ":"), allow_nan=False).encode("ascii")


def _write_new(path: Path, data: bytes) -> None:
    if not isinstance(path, Path) or not path.is_absolute() or ".." in path.parts:
        raise EncryptedBackupError("absolute output path required")
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL |
                 getattr(os, "O_NOFOLLOW", 0), 0o600)
    created = os.fstat(fd)
    try:
        with os.fdopen(fd, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
    except BaseException:
        # A failed write or fsync must not leave an incomplete snapshot that
        # a later recovery attempt could mistake for a completed output.
        try:
            current = path.lstat()
            if stat.S_ISREG(current.st_mode) and current.st_ino == created.st_ino:
                path.unlink()
        except OSError:
            pass
        raise


def seal_snapshot(snapshot: Path, envelope_dir: Path, *, key: bytes,
                  generation: int) -> dict:
    key_id = _key_id(key)
    if type(generation) is not int or not 1 <= generation <= 2**63 - 1:
        raise EncryptedBackupError("positive bounded backup generation required")
    if (not isinstance(envelope_dir, Path) or not envelope_dir.is_absolute()
            or envelope_dir.is_symlink() or not envelope_dir.is_dir()
            or any((envelope_dir / name).exists() for name in ("snapshot.aesgcm", "manifest.json"))):
        raise EncryptedBackupError("fresh absolute envelope directory required")
    plaintext = _regular_bytes(snapshot, MAX_SNAPSHOT)
    nonce = os.urandom(12)
    manifest = {"schema_version": 1, "algorithm": "AES-256-GCM",
                "key_id": key_id, "generation": generation,
                "nonce_hex": nonce.hex(),
                "snapshot_sha256": hashlib.sha256(plaintext).hexdigest(),
                "production_approved": False,
                "immutable_offsite_storage_proved": False}
    ciphertext = AESGCM(key).encrypt(nonce, plaintext, _aad(manifest))
    manifest["ciphertext_sha256"] = hashlib.sha256(ciphertext).hexdigest()
    _write_new(envelope_dir / "snapshot.aesgcm", ciphertext)
    _write_new(envelope_dir / "manifest.json",
               (json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n").encode())
    return manifest


def unseal_snapshot(envelope_dir: Path, destination: Path, *, key: bytes,
                    minimum_generation: int, revoked_key_ids: frozenset[str] = frozenset()) -> dict:
    key_id = _key_id(key)
    if (type(minimum_generation) is not int or minimum_generation < 1
            or not isinstance(revoked_key_ids, frozenset)
            or not isinstance(envelope_dir, Path) or not envelope_dir.is_absolute()
            or envelope_dir.is_symlink() or not envelope_dir.is_dir()):
        raise EncryptedBackupError("invalid recovery policy or envelope directory")
    raw = _regular_bytes(envelope_dir / "manifest.json", 4096)
    try:
        manifest = json.loads(raw, object_pairs_hook=_unique)
    except (ValueError, UnicodeError) as exc:
        raise EncryptedBackupError("invalid backup manifest") from exc
    if (not isinstance(manifest, dict) or set(manifest) != FIELDS
            or type(manifest["schema_version"]) is not int or manifest["schema_version"] != 1
            or manifest["algorithm"] != "AES-256-GCM"
            or manifest["key_id"] != key_id or key_id in revoked_key_ids
            or type(manifest["generation"]) is not int
            or not minimum_generation <= manifest["generation"] <= 2**63 - 1
            or not isinstance(manifest["nonce_hex"], str)
            or re.fullmatch(r"[a-f0-9]{24}", manifest["nonce_hex"]) is None
            or any(not isinstance(manifest[name], str) or HEX.fullmatch(manifest[name]) is None
                   for name in ("snapshot_sha256", "ciphertext_sha256"))
            or manifest["production_approved"] is not False
            or manifest["immutable_offsite_storage_proved"] is not False):
        raise EncryptedBackupError("backup key revoked, generation rolled back, or manifest invalid")
    ciphertext = _regular_bytes(envelope_dir / "snapshot.aesgcm", MAX_SNAPSHOT + 16)
    if hashlib.sha256(ciphertext).hexdigest() != manifest["ciphertext_sha256"]:
        raise EncryptedBackupError("ciphertext digest mismatch")
    try:
        plaintext = AESGCM(key).decrypt(bytes.fromhex(manifest["nonce_hex"]),
                                        ciphertext, _aad(manifest))
    except InvalidTag as exc:
        raise EncryptedBackupError("backup authentication failed") from exc
    if hashlib.sha256(plaintext).hexdigest() != manifest["snapshot_sha256"]:
        raise EncryptedBackupError("decrypted backup digest mismatch")
    _write_new(destination, plaintext)
    return {"status": "LOCAL_ENCRYPTED_BACKUP_ROUNDTRIP_PASS",
            "generation": manifest["generation"], "key_id": key_id,
            "snapshot_sha256": manifest["snapshot_sha256"],
            "ciphertext_sha256": manifest["ciphertext_sha256"],
            "immutable_offsite_storage_proved": False,
            "stage_041_complete": False, "production_approved": False}
