#!/usr/bin/env python3
"""Opt-in, bounded Python microbenchmark with explicit provenance limitations.

Only the built-in SHA-256 bytes workload is exposed via CLI. No command,
plugin, network or arbitrary user script is executed by this entry point.
Results are NOT Rust hot-path metrics, hardware certificates or production GO.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import time
from typing import Callable


class BenchmarkError(ValueError):
    """Malformed benchmark input, invalid clock or missing provenance."""


@dataclass(frozen=True)
class BenchmarkConfig:
    name: str
    profile: str
    warmups: int
    samples: int
    payload_bytes: int


def validate_config(config: BenchmarkConfig) -> None:
    if (
        not re.fullmatch(r'[A-Za-z][A-Za-z0-9_-]{0,63}', config.name)
        or config.profile not in ('warm', 'cold')
        or type(config.samples) is not int or not 1 <= config.samples <= 10_000
        or type(config.warmups) is not int or not 0 <= config.warmups <= 10_000
        or type(config.payload_bytes) is not int or not 1 <= config.payload_bytes <= 1_048_576
        or config.profile == 'cold' and config.warmups != 0
    ):
        raise BenchmarkError('Invalid bounded benchmark configuration')


def nearest_rank(sorted_ns: list[int], percentile: int) -> int:
    if not sorted_ns or type(percentile) is not int or not 1 <= percentile <= 100:
        raise BenchmarkError('Invalid percentile or missing samples')
    return sorted_ns[(len(sorted_ns) * percentile + 99) // 100 - 1]


def measure(
    config: BenchmarkConfig,
    workload: Callable[[int], object],
    *,
    clock: Callable[[], int] = time.perf_counter_ns,
) -> dict:
    """Measure caller-trusted code; the callback is NOT sandboxed.

    An injected test clock may be synthetic; only CLI records actual time.
    Cache condition is operator-declared, not a proven cache flush.
    """
    validate_config(config)
    for n in range(config.warmups):
        workload(n)
    sample_ns: list[int] = []
    for n in range(config.samples):
        start = clock()
        workload(n)
        stop = clock()
        if type(start) is not int or type(stop) is not int or stop < start:
            raise BenchmarkError('Monotonic high resolution clock invalid')
        sample_ns.append(stop - start)
    sample_ns.sort()

    def microseconds(value: float) -> float:
        return round(value / 1000.0, 3)

    return {
        'schema_version': 1,
        'status': 'MEASURED_LOCAL_PYTHON_ONLY',
        'production_approved': False,
        'benchmark_name': config.name,
        'cache_condition': config.profile,
        'cache_condition_unverified': True,
        'warmup_iterations': config.warmups,
        'measured_iterations': config.samples,
        'payload_bytes': config.payload_bytes,
        'mean_us': microseconds(sum(sample_ns) / len(sample_ns)),
        'p50_us': microseconds(nearest_rank(sample_ns, 50)),
        'p95_us': microseconds(nearest_rank(sample_ns, 95)),
        'p99_us': microseconds(nearest_rank(sample_ns, 99)),
        'max_us': microseconds(sample_ns[-1]),
        'limitations': 'not_rust,not_hardware_attestation,not_full_end_to_end,not_cache_flush_proof',
    }


def _git(repo: Path, *args: str) -> str:
    try:
        cmd = subprocess.run(
            ['git', '-C', str(repo), *args], check=False, capture_output=True,
            text=True, timeout=3,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise BenchmarkError('Cannot establish source control provenance') from exc
    if cmd.returncode != 0:
        raise BenchmarkError('Cannot establish source control provenance')
    return cmd.stdout.strip()


def environment(repo: Path) -> dict:
    """System-reported fields; NOT proof of a physical dedicated host."""
    sha = _git(repo, 'rev-parse', '--verify', 'HEAD^{commit}')
    if not re.fullmatch(r'[0-9a-f]{40}', sha):
        raise BenchmarkError('Invalid Git commit SHA')
    git_status = _git(repo, 'status', '--porcelain')
    cpu_model = 'UNAVAILABLE'
    if sys.platform.startswith('linux'):
        try:
            with open('/proc/cpuinfo', encoding='utf-8') as f:
                for line in f:
                    if line.startswith('model name') and ':' in line:
                        cpu_model = line.partition(':')[2].strip()[:256]
                        break
        except OSError:
            pass
    return {
        'git_commit_sha': sha,
        'git_worktree_dirty': bool(git_status),
        'platform': platform.system(),
        'architecture': platform.machine(),
        'os_release': platform.release(),
        'python_version': platform.python_version(),
        'cpu_model_reported': cpu_model,
        'logical_cpu_count_reported': os.cpu_count(),
        'time_utc': datetime.now(timezone.utc).isoformat(),
        'clock_source': 'time.perf_counter_ns',
        'environment_attested': False,
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--repo', type=Path, default=Path('.'))
    ap.add_argument('--samples', type=int, default=100)
    ap.add_argument('--warmups', type=int, default=10)
    ap.add_argument('--payload-bytes', type=int, default=4096)
    ap.add_argument('--profile', choices=('warm', 'cold'), default='warm')
    args = ap.parse_args(argv)
    config = BenchmarkConfig(
        name='sha256_bytes', profile=args.profile, warmups=args.warmups,
        samples=args.samples, payload_bytes=args.payload_bytes,
    )
    try:
        validate_config(config)
        env = environment(args.repo)
        data = bytes((i % 251 for i in range(config.payload_bytes)))
        stats = measure(config, lambda _: hashlib.sha256(data).digest())
    except BenchmarkError as exc:
        print(json.dumps({'status': 'INVALID_BENCHMARK', 'production_approved': False,
                          'error': str(exc)}))
        return 2
    print(json.dumps({'environment': env, 'benchmark': stats}, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
