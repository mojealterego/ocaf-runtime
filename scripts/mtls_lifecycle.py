#!/usr/bin/env python3
"""Fail-closed, read-only OpenSSL mTLS certificate/rotation preflight.

Operator gate, NOT an automated hot-reload, CRL/OCSP check, or proof that
Zenoh binds a certificate's SAN to an authenticated RPC peer identity.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys

MAX_PEM_BYTES = 1024 * 1024
MAX_VALIDITY_SECONDS = 365 * 24 * 60 * 60
MAX_CMD_BYTES = 1024 * 1024
DNS_LABEL = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\Z")


class MtlsError(ValueError):
    """The certificate is not acceptable for the requested identity."""


@dataclass(frozen=True)
class CheckRequest:
    role: str
    cert: Path
    key: Path
    ca: Path
    expected_dns: str | None
    expected_ip: str | None
    min_valid_seconds: int
    previous_cert: Path | None = None


def _file(path: Path, *, private: bool) -> None:
    if not path.is_absolute():
        raise MtlsError("certificate material must have an absolute path")
    try:
        info = path.lstat()
    except OSError as exc:
        raise MtlsError("certificate material unavailable") from exc
    if not stat.S_ISREG(info.st_mode) or not 0 < info.st_size <= MAX_PEM_BYTES:
        raise MtlsError("certificate material must be a bounded regular file")
    if info.st_mode & 0o022:
        raise MtlsError("certificate files may not be group/world-writable")
    if private and (info.st_mode & 0o077 or info.st_uid != os.geteuid()):
        raise MtlsError("private key must be owned by this user and owner-only")


def _run(*parts: str) -> bytes:
    try:
        result = subprocess.run(
            ["openssl", *parts], check=False, capture_output=True, timeout=10,
            stdin=subprocess.DEVNULL,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise MtlsError("cryptographic preflight unavailable") from exc
    if result.returncode or not result.stdout or len(result.stdout) > MAX_CMD_BYTES:
        # Do not echo OpenSSL stderr, which may reveal local paths and
        # private operator configuration.
        raise MtlsError("cryptographic validation failed")
    return result.stdout


def _validated_identity(request: CheckRequest) -> tuple[str, str]:
    if request.role not in ("server", "client"):
        raise MtlsError("role must be server or client")
    if (request.expected_dns is None) == (request.expected_ip is None):
        raise MtlsError("require precisely one expected DNS or IP identity")
    if request.expected_dns is not None:
        name = request.expected_dns
        if len(name) > 253 or not name or any(
            DNS_LABEL.fullmatch(label) is None for label in name.split(".")
        ):
            raise MtlsError("invalid exact DNS identity")
        return "DNS", name.lower()
    try:
        identity = str(ipaddress.ip_address(request.expected_ip))
    except ValueError as exc:
        raise MtlsError("invalid IP identity") from exc
    return "IP", identity


def _identity_san(cert: Path, kind: str, identity: str) -> None:
    raw = _run("x509", "-in", str(cert), "-noout", "-ext", "subjectAltName")
    try:
        fields = raw.decode("ascii").splitlines()[1:]
    except UnicodeDecodeError as exc:
        raise MtlsError("invalid certificate SAN extension") from exc
    names = [field.strip() for field in ",".join(fields).split(",")]
    label = "DNS:" if kind == "DNS" else "IP Address:"
    matched = False
    for entry in names:
        if not entry.startswith(label):
            continue
        value = entry[len(label):].strip()
        if kind == "DNS":
            matched |= value.lower() == identity
        else:
            try:
                matched |= str(ipaddress.ip_address(value)) == identity
            except ValueError:
                raise MtlsError("invalid certificate IP SAN") from None
    if not matched:
        raise MtlsError("exact subject alternative name missing")


def _fingerprint(cert: Path) -> str:
    value = _run("x509", "-in", str(cert), "-noout", "-fingerprint", "-sha256")
    try:
        text = value.decode("ascii").strip()
    except UnicodeDecodeError as exc:
        raise MtlsError("invalid certificate fingerprint") from exc
    hex_part = text.rsplit("=", 1)[-1].replace(":", "").lower()
    if re.fullmatch(r"[0-9a-f]{64}", hex_part) is None:
        raise MtlsError("invalid certificate fingerprint")
    return hex_part


def check(request: CheckRequest) -> dict[str, object]:
    """Validate trust, EKU, exact SAN, key pairing and minimum remaining life."""
    kind, identity = _validated_identity(request)
    if type(request.min_valid_seconds) is not int or not 0 <= request.min_valid_seconds <= MAX_VALIDITY_SECONDS:
        raise MtlsError("invalid certificate remaining-lifetime policy")
    _file(request.cert, private=False)
    _file(request.key, private=True)
    _file(request.ca, private=False)
    if request.previous_cert is not None:
        _file(request.previous_cert, private=False)

    purpose = "sslserver" if request.role == "server" else "sslclient"
    verify_identity = "-verify_hostname" if kind == "DNS" else "-verify_ip"
    _run("verify", "-CAfile", str(request.ca), "-purpose", purpose,
         verify_identity, identity, str(request.cert))
    _identity_san(request.cert, kind, identity)
    _run("x509", "-in", str(request.cert), "-noout", "-checkend", str(request.min_valid_seconds))
    _run("x509", "-in", str(request.ca), "-noout", "-checkend", str(request.min_valid_seconds))
    certificate_pubkey = _run("x509", "-in", str(request.cert), "-noout", "-pubkey").strip()
    private_pubkey = _run("pkey", "-in", str(request.key), "-pubout", "-passin", "pass:").strip()
    if hashlib.sha256(certificate_pubkey).digest() != hashlib.sha256(private_pubkey).digest():
        raise MtlsError("certificate and private key do not match")

    digest = _fingerprint(request.cert)
    previous = None
    if request.previous_cert is not None:
        previous = _fingerprint(request.previous_cert)
        if digest == previous:
            raise MtlsError("rotation requires a distinct leaf certificate")
    receipt: dict[str, object] = {
        "status": "PASS",
        "role": request.role,
        "identity_kind": kind,
        "identity": identity,
        "certificate_sha256": digest,
        "min_valid_seconds": request.min_valid_seconds,
        "rotation_candidate": previous is not None,
        "scope": "read-only OpenSSL PKI preflight; not revocation or runtime peer identity binding",
    }
    if previous is not None:
        receipt["previous_certificate_sha256"] = previous
    return receipt


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--role", required=True, choices=("server", "client"))
    parser.add_argument("--cert", required=True, type=Path)
    parser.add_argument("--key", required=True, type=Path)
    parser.add_argument("--ca", required=True, type=Path)
    identity = parser.add_mutually_exclusive_group(required=True)
    identity.add_argument("--expected-dns")
    identity.add_argument("--expected-ip")
    parser.add_argument("--min-valid-seconds", type=int, default=86400)
    parser.add_argument("--previous-cert", type=Path)
    args = parser.parse_args(argv)
    try:
        result = check(CheckRequest(
            role=args.role, cert=args.cert, key=args.key, ca=args.ca,
            expected_dns=args.expected_dns, expected_ip=args.expected_ip,
            min_valid_seconds=args.min_valid_seconds, previous_cert=args.previous_cert,
        ))
        print(json.dumps(result, sort_keys=True))
        return 0
    except MtlsError as exc:
        print(json.dumps({"status": "FAIL", "reason": str(exc)}, sort_keys=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
