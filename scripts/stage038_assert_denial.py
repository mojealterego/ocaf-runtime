#!/usr/bin/env python3
"""Fail closed unless a negative two-host RPC probe reached the application denial boundary.

The positive probe must run first; this validator distinguishes a server-side
RPC rejection from unrelated TLS, relay, DNS, process, or timeout failures.
It never reads or prints private keys, HMAC values, or guest outputs.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import stat
import sys

MAX_DIAGNOSTIC_BYTES = 8192
EXPECTED_DENIAL = b"Error: remote RPC rejected\n"


def private_evidence_file(path: Path) -> bytes:
    """Read only bounded regular files without following symlinks."""
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0))
    with os.fdopen(fd, "rb") as stream:
        metadata = os.fstat(stream.fileno())
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_DIAGNOSTIC_BYTES:
            raise ValueError("diagnostic is not a bounded regular file")
        payload = stream.read(MAX_DIAGNOSTIC_BYTES + 1)
    if len(payload) > MAX_DIAGNOSTIC_BYTES:
        raise ValueError("diagnostic exceeds limit")
    return payload


def validate_denial(exit_code: int, stdout: bytes, stderr: bytes) -> None:
    if type(exit_code) is not int or not 1 <= exit_code <= 255:
        raise ValueError("negative RPC probe did not exit with an application failure")
    if stdout:
        raise ValueError("negative RPC probe unexpectedly emitted a response")
    if stderr != EXPECTED_DENIAL:
        raise ValueError("negative RPC probe did not report the expected server-side rejection")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exit-code", required=True, type=int)
    parser.add_argument("--stdout-file", required=True, type=Path)
    parser.add_argument("--stderr-file", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        validate_denial(args.exit_code,
                        private_evidence_file(args.stdout_file),
                        private_evidence_file(args.stderr_file))
    except (OSError, ValueError) as exc:
        print(f"NO_GO: {exc}", file=sys.stderr)
        return 2
    print("CROSS_PRINCIPAL_APPLICATION_RPC_DENIAL_VERIFIED_NOT_PRODUCTION_GO")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
