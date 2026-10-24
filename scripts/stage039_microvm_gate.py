#!/usr/bin/env python3
"""OCAF 0.39 fail-closed Firecracker/KVM and Aya/eBPF admission; NO launch."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import stat
import sys

MAX_MANIFEST_BYTES = 64 * 1024
IDENTITY = re.compile(r"[a-z][a-z0-9_-]{0,62}\Z")
HEX_SHA256 = re.compile(r"[a-f0-9]{64}\Z")
REQUIRED_ASSETS = frozenset({"firecracker", "guest_kernel", "guest_rootfs", "aya_cgroup_device"})
WASI_ASSETS = frozenset({"wasmtime_runtime", "guest_openat_probe", "wasi_module"})
ASSET_MAXIMUM_BYTES = {"firecracker": 256*1024*1024, "guest_kernel": 128*1024*1024,
                       "guest_rootfs": 8*1024*1024*1024, "aya_cgroup_device": 16*1024*1024,
                       "wasmtime_runtime": 256*1024*1024,
                       "guest_openat_probe": 16*1024*1024,
                       "wasi_module": 16*1024*1024}
HOST_CHECKS = frozenset({"linux_x86_64", "systemd_pid1", "cgroup_v2_limits_available",
                        "kvm_device_character", "kvm_device_accessible",
                        "btf_core_available", "bpffs_mounted"})


def _unique_object(pairs: list[tuple[str, object]]) -> dict:
    out = {}
    for key, val in pairs:
        if key in out:
            raise ValueError("duplicate JSON member")
        out[key] = val
    return out


def _reject_nonfinite(_: str) -> None:
    raise ValueError("nonfinite number")


def parse_manifest(raw: bytes) -> dict:
    if not 0 < len(raw) <= MAX_MANIFEST_BYTES:
        raise ValueError("manifest too large or empty")
    try:
        obj = json.loads(raw, object_pairs_hook=_unique_object, parse_constant=_reject_nonfinite)
    except (UnicodeError, ValueError, TypeError) as exc:
        raise ValueError("invalid manifest JSON") from exc
    base_fields = {"schema_version", "workload_id", "tenant_id", "agent_id", "isolation", "assets"}
    if (not isinstance(obj, dict) or set(obj) not in (base_fields, base_fields | {"wasi"})
            or type(obj["schema_version"]) is not int or obj["schema_version"] != 1):
        raise ValueError("unsupported manifest schema")
    if "wasi" in obj:
        wasi = obj["wasi"]
        if (not isinstance(wasi, dict) or set(wasi) != {"task_nonce", "module_sha256"}
                or not isinstance(wasi["task_nonce"], str)
                or re.fullmatch(r"[0-9a-f]{32}", wasi["task_nonce"]) is None
                or not isinstance(wasi["module_sha256"], str)
                or HEX_SHA256.fullmatch(wasi["module_sha256"]) is None):
            raise ValueError("invalid tenant WASI workload binding")
    for key in ("workload_id", "tenant_id", "agent_id"):
        if not isinstance(obj[key], str) or IDENTITY.fullmatch(obj[key]) is None:
            raise ValueError("invalid principal identity")
    policy = obj["isolation"]
    if not isinstance(policy, dict) or set(policy) != {
            "hypervisor", "jailer", "network", "guest_egress", "rootfs_readonly",
            "mmds_enabled", "seccomp", "vcpus", "mem_mib"}:
        raise ValueError("incomplete/unknown isolation contract")
    minimum = {"hypervisor": "firecracker", "jailer": True, "network": "none",
               "guest_egress": False, "rootfs_readonly": True,
               "mmds_enabled": False, "seccomp": "default"}
    if any(type(policy[k]) is not type(v) or policy[k] != v for k, v in minimum.items()):
        raise ValueError("isolation relaxation denied")
    for k, low, high in (("vcpus", 1, 4), ("mem_mib", 128, 2048)):
        if type(policy[k]) is not int or not low <= policy[k] <= high:
            raise ValueError("resource cap denied")
    assets = obj["assets"]
    required_assets = REQUIRED_ASSETS | (WASI_ASSETS if "wasi" in obj else frozenset())
    if not isinstance(assets, dict) or set(assets) != required_assets:
        raise ValueError("missing pinned assets")
    seen = set()
    for item in assets.values():
        if not isinstance(item, dict) or set(item) != {"path", "sha256"}:
            raise ValueError("invalid asset schema")
        p, digest = item["path"], item["sha256"]
        if (not isinstance(p, str) or len(p) > 512 or
                re.fullmatch(r"/[a-zA-Z0-9_./-]+", p) is None or p in seen):
            raise ValueError("invalid/duplicate asset path")
        seen.add(p)
        if not isinstance(digest, str) or HEX_SHA256.fullmatch(digest) is None:
            raise ValueError("invalid pinned SHA256")
    if ("wasi" in obj and
        obj["wasi"]["module_sha256"] != assets["wasi_module"]["sha256"]):
        raise ValueError("WASI module digest differs from separately pinned asset")
    return obj


def _artifact_digest(value: str, root: Path, limit: int, executable: bool) -> str:
    p = Path(value)
    if not p.is_absolute() or ".." in p.parts or not root.is_absolute() or root.is_symlink():
        raise ValueError("untrusted artifact root/path")
    if not root.is_dir() or root.stat().st_mode & 0o022:
        raise ValueError("mutable approved root")
    try:
        rel = p.relative_to(root)
    except ValueError as exc:
        raise ValueError("path outside approved root") from exc
    if not rel.parts:
        raise ValueError("artifact is a directory")
    current = root
    for i, part in enumerate(rel.parts):
        current = current / part
        if current.is_symlink():
            raise ValueError("symlink asset path")
        if i < len(rel.parts)-1 and (not current.is_dir() or current.stat().st_mode & 0o022):
            raise ValueError("mutable parent directory")
    try:
        fd = os.open(p, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0))
    except OSError as exc:
        raise ValueError("cannot open pinned asset") from exc
    try:
        st = os.fstat(fd)
        if (not stat.S_ISREG(st.st_mode) or st.st_nlink != 1 or
                not 0 < st.st_size <= limit or st.st_uid not in {0, os.geteuid()}
                or st.st_mode & 0o022 or executable and not st.st_mode & 0o111):
            raise ValueError("artifact file security denied")
        digest = hashlib.sha256()
        while True:
            data = os.read(fd, 1024*1024)
            if not data:
                break
            digest.update(data)
        return digest.hexdigest()
    finally:
        os.close(fd)


def inspect_manifest(raw: bytes, approved_root: Path) -> dict:
    manifest = parse_manifest(raw)
    checks = {}
    for name in sorted(manifest["assets"]):
        asset = manifest["assets"][name]
        digest = _artifact_digest(asset["path"], Path(approved_root),
                                  ASSET_MAXIMUM_BYTES[name],
                                  name in {"firecracker", "wasmtime_runtime", "guest_openat_probe"})
        if asset["sha256"] != digest:
            raise ValueError("pinned asset digest mismatch: " + name)
        checks[name] = digest
    return {"schema_version": 1, "status": "STATIC_ARTIFACTS_VERIFIED_ONLY",
            "observed_at_utc": datetime.now(timezone.utc).isoformat(),
            "workload_id": manifest["workload_id"], "tenant_id": manifest["tenant_id"],
            "agent_id": manifest["agent_id"], "verified_assets": len(checks),
            "artifact_sha256": checks, "microvm_launched": False,
            "ebpf_attached": False, "kvm_execution_tested": False,
            "stage_039_complete": False, "production_approved": False,
            "limitations": "No Firecracker launch, secure jailer, eBPF attach/link, runtime sandbox isolation or host attestation was tested."}


def _bpffs_mounted() -> bool:
    try:
        return any(len(x := row.split()) >= 3 and x[1] == "/sys/fs/bpf" and x[2] == "bpf"
                   for row in Path("/proc/mounts").read_text(encoding="ascii").splitlines())
    except (UnicodeError, OSError):
        return False


def host_observations() -> dict[str, bool]:
    def read(path: str) -> str:
        try:
            return Path(path).read_text(encoding="ascii").strip()
        except (UnicodeError, OSError):
            return ""
    try:
        st = Path("/dev/kvm").stat()
        kvm = stat.S_ISCHR(st.st_mode) and os.major(st.st_rdev) == 10 and os.minor(st.st_rdev) == 232
    except OSError:
        kvm = False
    ctrls = set(read("/sys/fs/cgroup/cgroup.controllers").split())
    return {
        "linux_x86_64": sys.platform == "linux" and platform.machine() in ("x86_64", "AMD64"),
        "systemd_pid1": read("/proc/1/comm") == "systemd",
        "cgroup_v2_limits_available": {"cpu", "memory", "pids"}.issubset(ctrls),
        "kvm_device_character": kvm,
        "kvm_device_accessible": kvm and os.access("/dev/kvm", os.R_OK | os.W_OK),
        "btf_core_available": Path("/sys/kernel/btf/vmlinux").is_file(),
        "bpffs_mounted": _bpffs_mounted(),
    }


def preflight() -> dict:
    checks = host_observations()
    if set(checks) != HOST_CHECKS or any(type(v) is not bool for v in checks.values()):
        raise ValueError("invalid host checks")
    ok = all(checks.values())
    return {"schema_version": 1, "status": "HOST_PREREQUISITES_ONLY" if ok else "NO_GO",
            "observed_at_utc": datetime.now(timezone.utc).isoformat(),
            "passed": ok, "checks": checks, "microvm_launched": False,
            "ebpf_attached": False, "kvm_execution_tested": False,
            "stage_039_complete": False, "production_approved": False,
            "limitations": "Preflight cannot prove microVM or eBPF isolation."}


def save_report(path: Path, report: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL |
                 getattr(os, "O_NOFOLLOW", 0), 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as output:
        json.dump(report, output, indent=2, sort_keys=True)
        output.write("\n")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--host-only", action="store_true")
    mode.add_argument("--manifest", type=Path)
    ap.add_argument("--approved-root", type=Path)
    ap.add_argument("--report", required=True, type=Path)
    args = ap.parse_args()
    try:
        if args.host_only:
            report = preflight()
        else:
            if args.approved_root is None:
                raise ValueError("manifest requires --approved-root")
            report = inspect_manifest(args.manifest.read_bytes(), args.approved_root)
        save_report(args.report, report)
    except (OSError, ValueError) as exc:
        ap.error(str(exc))
    print(json.dumps({"status": report["status"], "report": str(args.report),
                      "stage_039_complete": False, "production_approved": False}))
    return int(report["status"] == "NO_GO")


if __name__ == "__main__":
    raise SystemExit(main())
