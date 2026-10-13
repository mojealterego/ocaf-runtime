#!/usr/bin/env python3
"""Create a detached Ed25519 signature for a trusted WASI deployment manifest.

The signing private key stays outside the repository. The OCAF runtime verifies
exactly DOMAIN + original JSON file bytes, without JSON re-serialization.
"""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import stat
import subprocess
import sys
import tempfile

DOMAIN = b"OCAF-WASI-MANIFEST-V1\x00"
MAX_MANIFEST = 24 * 1024


def unique_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    output: dict[str, object] = {}
    for key, value in pairs:
        if key in output:
            raise ValueError("duplicate JSON object key")
        output[key] = value
    return output


def private_file(path: pathlib.Path, *, max_size: int) -> bytes:
    if not path.is_absolute():
        raise ValueError("all signing paths must be absolute")
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise ValueError("expected an ordinary single-linked file")
    if stat.S_IMODE(info.st_mode) & 0o077:
        raise ValueError("signing files must be private (mode 0600)")
    if not 0 < info.st_size <= max_size:
        raise ValueError("input size outside allowed bounds")
    raw = path.read_bytes()
    if not 0 < len(raw) <= max_size:
        raise ValueError("input changed or exceeded size bound")
    return raw


def sign_manifest(private_key: pathlib.Path, manifest: pathlib.Path, output: pathlib.Path) -> None:
    private_file(private_key, max_size=16 * 1024)
    raw = private_file(manifest, max_size=MAX_MANIFEST)
    json.loads(raw.decode("utf-8"), object_pairs_hook=unique_keys)
    if not output.is_absolute():
        raise ValueError("output path must be absolute")
    os.lstat(output) if output.exists() or output.is_symlink() else None
    with tempfile.TemporaryDirectory(prefix="ocaf-wasi-sign-") as temp:
        payload_file = pathlib.Path(temp) / "signed-payload"
        payload_file.write_bytes(DOMAIN + raw)
        process = subprocess.run(
            [
                "openssl", "pkeyutl", "-sign", "-rawin",
                "-inkey", str(private_key), "-in", str(payload_file),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=10,
        )
        if process.returncode != 0 or len(process.stdout) != 64:
            raise ValueError("OpenSSL Ed25519 signing failed")
        fd = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "wb") as result:
            result.write(process.stdout)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--private-key", type=pathlib.Path, required=True)
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--signature", type=pathlib.Path, required=True)
    args = parser.parse_args()
    try:
        sign_manifest(args.private_key, args.manifest, args.signature)
        print("Detached WASI manifest signature written; no private key exported.")
        return 0
    except (ValueError, OSError, UnicodeError, subprocess.TimeoutExpired) as error:
        print(f"WASI manifest signing rejected: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
