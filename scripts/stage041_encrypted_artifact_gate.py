#!/usr/bin/env python3
"""Fail closed before uploading the 0.41 encrypted cross-kernel lab bundle."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re

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


def verify_encrypted_only(root: Path) -> None:
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
        transfer = json.loads((root / "manifest.json").read_bytes(), object_pairs_hook=_unique)
        envelope = json.loads((root / "envelope/manifest.json").read_bytes(),
                              object_pairs_hook=_unique)
    except (OSError, ValueError, UnicodeError, TypeError) as exc:
        raise ArtifactGateError("invalid encrypted transfer manifest") from exc
    if (not isinstance(transfer, dict) or not isinstance(envelope, dict)
            or not isinstance(transfer.get("snapshot_sha256"), str)
            or SHA.fullmatch(transfer["snapshot_sha256"]) is None
            or transfer["snapshot_sha256"] != envelope.get("snapshot_sha256")
            or not isinstance(envelope.get("ciphertext_sha256"), str)
            or SHA.fullmatch(envelope["ciphertext_sha256"]) is None
            or envelope.get("production_approved") is not False
            or envelope.get("immutable_offsite_storage_proved") is not False):
        raise ArtifactGateError("encrypted manifest scope or digest is inconsistent")
    ciphertext = root / "envelope/snapshot.aesgcm"
    if hashlib.sha256(ciphertext.read_bytes()).hexdigest() != envelope["ciphertext_sha256"]:
        raise ArtifactGateError("encrypted payload SHA256 mismatch")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    args = parser.parse_args()
    bundle = args.bundle if args.bundle.is_absolute() else Path.cwd() / args.bundle
    verify_encrypted_only(bundle)
    print("STAGE041_ENCRYPTED_ONLY_ARTIFACT_ALLOWLIST_VERIFIED")


if __name__ == "__main__":
    main()
