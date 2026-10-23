#!/usr/bin/env python3
"""Verify an Ed25519 signed guest workload manifest before VM admission."""
from __future__ import annotations

import base64
import binascii
import json
import re
from typing import Any, Mapping

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

KEY_ID = re.compile(r"[A-Za-z0-9._-]{1,128}\Z")


class SignedManifestError(ValueError):
    pass


def _b64(value: Any, label: str) -> bytes:
    if not isinstance(value, str) or not value:
        raise SignedManifestError(f"missing {label}")
    try:
        raw = base64.b64decode(value.encode("ascii"), validate=True)
    except (ValueError, UnicodeEncodeError, binascii.Error) as exc:
        raise SignedManifestError(f"invalid {label}") from exc
    if label in ("public_key", "signature") and len(raw) != (32 if label == "public_key" else 64):
        raise SignedManifestError(f"invalid {label} length")
    return raw


def canonical_payload(payload: dict[str, Any]) -> bytes:
    if not isinstance(payload, dict) or payload.get("schema_version") != 1:
        raise SignedManifestError("unsupported payload schema")
    return (json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def _unique_fields(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise SignedManifestError("duplicate signed manifest field")
        result[key] = value
    return result


def verify(raw: bytes, trusted_keys: Mapping[str, bytes]) -> dict[str, Any]:
    if not isinstance(raw, bytes) or not 0 < len(raw) <= 256 * 1024:
        raise SignedManifestError("manifest size is outside bounds")
    try:
        envelope = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_fields,
                              parse_constant=lambda _: (_ for _ in ()).throw(SignedManifestError("nonfinite value")))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise SignedManifestError("invalid signed manifest JSON") from exc
    if not isinstance(envelope, dict) or set(envelope) != {"key_id", "public_key", "signature", "payload"}:
        raise SignedManifestError("invalid signed manifest envelope")
    if not isinstance(envelope["key_id"], str) or KEY_ID.fullmatch(envelope["key_id"]) is None:
        raise SignedManifestError("invalid key_id")
    public_key = _b64(envelope["public_key"], "public_key")
    if not isinstance(trusted_keys, Mapping) or trusted_keys.get(envelope["key_id"]) != public_key:
        raise SignedManifestError("signing key is not trusted")
    signature = _b64(envelope["signature"], "signature")
    payload = envelope["payload"]
    message = canonical_payload(payload)
    try:
        Ed25519PublicKey.from_public_bytes(public_key).verify(signature, message)
    except Exception as exc:
        raise SignedManifestError("manifest signature verification failed") from exc
    return {"key_id": envelope["key_id"], "payload": payload, "signed": True}
