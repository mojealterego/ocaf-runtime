#!/usr/bin/env python3
"""Create two *separate* ephemeral, signed WASI tenant fixtures for Linux CI.

No trusted release keys, production secrets or external side effects are used.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from ci_stage038_signed_wasi_fixtures import DOMAIN, stdin_reading_wasi_fixture


def rewrite_signed_tenant(root: Path, tenant: str, principal: str, tool: str) -> None:
    module = stdin_reading_wasi_fixture(
        json.dumps({"ok": True, "tenant": tenant}, separators=(",", ":"))
    )
    (root / "module.wasm").write_bytes(module)
    manifest_path = root / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["tenant_id"] = tenant
    manifest["agent_id"] = principal
    manifest["tool"]["name"] = tool
    manifest["module_sha256_hex"] = hashlib.sha256(module).hexdigest()
    raw = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode("utf-8")
    manifest_path.write_bytes(raw)
    (root / "signing-input.bin").write_bytes(DOMAIN + raw)
    subprocess.run(
        ["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", str(root / "publisher.key"),
         "-in", str(root / "signing-input.bin"), "-out", str(root / "manifest.sig")],
        check=True, capture_output=True, timeout=20,
    )
    peers_path = root / "peers.json"
    peers = json.loads(peers_path.read_text(encoding="utf-8"))
    if set(peers) != {"agent-a", "agent-b"}:
        raise ValueError("unexpected per-tenant principal set")
    for peer, config in peers.items():
        config["allowed_tools"] = [tool] if peer == principal else []
    peers_path.write_text(json.dumps(peers, sort_keys=True), encoding="utf-8")
    for name in ("module.wasm", "manifest.json", "signing-input.bin",
                 "manifest.sig", "peers.json"):
        (root / name).chmod(0o600)
    subprocess.run(
        ["openssl", "pkeyutl", "-verify", "-rawin", "-pubin", "-inkey",
         str(root / "publisher.pem"), "-in", str(root / "signing-input.bin"),
         "-sigfile", str(root / "manifest.sig")],
        check=True, capture_output=True, timeout=20,
    )


def create_fixtures(directory: Path) -> None:
    if os.environ.get("GITHUB_ACTIONS") != "true":
        raise ValueError("only ephemeral GitHub Actions runners are supported")
    if directory.exists():
        raise ValueError("cannot reuse existing private fixture directory")
    directory.mkdir(mode=0o700, parents=True)
    script = Path(__file__).with_name("ci_stage038_signed_wasi_fixtures.py")
    for tenant in ("tenant-a", "tenant-b"):
        subprocess.run([sys.executable, str(script), "--directory", str(directory / tenant)],
                       check=True, capture_output=True, timeout=75)
    rewrite_signed_tenant(directory / "tenant-a", "tenant-a", "agent-a", "ocaf_ci_tenant_a")
    rewrite_signed_tenant(directory / "tenant-b", "tenant-b", "agent-b", "ocaf_ci_tenant_b")
    ca_a = (directory / "tenant-a" / "ca.pem").read_bytes()
    ca_b = (directory / "tenant-b" / "ca.pem").read_bytes()
    if ca_a == ca_b:
        raise ValueError("different tenants unexpectedly share certificate authority")
    print(json.dumps({"status": "TWO_EPHEMERAL_SIGNED_TENANTS_READY",
                      "independent_ca": True, "production_approved": False}))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    try:
        create_fixtures(args.directory)
    except (ValueError, OSError, subprocess.SubprocessError) as exc:
        print(f"NO_GO: {type(exc).__name__}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
