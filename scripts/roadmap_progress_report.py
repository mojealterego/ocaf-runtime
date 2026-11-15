#!/usr/bin/env python3
"""Conservative, read-only progress accounting for the canonical OCAF roadmap.

A recorded partial status is not a proof of deployment, CI success or product
acceptance. Unknown statuses fail closed rather than being silently counted.

Usage: python3 scripts/roadmap_progress_report.py --repo .
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys
from typing import Any


INVENTORY = Path('docs/roadmap/canonical_v11_inventory.json')
MAX_INVENTORY_BYTES = 2 * 1024 * 1024
EXPECTED_REPOSITORY = 'mojealterego/ocaf-runtime'
MAIN_RECORDED_PROGRESS = frozenset({
    'PARTIALLY_VERIFIED',
    'COMPLETED_BY_OPERATOR_TECHNICAL_ACCEPTANCE_OPEN',
    'CLOSED_BY_OPERATOR_SIMULATION_TECHNICAL_OPEN',
    'ACTIVE_TECHNICAL_OPEN',
    'PRODUCTION_ACCEPTED_SIMULATION',
})
MAIN_NONPROGRESS = frozenset({
    'PLANNED', 'PLANNED_OR_UNVERIFIED',
    'HISTORICAL_NEEDS_VERIFICATION', 'UNSPECIFIED_IN_CURRENT_REPO',
})
TECH_RECORDED_PROGRESS = frozenset({
    'PARTIALLY_IMPLEMENTED_ACCEPTANCE_OPEN',
    'IMPLEMENTED_LAB_EVIDENCE_ACCEPTANCE_OPEN',
    'IMPLEMENTED_TEST_HARNESS_ACCEPTANCE_OPEN',
    '**PASS 12 tests**, CI host NO-GO',
    '**PASS 17 tests**',
    '**PASS 10 tests**',
    '**REAL KVM API 12 and CREATE_VM PASS as root, run 38704086975**',
    '**Static chroot config + 6 tests PASS; live jailer / TOCTOU runtime proof OPEN**',
    '**LIVE EXECUTOR WRITTEN; 6 contract tests PASS; host KVM NO-GO**',
    '**REAL kernel Aya attach and A/B negative+recovery PASS, run 38704086975**',
    'RUNNER_IMPLEMENTED_LIVE_TWO_KERNEL_EVIDENCE_PENDING',
    'PARTIAL_QUORUM_CERTIFICATE_HARDENING_THREE_HOST_EVIDENCE_PENDING',
})
TECH_NONPROGRESS = frozenset({
    'PLANNED', 'UNVERIFIED_ACCEPTANCE_OPEN',
    '**NO-GO / hosts unavailable**', '**NO-GO**',
    'HISTORICAL_LAB_SCOPE_LIMITED',
})


class ReportError(ValueError):
    """The manifest cannot be assessed without inventing progress."""


def _unique_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ReportError(f'duplicate JSON key: {key}')
        result[key] = value
    return result


def evaluate(repo: Path) -> dict[str, Any]:
    """Inspect declared stage statuses only; never infer runtime execution."""
    path = Path(repo) / INVENTORY
    try:
        if (path.is_symlink() or not path.is_file()
                or not 0 < path.stat().st_size <= MAX_INVENTORY_BYTES):
            raise ReportError('unsafe inventory path or size')
        raw = path.read_bytes()
        if len(raw) > MAX_INVENTORY_BYTES:
            raise ReportError('unsafe inventory size')
        data = json.loads(raw, object_pairs_hook=_unique_keys)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ReportError('invalid inventory document') from exc
    if not isinstance(data, dict) or data.get('repository') != EXPECTED_REPOSITORY:
        raise ReportError('repository identity mismatch')
    if (type(data.get('schema_version')) is not int
            or data['schema_version'] != 1
            or data.get('version') != 'v11-original-builder-scope-restored'
            or data.get('branch') != 'main'):
        raise ReportError('canonical inventory identity mismatch')
    if data.get('production_approved') is not False:
        raise ReportError('production approval cannot be inferred')
    if (type(data.get('reference_count')) is not int
            or data['reference_count'] != 33):
        raise ReportError('reference count mismatch')

    counted: dict[str, dict[str, Any]] = {}
    seen_ids: set[str] = set()
    for section, progress, other in (
        ('main', MAIN_RECORDED_PROGRESS, MAIN_NONPROGRESS),
        ('technical', TECH_RECORDED_PROGRESS, TECH_NONPROGRESS),
    ):
        entries = data.get(section)
        declared = data.get(f'{section}_count')
        if (type(declared) is not int or not isinstance(entries, list)
                or declared != len(entries) or (section == 'main' and declared < 334)
                or (section == 'technical' and declared != 64)):
            raise ReportError(f'{section} count mismatch')
        state_counts: Counter[str] = Counter()
        progress_ids: list[str] = []
        for item in entries:
            if not isinstance(item, dict):
                raise ReportError(f'invalid stage in {section}')
            stage_id, title, status = (item.get('id'), item.get('title'), item.get('status'))
            if (not isinstance(stage_id, str) or not stage_id.strip()
                    or not isinstance(title, str) or not title.strip()):
                raise ReportError(f'invalid stage in {section}')
            if stage_id in seen_ids:
                raise ReportError(f'duplicate stage id: {stage_id}')
            seen_ids.add(stage_id)
            if not isinstance(status, str) or status not in progress | other:
                raise ReportError(f'unclassified status in {section}: {status!r}')
            state_counts[status] += 1
            if status in progress:
                progress_ids.append(stage_id)
        counted[section] = {
            'count': len(entries),
            'status_counts': dict(sorted(state_counts.items())),
            'progress_ids': progress_ids,
        }

    return {
        'schema_version': 1,
        'repository': EXPECTED_REPOSITORY,
        'inventory_sha256': hashlib.sha256(raw).hexdigest(),
        'total_stages': counted['main']['count'] + counted['technical']['count'],
        'main_stages': counted['main']['count'],
        'technical_stages': counted['technical']['count'],
        'main_with_recorded_progress': len(counted['main']['progress_ids']),
        'technical_with_recorded_progress': len(counted['technical']['progress_ids']),
        'total_with_recorded_progress': (
            len(counted['main']['progress_ids']) + len(counted['technical']['progress_ids'])
        ),
        'main_progress_ids': counted['main']['progress_ids'],
        'technical_progress_ids': counted['technical']['progress_ids'],
        'main_status_counts': counted['main']['status_counts'],
        'technical_status_counts': counted['technical']['status_counts'],
        'production_approved': False,
        'production_verification': 'NOT_EVALUATED',
        'evidence_level': 'MANIFEST_STATUS_ONLY',
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path('.'))
    args = parser.parse_args(argv)
    try:
        report = evaluate(args.repo)
    except ReportError as exc:
        print(json.dumps({'status': 'INVALID_INVENTORY', 'reason': str(exc),
                          'production_approved': False}, sort_keys=True))
        return 1
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
