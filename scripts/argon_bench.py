#!/usr/bin/env python3
"""Operator-invoked, reproducible process-level latency benchmark.

Measures actual process invocations, including startup and I/O overhead.
This is NOT an in-process Rust hot-path or network latency benchmark.
Never runs through a shell and never stores raw potentially secret argv.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path


class BenchmarkError(RuntimeError):
    pass


def nearest_rank(samples: list[float], quantile: float) -> float:
    if not samples or not 0 < quantile <= 1:
        raise BenchmarkError("invalid samples or percentile")
    if any(not math.isfinite(x) or x < 0 for x in samples):
        raise BenchmarkError("invalid measurement")
    ordered = sorted(samples)
    return ordered[max(0, math.ceil(quantile * len(ordered)) - 1)]


def benchmark(argv: list[str], *, warmups: int, samples: int, timeout_s: float) -> list[float]:
    if not argv or not argv[0] or warmups < 0 or warmups > 100 or samples < 1 or samples > 10000:
        raise BenchmarkError("invalid command or sample bounds")
    if not math.isfinite(timeout_s) or not 0 < timeout_s <= 600:
        raise BenchmarkError("invalid per-invocation timeout")
    durations = []
    for index in range(warmups + samples):
        start = time.perf_counter_ns()
        try:
            outcome = subprocess.run(
                argv, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL, shell=False, check=False,
                timeout=timeout_s,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise BenchmarkError("command unavailable or timed out") from error
        elapsed_ms = (time.perf_counter_ns() - start) / 1_000_000
        if outcome.returncode != 0:
            raise BenchmarkError(f"command returned nonzero exit status: {outcome.returncode}")
        if index >= warmups:
            durations.append(elapsed_ms)
    return durations


def verify_receipt(receipt: dict, *, expected_sha: str, max_p99_ms: float) -> bool:
    try:
        values = receipt["samples_ms"]
        if receipt["schema_version"] != 1 or not receipt["clean_tree"]:
            return False
        if not isinstance(expected_sha, str) or len(expected_sha) != 40:
            return False
        if receipt["commit_sha"] != expected_sha or not isinstance(values, list) or not values:
            return False
        if any(isinstance(x, bool) or not isinstance(x, (int, float)) for x in values):
            return False
        if not math.isfinite(max_p99_ms) or max_p99_ms <= 0:
            return False
        for q, key in [(0.50, "p50_ms"), (0.95, "p95_ms"), (0.99, "p99_ms")]:
            expected = nearest_rank(values, q)
            actual = receipt[key]
            if isinstance(actual, bool) or not isinstance(actual, (int, float)):
                return False
            if not math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-9):
                return False
        return receipt["p99_ms"] <= max_p99_ms
    except (KeyError, TypeError, ValueError, BenchmarkError):
        return False


def git_identity() -> tuple[str, bool]:
    def git(*args: str) -> str:
        try:
            result = subprocess.run(
                ["git", *args], capture_output=True, text=True,
                timeout=10, shell=False, check=True,
            )
        except (subprocess.SubprocessError, OSError) as error:
            raise BenchmarkError("repository identity unavailable") from error
        return result.stdout.strip()

    sha = git("rev-parse", "HEAD")
    if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha):
        raise BenchmarkError("invalid Git commit")
    return sha, not bool(git("status", "--porcelain", "--untracked-files=normal"))


def tool_version(command: str) -> str | None:
    executable = shutil.which(command)
    if executable is None:
        return None
    try:
        r = subprocess.run(
            [executable, "--version"], text=True, capture_output=True,
            check=False, shell=False, timeout=5,
        )
        return r.stdout.strip()[:200] if r.returncode == 0 else None
    except (OSError, subprocess.SubprocessError):
        return None


def atomic_receipt_write(path: Path, payload: dict) -> None:
    directory = path.parent.resolve(strict=True)
    if path.is_symlink() or not directory.is_dir():
        raise BenchmarkError("invalid receipt path")
    encoded = (json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="wb", prefix=".ocaf-argon-",
                                         dir=directory, delete=False) as stream:
            temporary = Path(stream.name)
            os.chmod(stream.name, 0o600)
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--samples", type=int, default=50)
    p.add_argument("--warmups", type=int, default=5)
    p.add_argument("--timeout-s", type=float, default=20)
    p.add_argument("--max-p99-ms", type=float, required=True)
    p.add_argument("command", nargs=argparse.REMAINDER)
    a = p.parse_args(argv)
    command = a.command[1:] if a.command[:1] == ["--"] else a.command
    try:
        sha, clean = git_identity()
        if not clean:
            raise BenchmarkError("dirty Git checkout: no reproducible receipt")
        durations = benchmark(
            command, warmups=a.warmups, samples=a.samples, timeout_s=a.timeout_s,
        )
        receipt = {
            "schema_version": 1,
            "benchmark_type": "process_wall_clock_including_startup",
            "commit_sha": sha,
            "clean_tree": clean,
            "captured_at_utc": datetime.now(timezone.utc).isoformat(),
            "command_executable": Path(command[0]).name,
            "command_argv_sha256": hashlib.sha256("\0".join(command).encode()).hexdigest(),
            "platform": platform.system(),
            "architecture": platform.machine(),
            "cpu_count": os.cpu_count(),
            "python_version": sys.version.split()[0],
            "rustc_version": tool_version("rustc"),
            "warmups": a.warmups,
            "samples": a.samples,
            "samples_ms": durations,
            "p50_ms": nearest_rank(durations, 0.50),
            "p95_ms": nearest_rank(durations, 0.95),
            "p99_ms": nearest_rank(durations, 0.99),
            "max_p99_ms": a.max_p99_ms,
        }
        passed = verify_receipt(receipt, expected_sha=sha, max_p99_ms=a.max_p99_ms)
        receipt["meets_threshold"] = passed
        atomic_receipt_write(a.output, receipt)
        print("PASS" if passed else "NO-GO", "process p99:", receipt["p99_ms"], "ms")
        return 0 if passed else 2
    except (BenchmarkError, ValueError, OSError) as error:
        print(f"Benchmark NO-GO: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
