#!/usr/bin/env python3
"""Deterministic CycloneDX inventory of the exact stage 0.39 KVM inputs.

This is a prelaunch digest and source inventory, not an attestation of the
inodes eventually consumed by Firecracker or a vulnerability clearance.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import stat

import stage039_microvm_gate as gate
import stage039_signed_manifest as signed

SOURCE_LOCKS = (
    "Cargo.lock",
    "stage039/guest-aya/ebpf/Cargo.lock",
    "stage039/guest-aya/loader/Cargo.lock",
)
SOURCE_BUILDS = (
    "scripts/stage039_prepare_guest_wasi.sh",
    "stage039/guest-aya/build.sh",
    "stage039/guest-aya/ebpf/src/main.rs",
    "stage039/guest-aya/loader/src/main.rs",
)
MAX_SOURCE_BYTES = 16 * 1024 * 1024


class SupplyChainDenied(ValueError):
    pass


def stable_digest(path: Path, limit: int, *, root: Path) -> tuple[str, int]:
    """Hash one regular inode through O_NOFOLLOW and reject observed changes."""
    if (not root.is_absolute() or root.is_symlink() or not root.is_dir()
            or not path.is_absolute() or path.is_symlink()
            or ".." in path.parts or not path.is_relative_to(root)):
        raise SupplyChainDenied("untrusted inventory path")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
    try:
        fd = os.open(path, flags)
    except OSError as exc:
        raise SupplyChainDenied("inventory input unavailable") from exc
    try:
        before = os.fstat(fd)
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or before.st_mode & 0o022 or not 0 < before.st_size <= limit):
            raise SupplyChainDenied("mutable or unbounded inventory input")
        digest = hashlib.sha256()
        while chunk := os.read(fd, 1024 * 1024):
            digest.update(chunk)
        after = os.fstat(fd)
        named = path.stat(follow_symlinks=False)
        identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        if identity(before) != identity(after) or identity(after) != identity(named):
            raise SupplyChainDenied("inventory inode changed during read")
        return digest.hexdigest(), after.st_size
    finally:
        os.close(fd)


def _component(name: str, digest: str, size: int, kind: str) -> dict:
    return {
        "type": kind,
        "name": name,
        "bom-ref": "ocaf039:" + name,
        "hashes": [{"alg": "SHA-256", "content": digest}],
        "properties": [{"name": "ocaf039:bytes", "value": str(size)}],
    }


def inventory(approved: Path, repository: Path) -> tuple[dict, dict]:
    if (not approved.is_absolute() or approved.is_symlink() or not approved.is_dir()
            or approved.stat().st_mode & 0o022 or not repository.is_absolute()
            or repository.is_symlink() or not repository.is_dir()):
        raise SupplyChainDenied("private approved assets and repository required")
    key_path = approved / "stage039-ci-public.key"
    key_digest, _ = stable_digest(key_path, 32, root=approved)
    public = key_path.read_bytes()
    if len(public) != 32 or hashlib.sha256(public).hexdigest() != key_digest:
        raise SupplyChainDenied("trusted lab signer changed")
    manifests = {}
    for tenant in ("a", "b"):
        name = f"request-wasi-{tenant}.json"
        path = approved / name
        digest, _size = stable_digest(path, gate.MAX_MANIFEST_BYTES, root=approved)
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != digest:
            raise SupplyChainDenied("manifest changed after stable read")
        parsed = gate.parse_manifest(raw)
        signed_path = approved / f"signed-wasi-{tenant}.json"
        signed_digest, _ = stable_digest(signed_path, 256 * 1024, root=approved)
        signed_raw = signed_path.read_bytes()
        if hashlib.sha256(signed_raw).hexdigest() != signed_digest:
            raise SupplyChainDenied("signed envelope changed after stable read")
        verified = signed.verify(signed_raw, {"stage039-ci": public})
        if verified["payload"] != parsed:
            raise SupplyChainDenied("signed manifest differs from reviewed request")
        if parsed["tenant_id"] != "tenant-" + tenant or "wasi" not in parsed:
            raise SupplyChainDenied("two distinct signed-task manifests required")
        if gate.inspect_manifest(raw, approved)["verified_assets"] != len(parsed["assets"]):
            raise SupplyChainDenied("asset inspection incomplete")
        manifests[tenant] = (parsed, digest, signed_digest)
    if (manifests["a"][0]["wasi"]["task_nonce"] ==
            manifests["b"][0]["wasi"]["task_nonce"] or
            manifests["a"][0]["wasi"]["module_sha256"] ==
            manifests["b"][0]["wasi"]["module_sha256"]):
        raise SupplyChainDenied("cross-tenant WASI task reuse")
    components = []
    evidence = {"manifest_sha256": {}, "signed_envelope_sha256": {},
                "trusted_lab_public_key_sha256": key_digest,
                "asset_sha256": {}, "source_sha256": {}}
    for tenant in ("a", "b"):
        parsed, manifest_digest, signed_digest = manifests[tenant]
        evidence["manifest_sha256"][tenant] = manifest_digest
        evidence["signed_envelope_sha256"][tenant] = signed_digest
        for asset, record in sorted(parsed["assets"].items()):
            path = Path(record["path"])
            digest, size = stable_digest(
                path, gate.ASSET_MAXIMUM_BYTES[asset], root=approved)
            if digest != record["sha256"]:
                raise SupplyChainDenied("signed asset changed: " + asset)
            name = f"tenant-{tenant}/{asset}"
            components.append(_component(name, digest, size, "file"))
            evidence["asset_sha256"][name] = digest
    for name in SOURCE_LOCKS + SOURCE_BUILDS:
        digest, size = stable_digest(repository / name, MAX_SOURCE_BYTES,
                                     root=repository)
        components.append(_component("source/" + name, digest, size, "file"))
        evidence["source_sha256"][name] = digest
    components.sort(key=lambda item: item["bom-ref"])
    bom = {
        "bomFormat": "CycloneDX", "specVersion": "1.5", "version": 1,
        "metadata": {"component": {"type": "application", "name": "ocaf-stage039-kvm-inputs"}},
        "components": components,
    }
    evidence.update({"status": "STAGE039_PRELAUNCH_INPUT_SBOM_VERIFIED",
                     "component_count": len(components),
                     "running_inode_verified": False,
                     "independent_signer_verified": False,
                     "guest_escape_tested": False,
                     "vulnerability_review_complete": False,
                     "independent_scope_review_complete": False,
                     "stage_039_complete": False, "production_approved": False})
    return bom, evidence


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--approved", type=Path, required=True)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--sbom", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if args.sbom.exists() or args.report.exists() or args.sbom == args.report:
        parser.error("fresh separate SBOM and report paths required")
    try:
        bom, evidence = inventory(args.approved, args.repository)
        sbom_raw = (json.dumps(bom, sort_keys=True, separators=(",", ":")) + "\n").encode()
        evidence["sbom_sha256"] = hashlib.sha256(sbom_raw).hexdigest()
        report_raw = (json.dumps(evidence, sort_keys=True, indent=2) + "\n").encode()
        for path, raw in ((args.sbom, sbom_raw), (args.report, report_raw)):
            fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL |
                         getattr(os, "O_NOFOLLOW", 0), 0o600)
            with os.fdopen(fd, "wb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    print(evidence["status"])


if __name__ == "__main__":
    main()
