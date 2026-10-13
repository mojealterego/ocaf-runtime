#!/usr/bin/env python3
"""Linux WASI stress/soak evidence, not an independent security certification.

Runs the real Wasmtime acceptance repeatedly, samples Linux /proc resources,
and rejects unexpected private-workspace residue. All runs are local and
bounded; no credentials, network upload, or mutating system configuration.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import tempfile
import time


def file_sha256(path: Path) -> str:
    sha = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            sha.update(chunk)
    return sha.hexdigest()


def private_workspaces() -> set[str]:
    # Only report directories owned by this execution UID; do not touch them.
    root = Path("/tmp")
    return {
        p.name for p in root.glob("ocaf-wasi-*")
        if p.is_dir() and not p.is_symlink() and p.stat().st_uid == os.geteuid()
    }


def descendants(root: int) -> set[int]:
    parent_by_pid: dict[int, int] = {}
    for p in Path("/proc").iterdir():
        if not p.name.isdigit():
            continue
        try:
            raw = (p / "stat").read_text()
            _, fields = raw.rsplit(") ", 1)
            parent_by_pid[int(p.name)] = int(fields.split()[1])
        except (ValueError, OSError):
            continue
    found = {root}
    pending = [root]
    while pending:
        current = pending.pop()
        for pid, ppid in parent_by_pid.items():
            if ppid == current and pid not in found:
                found.add(pid)
                pending.append(pid)
    return found


def sample_tree(root: int) -> tuple[int, int, int]:
    pids = descendants(root)
    total_rss_kib = 0
    max_fd = 0
    for pid in pids:
        try:
            status = (Path("/proc") / str(pid) / "status").read_text()
            for line in status.splitlines():
                if line.startswith("VmRSS:"):
                    total_rss_kib += int(line.split()[1])
                    break
            max_fd = max(max_fd, len(list((Path("/proc") / str(pid) / "fd").iterdir())))
        except (OSError, ValueError):
            continue
    return total_rss_kib, max_fd, len(pids)


def single_cycle(timeout: float) -> dict[str, object]:
    args = [
        "cargo", "test", "--locked", "--no-default-features", "--test",
        "wasi_sandbox", "--", "--test-threads=1"
    ]
    begin = time.monotonic()
    peak_rss_kib = peak_fds = peak_processes = samples = 0
    timed_out = False
    with tempfile.TemporaryFile() as output:
        proc = subprocess.Popen(
            args, stdout=output, stderr=subprocess.STDOUT,
            start_new_session=True, close_fds=True
        )
        while proc.poll() is None:
            samples += 1
            rss_kib, fds, processes = sample_tree(proc.pid)
            peak_rss_kib = max(peak_rss_kib, rss_kib)
            peak_fds = max(peak_fds, fds)
            peak_processes = max(peak_processes, processes)
            if time.monotonic() - begin > timeout:
                timed_out = True
                try:
                    os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                break
            time.sleep(0.02)
        returncode = proc.wait()
        output.seek(0, os.SEEK_END)
        output_bytes = output.tell()
    return {
        "returncode": returncode,
        "timed_out": timed_out,
        "elapsed_sec": round(time.monotonic() - begin, 3),
        "peak_observed_rss_kib": peak_rss_kib,
        "peak_observed_open_fds_per_process": peak_fds,
        "peak_observed_process_count": peak_processes,
        "samples": samples,
        "captured_log_bytes_not_embedded": output_bytes,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cycles", type=int, default=2)
    parser.add_argument("--timeout", type=float, default=90)
    parser.add_argument("--max-rss-kib", type=int, default=0)
    parser.add_argument("--max-fds", type=int, default=0)
    parser.add_argument("--max-processes", type=int, default=0)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if platform.system() != "Linux" or not Path("/proc/self/status").is_file():
        parser.error("Linux /proc is required")
    if not (1 <= args.cycles <= 10000 and 2 <= args.timeout <= 3600):
        parser.error("cycles must be 1..10000, timeout 2..3600 seconds")
    if min(args.max_rss_kib, args.max_fds, args.max_processes) < 0:
        parser.error("resource ceilings cannot be negative")
    engine_name = os.getenv("OCAF_TEST_WASMTIME")
    if not engine_name:
        parser.error("set OCAF_TEST_WASMTIME to an operator-pinned real Wasmtime")
    engine = Path(engine_name)
    if not engine.is_absolute() or not engine.is_file():
        parser.error("OCAF_TEST_WASMTIME must be an absolute executable file")
    digest = file_sha256(engine)
    expected = os.getenv("OCAF_TEST_WASMTIME_SHA256")
    if expected and expected != digest:
        parser.error("Wasmtime binary SHA-256 does not match operator pin")

    started_at = int(time.time())
    before = private_workspaces()
    results: list[dict[str, object]] = []
    passed = True
    for cycle in range(args.cycles):
        result = single_cycle(args.timeout)
        result["cycle"] = cycle + 1
        for metric, maximum in [
            ("peak_observed_rss_kib", args.max_rss_kib),
            ("peak_observed_open_fds_per_process", args.max_fds),
            ("peak_observed_process_count", args.max_processes),
        ]:
            if maximum and result[metric] > maximum:
                result["ceiling_violation"] = metric
        results.append(result)
        if result["returncode"] != 0 or result["timed_out"] or "ceiling_violation" in result:
            passed = False
            break

    # Tokio reaping and private directory Drop may complete shortly after
    # the test parent exits; allow a bounded grace period, never delete.
    leaked: set[str] = set()
    for _ in range(30):
        leaked = private_workspaces() - before
        if not leaked:
            break
        time.sleep(0.1)
    if leaked:
        passed = False
    rev = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=False
    )
    report = {
        "schema_version": 1,
        "acceptance": "Linux WASIp1 subprocess soak; not a security certification",
        "git_commit": rev.stdout.strip() if rev.returncode == 0 else None,
        "started_unix_secs": started_at,
        "completed_unix_secs": int(time.time()),
        "kernel": platform.release(),
        "architecture": platform.machine(),
        "engine_sha256": digest,
        "limits": {
            "max_rss_kib": args.max_rss_kib,
            "max_fds": args.max_fds,
            "max_processes": args.max_processes,
        },
        "cycles_requested": args.cycles,
        "cycles_completed": len(results),
        "cycles": results,
        "new_private_workspace_residue": sorted(leaked),
        "passed": passed,
        "sampling_note": "20 ms best-effort /proc process-tree samples; summed RSS double-counts shared pages",
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"passed": passed, "cycles": len(results), "report": str(args.report)}))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
