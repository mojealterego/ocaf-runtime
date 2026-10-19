#!/usr/bin/env python3
"""Read-only Make execution gate for the 33 OCAF groups.

A Make success is NEVER treated as code delivered. This command does not call
Make's /run endpoint, does not consume AI-agent credits, and requires no writes.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from collections.abc import Mapping
from typing import Any

# Fixed group-to-scenario mapping observed in the OCAF Make team on 2026-10-19.
SCENARIOS: dict[int, int] = {
    1: 8037161, 2: 8037164, 3: 8037165, 4: 8037173, 5: 8037166,
    6: 8037655, 7: 8037656, 8: 8037658, 9: 8037659, 10: 8037661,
    11: 8037662, 12: 8039684, 13: 8039689, 14: 8039691, 15: 8039692,
    16: 8039693, 17: 8039695, 18: 8039696, 19: 8039697, 20: 8039703,
    21: 8039704, 22: 8039705, 23: 8039706, 24: 8039708, 25: 8039710,
    26: 8039712, 27: 8039715, 28: 8039716, 29: 8039717, 30: 8039718,
    31: 8039727, 32: 8039719, 33: 8039720,
}

_SHA = re.compile(r'^[0-9a-f]{40}$')
_CI_ID = re.compile(r'^[1-9][0-9]*$')


class GateError(RuntimeError):
    """Fail-closed API or evidence validation error."""


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise GateError('Redirect refused (will not forward Make API credentials)')


def request_json(url: str, token: str, *, github: bool = False) -> dict[str, Any]:
    """Bounded HTTPS GET; refuses redirects and never prints credentials."""
    if not token or '\n' in token or '\r' in token:
        raise GateError('Missing or invalid API token')
    prefix = 'https://api.github.com/' if github else 'https://eu1.make.com/api/v2/'
    if not url.startswith(prefix):
        raise GateError('Unapproved API host')
    req = urllib.request.Request(
        url,
        headers={
            'Accept': 'application/vnd.github+json' if github else 'application/json',
            'Authorization': ('Bearer ' if github else 'Token ') + token,
            'User-Agent': 'OCAF-Evidence-Gate/1.0',
        },
        method='GET',
    )
    try:
        with urllib.request.build_opener(_NoRedirect()).open(req, timeout=12) as response:
            if response.status != 200:
                raise GateError('HTTP response was not 200')
            blob = response.read(1_000_001)
            if len(blob) > 1_000_000:
                raise GateError('API response exceeds 1 MB limit')
            result = json.loads(blob)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, OSError) as exc:
        raise GateError('Cannot retrieve valid API response') from exc
    if not isinstance(result, dict):
        raise GateError('Unexpected API response type')
    return result


def extract_logs(payload: Mapping[str, Any]) -> list[dict[str, Any]]:
    """Parse Make scenario log collection, rejecting missing/invalid results."""
    entries = payload.get('scenarioLogs', payload.get('logs'))
    if not isinstance(entries, list) or not all(isinstance(x, dict) for x in entries):
        raise GateError('Make execution logs missing or malformed')
    return entries


def _log_key(entry: dict[str, Any]) -> str:
    # Make's log IDs often begin with a millisecond timestamp.
    return str(entry.get('startedAt') or entry.get('timestamp') or entry.get('imtId') or '')


def classify(logs: list[dict[str, Any]]) -> str:
    """A successful Make execution is only an execution, not a delivery."""
    if not logs:
        return 'NEVER_EXECUTED'
    newest = max(enumerate(logs), key=lambda p: (_log_key(p[1]), -p[0]))[1]
    status = newest.get('status')
    if status in (1, '1', 'success'):
        return 'EXECUTED_NOT_DELIVERED'
    if status in (3, '3', 'error', 'failed', 'failure'):
        return 'FAILED'
    if status in (2, '2', 'warning'):
        return 'WARNING'
    if status in ('pending', 'running'):
        return 'PENDING'
    return 'UNKNOWN'


def is_delivered(claim: Mapping[str, Any], verifier=None) -> bool:
    """Only count evidence after an independent GitHub verifier succeeds.

    Caller-provided output fields are untrusted. No verifier -> no delivery.
    """
    if not callable(verifier):
        return False
    sha, ci_id = claim.get('implementation_sha'), str(claim.get('ci_run_id', ''))
    if (claim.get('completion_state') != 'VERIFIED'
        or not isinstance(sha, str) or not _SHA.fullmatch(sha)
        or not _CI_ID.fullmatch(ci_id)
        or claim.get('ci_conclusion') != 'success'):
        return False
    return verifier(sha, ci_id) is True


def verify_github_evidence(sha: str, ci_id: str, *, token: str, repo: str) -> bool:
    """Require actual GitHub commit and successful Actions run at that same SHA."""
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repo):
        raise GateError('Invalid GitHub repository')
    if not _SHA.fullmatch(sha) or not _CI_ID.fullmatch(ci_id):
        return False
    commit = request_json(f'https://api.github.com/repos/{repo}/commits/{sha}', token, github=True)
    run = request_json(f'https://api.github.com/repos/{repo}/actions/runs/{ci_id}', token, github=True)
    return (commit.get('sha') == sha and run.get('head_sha') == sha
            and run.get('conclusion') == 'success'
            and run.get('status') == 'completed')


def audit(zone: str, *, token: str, groups: Mapping[int, int] | None = None) -> dict[str, Any]:
    """Never launches Make work. The zone must be the account's verified EU1."""
    if zone != 'eu1.make.com':
        raise GateError('Unsupported Make zone')
    groups = SCENARIOS if groups is None else groups
    report: dict[str, Any] = {'groups': {}, 'delivered': 0, 'make_calls': 0}
    for number, scenario_id in groups.items():
        try:
            payload = request_json(
                f'https://{zone}/api/v2/scenarios/{scenario_id}/logs', token
            )
            logs = extract_logs(payload)
            state = classify(logs)
            last = max(logs, key=_log_key) if logs else None
            report['groups'][f'G{number}'] = {
                'scenario_id': scenario_id,
                'state': state,
                'execution_id': str(last.get('imtId') or last.get('executionId') or '') if last else None,
            }
        except GateError:
            report['groups'][f'G{number}'] = {
                'scenario_id': scenario_id,
                'state': 'API_ERROR',
                'execution_id': None,
            }
        report['make_calls'] += 1
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--zone', default='eu1.make.com', choices=['eu1.make.com'])
    parser.add_argument('--json-output', help='Optional path to write bounded JSON report')
    args = parser.parse_args(argv)
    token = os.environ.get('OCAF_MAKE_API_TOKEN', '')
    if not token:
        print('OCAF_MAKE_API_TOKEN missing; no scenario started and no API called.', file=sys.stderr)
        return 2
    result = audit(args.zone, token=token)
    output = json.dumps(result, indent=2, sort_keys=True)
    print(output)
    if args.json_output:
        with open(args.json_output, 'w', encoding='utf-8') as fh:
            fh.write(output + '\n')
    # Treat stale, failed, pending, and unverified as unready: intentionally
    # never returns code-delivery success without external SHA + CI proof.
    return 0 if result['groups'] and result['delivered'] == len(result['groups']) else 1


if __name__ == '__main__':
    sys.exit(main())
