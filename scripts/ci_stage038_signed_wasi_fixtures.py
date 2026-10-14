#!/usr/bin/env python3
"""Ephemeral signed WASI and mTLS identities for isolated CI. Not production PKI."""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

DOMAIN = b"OCAF-WASI-MANIFEST-V1\x00"


def openssl(*args: str) -> bytes:
    result = subprocess.run(["openssl", *args], capture_output=True, check=False, timeout=25)
    if result.returncode:
        raise RuntimeError("fixture OpenSSL operation failed")
    return result.stdout


def put(root: Path, name: str, payload: bytes) -> Path:
    path = root / name
    with path.open("xb") as output:
        output.write(payload)
    path.chmod(0o600)
    return path


def stdin_reading_wasi_fixture(message: str) -> bytes:
    """Real core WASIp1 command reading stdin and writing bounded JSON stdout."""
    def leb(value: int) -> bytes:
        out = bytearray()
        while True:
            byte = value & 0x7f
            value >>= 7
            out.append(byte | (0x80 if value else 0))
            if not value:
                return bytes(out)

    def name(value: str) -> bytes:
        raw = value.encode("utf-8")
        return leb(len(raw)) + raw

    def section(blob: bytearray, index: int, payload: bytes) -> None:
        blob.extend(bytes([index]) + leb(len(payload)) + payload)

    wasm = bytearray(bytes([0, 97, 115, 109, 1, 0, 0, 0]))
    section(wasm, 1, bytes([2, 0x60, 4, 0x7f, 0x7f, 0x7f, 0x7f, 1, 0x7f, 0x60, 0, 0]))
    imports = bytearray([2])
    for fn in ("fd_read", "fd_write"):
        imports.extend(name("wasi_snapshot_preview1"))
        imports.extend(name(fn))
        imports.extend([0, 0])
    section(wasm, 2, imports)
    section(wasm, 3, bytes([1, 1]))
    section(wasm, 5, bytes([1, 0, 1]))
    exports = bytes([2]) + name("memory") + bytes([2, 0]) + name("_start") + bytes([0, 2])
    section(wasm, 7, exports)
    body = bytes([
        0, 0x41, 0, 0x41, 0xc0, 0, 0x41, 1, 0x41, 0xd0, 0,
        0x10, 0, 0x1a, 0x41, 1, 0x41, 0, 0x41, 1, 0x41, 8,
        0x10, 1, 0x1a, 0x0b
    ])
    section(wasm, 10, bytes([1]) + leb(len(body)) + body)
    raw = (message + chr(10)).encode("utf-8")
    memory = bytearray(88)
    memory[0:4] = (88).to_bytes(4, "little")
    memory[4:8] = len(raw).to_bytes(4, "little")
    memory[64:68] = (128).to_bytes(4, "little")
    memory[68:72] = (64).to_bytes(4, "little")
    memory.extend(raw)
    section(wasm, 11, bytes([1, 0, 0x41, 0, 0x0b]) + leb(len(memory)) + memory)
    return bytes(wasm)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--directory", type=Path, required=True)
    args = ap.parse_args()
    if os.getenv("GITHUB_ACTIONS") != "true":
        ap.error("CI-only ephemeral PKI")
    if args.directory.exists():
        ap.error("refusing to reuse old identity material")
    old_umask = os.umask(0o077)
    try:
        root = args.directory.resolve()
        root.mkdir(parents=True, mode=0o700)
        wasm = stdin_reading_wasi_fixture('{"ok":true}')
        if not wasm.startswith(bytes([0, 97, 115, 109, 1, 0, 0, 0])):
            raise RuntimeError("generated WASI module header rejected")
        put(root, "module.wasm", wasm)
        manifest = {
            "schema_version": 1,
            "tenant_id": "tenant-ci",
            "agent_id": "agent-a",
            "tool": {
                "name": "ocaf_ci_noop",
                "description": "Synthetic unprivileged WASI admission fixture",
                "input_schema": {"type": "object"},
                "output_schema": {"type": "object"},
                "cost_sats": 0,
                "permissions": [],
                "timeout_ms": 5000,
                "enabled": True
            },
            "module_sha256_hex": hashlib.sha256(wasm).hexdigest(),
            "expires_unix_secs": int(time.time()) + 3600,
            "policy": {
                "fuel": 2000000, "max_linear_memory_bytes": 33554432,
                "max_wasm_stack_bytes": 1048576,
                "max_input_bytes": 16384, "max_output_bytes": 16384,
                "timeout_ms": 5000, "max_concurrent": 4
            }
        }
        raw = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()
        put(root, "manifest.json", raw)
        put(root, "signing-input.bin", DOMAIN + raw)
        openssl("genpkey", "-algorithm", "ED25519", "-out", str(root / "publisher.key"))
        openssl("pkey", "-in", str(root / "publisher.key"), "-pubout",
                "-out", str(root / "publisher.pem"))
        openssl("pkeyutl", "-sign", "-rawin", "-inkey", str(root / "publisher.key"),
                "-in", str(root / "signing-input.bin"), "-out", str(root / "manifest.sig"))
        if (root / "manifest.sig").stat().st_size != 64:
            raise RuntimeError("invalid synthetic signature length")
        openssl("pkeyutl", "-verify", "-pubin", "-inkey", str(root / "publisher.pem"),
                "-rawin", "-in", str(root / "signing-input.bin"),
                "-sigfile", str(root / "manifest.sig"))
        openssl("req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                "-nodes", "-keyout", str(root / "ca.key"),
                "-out", str(root / "ca.pem"), "-days", "1",
                "-subj", "/CN=ocaf-ci-only-root",
                "-addext", "basicConstraints=critical,CA:TRUE",
                "-addext", "keyUsage=critical,keyCertSign,cRLSign")
        peers: dict[str, object] = {}
        for name in ("server", "agent-a", "agent-b"):
            openssl("req", "-new", "-newkey", "ec",
                    "-pkeyopt", "ec_paramgen_curve:P-256", "-nodes",
                    "-keyout", str(root / (name + ".key")),
                    "-out", str(root / (name + ".csr")), "-subj", "/CN=" + name)
            usage = "serverAuth" if name == "server" else "clientAuth"
            san = "localhost" if name == "server" else name + ".test"
            put(root, name + ".ext",
                ("basicConstraints=critical,CA:FALSE\n"
                 "keyUsage=critical,digitalSignature\n"
                 "extendedKeyUsage=" + usage + "\n"
                 "subjectAltName=DNS:" + san + "\n").encode())
            openssl("x509", "-req", "-in", str(root / (name + ".csr")),
                    "-CA", str(root / "ca.pem"), "-CAkey", str(root / "ca.key"),
                    "-CAcreateserial", "-out", str(root / (name + ".pem")),
                    "-days", "1", "-sha256", "-extfile", str(root / (name + ".ext")))
            openssl("verify", "-CAfile", str(root / "ca.pem"),
                    "-purpose", "sslserver" if name == "server" else "sslclient",
                    str(root / (name + ".pem")))
            if name != "server":
                cert_der = openssl("x509", "-in", str(root / (name + ".pem")),
                                   "-outform", "DER")
                peers[name] = {
                    "key_b64": base64.b64encode(os.urandom(32)).decode("ascii"),
                    "allowed_tools": ["ocaf_ci_noop"],
                    "certificate_pem_path": "/var/lib/ocaf/" + name + ".pem",
                    "certificate_sha256": hashlib.sha256(cert_der).hexdigest(),
                    "tls_dns_san": name + ".test",
                }
        put(root, "peers.json", json.dumps(peers, sort_keys=True).encode())
        print(json.dumps({"result": "ephemeral_synthetic_fixtures_generated",
                          "peer_count": len(peers), "certificates_private": True}))
        return 0
    finally:
        os.umask(old_umask)


if __name__ == "__main__":
    raise SystemExit(main())
