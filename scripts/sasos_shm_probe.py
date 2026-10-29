#!/usr/bin/env python3
"""Linux memfd cross-process SHM evidence; NOT Zenoh SHM or inter-host zero-copy."""
from __future__ import annotations

import argparse
import hashlib
import json
import mmap
import os
import secrets
import subprocess
import sys

SIZE = 65_536
HEADER = 32
ACK_OFFSET = 0
NONCE_OFFSET = 1
PAYLOAD_OFFSET = HEADER


class ShmProbeError(RuntimeError):
    pass


def child(fd: int, size: int, nonce: str, digest: str) -> int:
    if size != SIZE or len(nonce) != 32 or len(digest) != 64:
        return 2
    try:
        with mmap.mmap(fd, SIZE, access=mmap.ACCESS_WRITE) as memory:
            if bytes(memory[NONCE_OFFSET:NONCE_OFFSET + 16]).hex() != nonce:
                return 3
            if hashlib.sha256(memory[PAYLOAD_OFFSET:]).hexdigest() != digest:
                return 4
            memory[ACK_OFFSET] = 0xA5
            memory.flush()
        return 0
    except (OSError, ValueError, BufferError):
        return 5


def run_probe(payload_size: int = SIZE - PAYLOAD_OFFSET) -> dict:
    if sys.platform != "linux" or not hasattr(os, "memfd_create"):
        raise ShmProbeError("Linux memfd required")
    if payload_size != SIZE - PAYLOAD_OFFSET:
        raise ShmProbeError("invalid shared memory payload length")
    fd = os.memfd_create("ocaf334-shm", flags=getattr(os, "MFD_CLOEXEC", 0))
    try:
        os.ftruncate(fd, SIZE)
        with mmap.mmap(fd, SIZE, access=mmap.ACCESS_WRITE) as memory:
            nonce = secrets.token_bytes(16)
            payload = secrets.token_bytes(payload_size)
            memory[NONCE_OFFSET:NONCE_OFFSET + 16] = nonce
            memory[PAYLOAD_OFFSET:] = payload
            digest = hashlib.sha256(payload).hexdigest()

            def invoke() -> subprocess.CompletedProcess:
                return subprocess.run(
                    [sys.executable, __file__, "--child", str(fd), str(SIZE),
                     nonce.hex(), digest],
                    pass_fds=(fd,), capture_output=True, text=True, timeout=10,
                    check=False,
                )

            accepted = invoke()
            if accepted.returncode != 0 or memory[ACK_OFFSET] != 0xA5:
                raise ShmProbeError("separate process failed to read shared segment")
            memory[ACK_OFFSET] = 0
            memory[PAYLOAD_OFFSET] ^= 0xFF
            rejected = invoke()
            if rejected.returncode == 0 or memory[ACK_OFFSET] != 0:
                raise ShmProbeError("tampered shared segment was accepted")
            memory[PAYLOAD_OFFSET] ^= 0xFF
            if invoke().returncode != 0:
                raise ShmProbeError("segment failed to recover after restoration")
        return {
            "status": "LINUX_MEMFD_TWO_PROCESS_PROBE",
            "separate_process_verified": True,
            "tampered_payload_rejected": True,
            "same_host_only": True,
            "zenoh_shm_e2e_verified": False,
            "cross_host_zero_copy_verified": False,
            "bytes": payload_size,
            "payload_sha256": digest,
        }
    except (OSError, subprocess.SubprocessError) as error:
        raise ShmProbeError("Linux shared-memory probe failed") from error
    finally:
        os.close(fd)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--child", nargs=4, metavar=("FD", "SIZE", "NONCE", "DIGEST"))
    parser.add_argument("--report")
    args = parser.parse_args()
    if args.child is not None:
        fd, size, nonce, digest = args.child
        return child(int(fd), int(size), nonce, digest)
    try:
        result = run_probe()
        encoded = json.dumps(result, sort_keys=True) + "\n"
        if args.report:
            with open(args.report, "w", encoding="utf-8") as file:
                file.write(encoded)
        print(encoded, end="")
        return 0
    except ShmProbeError as error:
        print(f"SASOS_SHM_FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
