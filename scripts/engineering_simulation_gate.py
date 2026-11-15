#!/usr/bin/env python3
"""Run bounded, pinned engineering tests without outsourcing stage acceptance.

This evidence gate proves execution of its *registered local test suites*, never
runtime hardware, external infrastructure, full stage closure, or a deployment.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


class GateError(ValueError):
    """Refuse unpinned, unsafe or unverifiable simulation evidence."""


# Strict allowlist: extend only with reviewed, executable engineering tests.
SUITES = {
    'SOURCE-2.05': {
        'scope': 'bounded_python_microbenchmark_harness',
        'paths': ('scripts/ocaf_benchmark_harness.py',),
        'tests': ('test_ocaf_benchmark_harness.py',),
    },
    'SOURCE-2.06': {
        'scope': 'local_git_reference_preservation_guardian',
        'paths': ('scripts/ocaf_branch_guardian.py',),
        'tests': ('test_ocaf_branch_guardian.py', 'test_ocaf_branch_guardian_grafts.py'),
    },
}
OID = re.compile(r'[0-9a-f]{40}\Z')
RUN_COUNT = re.compile(r'^Ran ([1-9][0-9]*) tests? in ', re.MULTILINE)
MAX_FILE_BYTES = 512 * 1024
MAX_CAPTURE_BYTES = 256 * 1024


def _git(repo: Path, *args: str) -> str:
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
    env['GIT_NO_REPLACE_OBJECTS'] = '1'
    try:
        proc = subprocess.run(
            ['git', '-C', str(repo), *args], check=False, capture_output=True,
            timeout=10, text=True, env=env,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise GateError('git unavailable') from exc
    if proc.returncode or len(proc.stdout) > 4096 or len(proc.stderr) > 4096:
        raise GateError('git verification failed')
    return proc.stdout.strip()


def _trusted_file(repo: Path, relative: str) -> bytes:
    file = repo / relative
    try:
        if file.is_symlink() or not file.is_file() or not file.resolve().is_relative_to(repo):
            raise GateError(f'missing or unsafe source: {relative}')
        if not 0 < file.stat().st_size <= MAX_FILE_BYTES:
            raise GateError(f'empty or oversized source: {relative}')
        return file.read_bytes()
    except OSError as exc:
        raise GateError(f'unreadable source: {relative}') from exc


def evaluate(repo: Path, expected_sha: str, requirement: str,
             timeout_seconds: int = 60) -> dict:
    """Execute actual allowlisted suites, pin git checkout and hash evidence."""
    if requirement not in SUITES:
        raise GateError('requirement has no registered engineering test suite')
    if not isinstance(expected_sha, str) or not OID.fullmatch(expected_sha):
        raise GateError('expected git sha must be 40 lowercase hex chars')
    if type(timeout_seconds) is not int or not 1 <= timeout_seconds <= 120:
        raise GateError('invalid bounded test timeout')
    root = repo.resolve(strict=True)
    if not root.is_dir() or _git(root, 'rev-parse', '--show-toplevel') != str(root):
        raise GateError('not a complete, canonical git checkout')
    if (_git(root, 'symbolic-ref', '--quiet', 'HEAD') != 'refs/heads/main'
            or _git(root, 'rev-parse', 'HEAD') != expected_sha):
        raise GateError('main or expected git sha mismatch')
    if _git(root, 'status', '--porcelain', '--untracked-files=all'):
        raise GateError('uncommitted or untracked source changes')
    spec = SUITES[requirement]
    # Hash the complete set of source files before running any test code.
    fingerprints = {}
    for relative in spec['paths'] + tuple('scripts/tests/' + t for t in spec['tests']):
        fingerprints[relative] = hashlib.sha256(_trusted_file(root, relative)).hexdigest()

    results = []
    total = 0
    env = {k: v for k, v in os.environ.items()
           if k not in ('PYTHONPATH', 'PYTHONHOME') and not k.startswith('GIT_')}
    env['PYTHONNOUSERSITE'] = '1'
    env['PYTHONDONTWRITEBYTECODE'] = '1'
    env['GIT_NO_REPLACE_OBJECTS'] = '1'
    env['PYTHONPATH'] = str(root)
    for suite in spec['tests']:
        command = [sys.executable, '-B', '-m', 'unittest', 'discover',
                   '-s', 'scripts/tests', '-p', suite, '-v']
        try:
            process = subprocess.run(command, cwd=root, env=env,
                                     capture_output=True, timeout=timeout_seconds)
        except (subprocess.TimeoutExpired, OSError) as exc:
            raise GateError(f'test runner unavailable or timed out: {suite}') from exc
        combined = process.stdout + b'\n' + process.stderr
        if len(combined) > MAX_CAPTURE_BYTES:
            raise GateError(f'oversized test transcript: {suite}')
        transcript = combined.decode('utf-8', errors='replace')
        matched = RUN_COUNT.findall(transcript)
        if (process.returncode != 0 or len(matched) != 1
                or not re.search(r'\nOK(?:\s|\Z)', transcript)):
            raise GateError(f'test suite did not pass: {suite}')
        count = int(matched[0])
        total += count
        results.append({'suite': suite, 'tests': count,
                        'transcript_sha256': hashlib.sha256(combined).hexdigest()})
    if (total == 0 or _git(root, 'rev-parse', 'HEAD') != expected_sha
            or _git(root, 'status', '--porcelain', '--untracked-files=all')):
        raise GateError('checkout changed or no tests executed')
    for relative, digest in fingerprints.items():
        if hashlib.sha256(_trusted_file(root, relative)).hexdigest() != digest:
            raise GateError('source changed during tests')
    return {
        'schema_version': 1,
        'repository': 'mojealterego/ocaf-runtime',
        'branch': 'main',
        'commit_sha': expected_sha,
        'requirement_id': requirement,
        'status': 'SCOPED_ENGINEERING_SIMULATION_PASS',
        'scope': spec['scope'],
        'suite_results': results,
        'test_count': total,
        'source_sha256': fingerprints,
        'external_audit_required': False,
        'operator_policy_approval': 'APPROVED_2026-11-15',
        'specific_release_approval': False,
        'full_stage_accepted': False,
        'real_external_host_verified': False,
        'production_approved': False,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--head-sha', required=True)
    parser.add_argument('--requirement', required=True, choices=tuple(SUITES))
    parser.add_argument('--timeout-seconds', type=int, default=60)
    args = parser.parse_args(argv)
    try:
        result = evaluate(args.repo, args.head_sha, args.requirement,
                          args.timeout_seconds)
    except (GateError, OSError) as exc:
        result = {'status': 'ENGINEERING_GATE_NO_GO', 'reason': str(exc),
                  'production_approved': False, 'full_stage_accepted': False}
        print(json.dumps(result, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
