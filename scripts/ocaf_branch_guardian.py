#!/usr/bin/env python3
"""Read-only, fail-closed local Git ref preservation check (SOURCE-2.06).

A trusted *independent* operator must pin SHA-256 of a previously observed
baseline JSON; computing a hash from the same untrusted channel does NOT
provide historical provenance. This does not query GitHub branch protection,
remote refs, CI runs, production deployments or workflow permissions.

Usage: python3 scripts/ocaf_branch_guardian.py --repo . \
    --baseline /path/to/operator-baseline.json --trusted-sha256 64hex
"""
from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import Any

MAX_BASELINE_BYTES = 128 * 1024
MAX_HISTORICAL_REFS = 1024
HEX40 = re.compile(r'[0-9a-f]{40}\Z')
HEX64 = re.compile(r'[0-9a-f]{64}\Z')
EXPECTED_REPOSITORY = 'mojealterego/ocaf-runtime'
REF = re.compile(r'refs/heads/[A-Za-z0-9][A-Za-z0-9._/-]{0,127}\Z')


def _unique_fields(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    d: dict[str, Any] = {}
    for key, value in pairs:
        if key in d:
            raise ValueError('duplicate JSON key')
        d[key] = value
    return d


def _valid_ref(ref: str) -> bool:
    if not REF.fullmatch(ref) or ref == 'refs/heads/main':
        return False
    suffix = ref[len('refs/heads/'):]
    if any(part in ('', '.', '..') or part.endswith('.') or part.startswith('.')
           for part in suffix.split('/')):
        return False
    return '..' not in suffix and '@{' not in suffix and not suffix.endswith('.lock')


def _git(repo: Path, *args: str) -> tuple[int, str]:
    # GIT_DIR, GIT_WORK_TREE and GIT_CONFIG_* from an inherited environment must
    # not redirect checks to a different checkout or alter Git's behavior.
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
    env['GIT_NO_REPLACE_OBJECTS'] = '1'
    try:
        result = subprocess.run(
            ['git', '-C', str(repo), *args], capture_output=True,
            text=True, encoding='utf-8', errors='replace', timeout=5, env=env,
            check=False,
        )
        return result.returncode, result.stdout.strip()
    except (OSError, subprocess.TimeoutExpired):
        return 127, ''


def evaluate(repo: Path, baseline: Path, trusted_sha256: str) -> dict[str, Any]:
    """Observe local refs only; never create, update, reset or delete a ref."""
    blockers: list[str] = []
    report: dict[str, Any] = {
        'schema_version': 1,
        'status': 'NO_GO',
        'production_approved': False,
        'remote_refs_verified': False,
        'workflow_safety_verified': False,
        'historical_refs_checked': 0,
        'observed_main_sha': None,
        'blockers': blockers,
        'limitations': 'Local Git refs only; no remote, branch protection, CI or operator attestation.',
    }
    if not isinstance(trusted_sha256, str) or not HEX64.fullmatch(trusted_sha256):
        blockers.append('invalid_trusted_pin')
        return report
    baseline = Path(baseline)
    try:
        if (baseline.is_symlink() or not baseline.is_file()
                or not 0 < baseline.stat().st_size <= MAX_BASELINE_BYTES):
            raise ValueError('untrusted baseline path')
        raw = baseline.read_bytes()
        if len(raw) > MAX_BASELINE_BYTES:
            raise ValueError('oversized baseline')
    except (OSError, ValueError):
        blockers.append('baseline_invalid')
        return report
    if not hmac.compare_digest(hashlib.sha256(raw).hexdigest(), trusted_sha256):
        blockers.append('baseline_pin_mismatch')
        return report
    try:
        data = json.loads(raw, object_pairs_hook=_unique_fields)
        if not isinstance(data, dict) or set(data) != {
            'schema_version', 'repository', 'baseline_main', 'historical_refs'
        }:
            raise ValueError('unexpected baseline schema')
        if type(data['schema_version']) is not int or data['schema_version'] != 1:
            raise ValueError('wrong schema version')
        if (not isinstance(data['repository'], str)
                or data['repository'] != EXPECTED_REPOSITORY):
            raise ValueError('invalid repository identity')
        if (not isinstance(data['baseline_main'], str)
                or not HEX40.fullmatch(data['baseline_main'])):
            raise ValueError('invalid baseline commit')
        refs = data['historical_refs']
        if (not isinstance(refs, dict) or len(refs) > MAX_HISTORICAL_REFS
                or any(not isinstance(name, str) or not _valid_ref(name)
                       or not isinstance(sha, str) or not HEX40.fullmatch(sha)
                       for name, sha in refs.items())):
            raise ValueError('invalid historical ref set')
    except (UnicodeError, ValueError, TypeError, json.JSONDecodeError):
        blockers.append('baseline_schema_invalid')
        return report

    root = Path(repo)
    rc, located = _git(root, 'rev-parse', '--show-toplevel')
    try:
        same_root = Path(located).resolve() == root.resolve() if rc == 0 else False
    except (OSError, ValueError, RuntimeError):
        same_root = False
    if not same_root:
        blockers.append('not_a_git_checkout')
        return report
    rc, active = _git(root, 'symbolic-ref', '--quiet', 'HEAD')
    if rc != 0 or active != 'refs/heads/main':
        blockers.append('not_on_main')
    rc, current = _git(root, 'rev-parse', '--verify', 'HEAD^{commit}')
    if rc != 0 or not HEX40.fullmatch(current):
        blockers.append('main_head_invalid')
        return report
    report['observed_main_sha'] = current

    baseline_main = data['baseline_main']
    rc, objtype = _git(root, 'cat-file', '-t', baseline_main)
    if rc != 0 or objtype != 'commit':
        blockers.append('baseline_main_unavailable')
        return report
    rc, _ = _git(root, 'merge-base', '--is-ancestor', baseline_main, current)
    if rc != 0:
        blockers.append('main_not_descendant_of_baseline')

    for ref_name, expected in sorted(refs.items()):
        rc, actual = _git(root, 'show-ref', '--verify', '--hash', ref_name)
        if rc != 0 or not actual:
            blockers.append('missing_historical_ref:' + ref_name)
            continue
        if not HEX40.fullmatch(actual) or actual != expected:
            blockers.append('historical_ref_changed:' + ref_name)
            continue
        rc, objtype = _git(root, 'cat-file', '-t', expected)
        if rc != 0 or objtype != 'commit':
            blockers.append('historical_ref_not_commit:' + ref_name)
            continue
        report['historical_refs_checked'] += 1
    if not blockers:
        report['status'] = 'LOCAL_REFS_PRESERVED_UNATTESTED'
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path('.'))
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--trusted-sha256', required=True)
    args = parser.parse_args(argv)
    result = evaluate(args.repo, args.baseline, args.trusted_sha256)
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0 if result['status'] == 'LOCAL_REFS_PRESERVED_UNATTESTED' else 1


if __name__ == '__main__':
    sys.exit(main())
