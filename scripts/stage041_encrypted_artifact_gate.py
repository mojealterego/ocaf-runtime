#!/usr/bin/env python3
"""Fail closed before uploading the 0.41 encrypted cross-kernel lab bundle."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re

from cryptography.exceptions import InvalidTag
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

import stage041_encrypted_backup as encrypted
import stage041_backup_restore as backup
import stage041_cross_host_restore as recovery

ALLOWED = frozenset({"source.json", "manifest.json", "envelope/manifest.json",
                     "envelope/snapshot.aesgcm"})
SHA = re.compile(r"[a-f0-9]{64}\Z")


class ArtifactGateError(ValueError):
    pass


def _unique(pairs):
    record = {}
    for key, value in pairs:
        if key in record:
            raise ArtifactGateError("duplicate manifest key")
        record[key] = value
    return record


def verify_encrypted_only(root: Path, *, key: bytes) -> None:
    key_id = encrypted._key_id(key)
    if not isinstance(root, Path) or not root.is_absolute() or root.is_symlink() or not root.is_dir():
        raise ArtifactGateError("absolute regular encrypted bundle directory required")
    actual = {str(path.relative_to(root)) for path in root.rglob("*")
              if path.is_file() or path.is_symlink()}
    if actual != ALLOWED:
        raise ArtifactGateError("encrypted bundle has missing, extra or plaintext files")
    for name in ALLOWED:
        path = root / name
        if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
            raise ArtifactGateError("unsafe or empty encrypted bundle member")
    try:
        transfer = json.loads(encrypted._regular_bytes(root / "manifest.json", 4096),
                              object_pairs_hook=_unique)
        envelope = json.loads(encrypted._regular_bytes(root / "envelope/manifest.json", 4096),
                              object_pairs_hook=_unique)
        source_bytes = encrypted._regular_bytes(root / "source.json", 131072)
        source = json.loads(source_bytes, object_pairs_hook=_unique)
    except (OSError, ValueError, UnicodeError, TypeError) as exc:
        raise ArtifactGateError("invalid encrypted transfer manifest") from exc
    if (not isinstance(transfer, dict) or not isinstance(envelope, dict)
            or not isinstance(source, dict)
            or set(transfer) != recovery.MANIFEST_FIELDS
            or type(transfer["schema_version"]) is not int or transfer["schema_version"] != 1
            or transfer["production_approved"] is not False
            or not isinstance(transfer["source_boot_id"], str)
            or recovery.UUID.fullmatch(transfer["source_boot_id"]) is None
            or not isinstance(transfer["run_nonce"], str)
            or recovery.NONCE.fullmatch(transfer["run_nonce"]) is None
            or not isinstance(transfer["source_report_sha256"], str)
            or SHA.fullmatch(transfer["source_report_sha256"]) is None
            or transfer["source_report_sha256"] != hashlib.sha256(source_bytes).hexdigest()
            or type(transfer["snapshot_size_bytes"]) is not int
            or not 5 < transfer["snapshot_size_bytes"] <= encrypted.MAX_SNAPSHOT
            or source.get("backup_sha256") != transfer.get("snapshot_sha256")
            or source.get("backup_size_bytes") != transfer["snapshot_size_bytes"]
            or source.get("original_rows") != 25
            or source.get("backup_file") != "snapshot.pgcustom"
            or source.get("same_kernel_source_and_restore") is not True
            or source.get("cross_host_restore_tested") is not False
            or source.get("stage_041_complete") is not False
            or set(envelope) != encrypted.FIELDS
            or not isinstance(transfer.get("snapshot_sha256"), str)
            or SHA.fullmatch(transfer["snapshot_sha256"]) is None
            or transfer["snapshot_sha256"] != envelope.get("snapshot_sha256")
            or envelope.get("algorithm") != "AES-256-GCM"
            or envelope.get("key_id") != key_id
            or not isinstance(envelope.get("nonce_hex"), str)
            or re.fullmatch(r"[a-f0-9]{24}", envelope["nonce_hex"]) is None
            or not isinstance(envelope.get("ciphertext_sha256"), str)
            or SHA.fullmatch(envelope["ciphertext_sha256"]) is None
            or envelope.get("production_approved") is not False
            or envelope.get("immutable_offsite_storage_proved") is not False):
        raise ArtifactGateError("encrypted manifest scope or digest is inconsistent")
    try:
        backup.verify_restore(source)
    except (ValueError, TypeError) as exc:
        raise ArtifactGateError("source PostgreSQL restore evidence is invalid") from exc
    try:
        ciphertext = encrypted._regular_bytes(root / "envelope/snapshot.aesgcm",
                                              encrypted.MAX_SNAPSHOT + 16)
    except (encrypted.EncryptedBackupError, OSError) as exc:
        raise ArtifactGateError("unsafe encrypted payload inode") from exc
    if hashlib.sha256(ciphertext).hexdigest() != envelope["ciphertext_sha256"]:
        raise ArtifactGateError("encrypted payload SHA256 mismatch")
    try:
        plaintext = AESGCM(key).decrypt(bytes.fromhex(envelope["nonce_hex"]),
                                        ciphertext, encrypted._aad(envelope))
    except InvalidTag as exc:
        raise ArtifactGateError("encrypted payload authentication failed") from exc
    if (not plaintext.startswith(b"PGDMP")
            or len(plaintext) != transfer["snapshot_size_bytes"]
            or hashlib.sha256(plaintext).hexdigest() != transfer["snapshot_sha256"]):
        raise ArtifactGateError("encrypted payload is not the pinned PostgreSQL dump")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    args = parser.parse_args()
    bundle = args.bundle if args.bundle.is_absolute() else Path.cwd() / args.bundle
    try:
        key = bytes.fromhex(os.environ["STAGE041_KEY_HEX"])
    except (KeyError, ValueError) as exc:
        raise ArtifactGateError("ephemeral encryption key unavailable") from exc
    verify_encrypted_only(bundle, key=key)
    print("STAGE041_ENCRYPTED_ONLY_ARTIFACT_ALLOWLIST_VERIFIED")


if __name__ == "__main__":
    main()
