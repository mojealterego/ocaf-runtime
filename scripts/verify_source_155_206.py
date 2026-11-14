#!/usr/bin/env python3
"""Fail-closed, read-only crosswalk for external OCAF reference points.

The SOURCE-1.55..SOURCE-2.06 identifiers are NOT the canonical OCAF roadmap
stage IDs, despite sharing numeric spellings. No release/CI status is inferred.

Usage: python3 scripts/verify_source_155_206.py --repo .
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any

EXPECTED_POINTS = (
    '1.55', '1.56', '1.57', '1.58', '1.59', '1.60', '1.65',
    '1.70', '1.75', '1.80', '1.85', '1.90', '1.95',
    '2.00', '2.01', '2.02', '2.03', '2.04', '2.05', '2.06',
)
ALLOWED_SOURCE_STATUSES = frozenset({'EXISTING_PARTIAL', 'OPEN', 'RESEARCH_ONLY'})
ALLOWED_ITEM_KEYS = frozenset({
    'source_id', 'source_title', 'canonical_id', 'canonical_title',
    'status', 'implementation_paths', 'remaining_gaps',
})
MAX_MANIFEST_SIZE = 128 * 1024
MAX_INVENTORY_SIZE = 2 * 1024 * 1024
SOURCE_ID = re.compile(r'SOURCE-(?:1\.(?:55|56|57|58|59|60|65|70|75|80|85|90|95)|2\.0[0-6])\Z')
PATH_PART = re.compile(r'[A-Za-z0-9_.-]+\Z')


def _unique_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON field')
        result[key] = value
    return result


def _read_json(path: Path, maximum: int) -> Any:
    if path.is_symlink() or not path.is_file() or not 0 < path.stat().st_size <= maximum:
        raise ValueError('missing, symlinked or oversized file')
    return json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=_unique_keys)


def _valid_repo_path(repo: Path, path: str) -> bool:
    if not isinstance(path, str) or not path or len(path) > 512 or '\\' in path:
        return False
    parts = path.split('/')
    if any(not PATH_PART.fullmatch(x) or x in ('.', '..') for x in parts):
        return False
    cursor = repo
    for part in parts:
        cursor = cursor / part
        if cursor.is_symlink():
            return False
    try:
        return cursor.is_file() and cursor.resolve().is_relative_to(repo.resolve())
    except (OSError, RuntimeError, ValueError):
        return False


def evaluate(repo: Path) -> dict[str, Any]:
    """Read-only structural reconciliation; does not compile/test/approve code."""
    problems: list[str] = []
    report: dict[str, Any] = {
        'status': 'INVALID_SOURCE_MAPPING',
        'production_approved': False,
        'source_points': 0,
        'problems': problems,
        'verification_scope': 'SOURCE_CROSSWALK_ONLY_NO_BUILD_NO_LIVE_E2E',
    }
    root = Path(repo)
    try:
        manifest = _read_json(
            root / 'docs/roadmap/ocaf_source_155_206_manifest.json', MAX_MANIFEST_SIZE
        )
    except (OSError, UnicodeError, ValueError, TypeError, json.JSONDecodeError):
        problems.append('source_manifest_invalid')
        return report
    if (
        not isinstance(manifest, dict)
        or set(manifest) != {
            'schema_version', 'source_kind', 'source_name', 'canonical_inventory',
            'production_approved', 'items'
        }
        or type(manifest['schema_version']) is not int
        or manifest['schema_version'] != 1
        or manifest['source_kind'] != 'EXTERNAL_ARCHITECTURE_RECONCILIATION'
        or not isinstance(manifest['source_name'], str)
        or not 0 < len(manifest['source_name']) <= 200
        or manifest['canonical_inventory'] != 'docs/roadmap/canonical_v11_inventory.json'
        or manifest['production_approved'] is not False
        or not isinstance(manifest['items'], list)
        or len(manifest['items']) != len(EXPECTED_POINTS)
    ):
        problems.append('source_manifest_invalid')
        return report

    try:
        inventory = _read_json(root / manifest['canonical_inventory'], MAX_INVENTORY_SIZE)
    except (OSError, UnicodeError, ValueError, TypeError, json.JSONDecodeError):
        problems.append('canonical_inventory_invalid')
        return report
    if (
        not isinstance(inventory, dict)
        or type(inventory.get('main_count')) is not int
        or inventory['main_count'] != 334
        or type(inventory.get('technical_count')) is not int
        or inventory['technical_count'] != 64
        or inventory.get('production_approved') is not False
        or not isinstance(inventory.get('main'), list)
    ):
        problems.append('canonical_inventory_invalid')
        return report

    canonical: dict[str, str] = {}
    for stage in inventory['main']:
        if not isinstance(stage, dict):
            problems.append('canonical_inventory_invalid')
            return report
        stage_id = stage.get('id')
        stage_title = stage.get('title')
        if isinstance(stage_id, str) and isinstance(stage_title, str):
            if stage_id in canonical:
                problems.append('canonical_inventory_invalid')
                return report
            canonical[stage_id] = stage_title

    seen: set[str] = set()
    for item in manifest['items']:
        if not isinstance(item, dict):
            problems.append('invalid_source_item')
            continue
        source_id = item.get('source_id')
        if not isinstance(source_id, str) or not SOURCE_ID.fullmatch(source_id):
            problems.append('invalid_source_id')
            continue
        if source_id in seen:
            problems.append('duplicate_source_id:' + source_id)
            continue
        seen.add(source_id)
        if set(item) != ALLOWED_ITEM_KEYS:
            problems.append('invalid_item_schema:' + source_id)
            continue
        canonical_id = source_id.removeprefix('SOURCE-')
        if (
            item['canonical_id'] != canonical_id
            or canonical.get(canonical_id) is None
            or item['canonical_title'] != canonical[canonical_id]
        ):
            problems.append('canonical_mismatch:' + source_id)
        if not isinstance(item['source_title'], str) or not item['source_title'].strip():
            problems.append('invalid_source_title:' + source_id)
        elif item['source_title'] == item['canonical_title']:
            problems.append('source_collides_with_canonical:' + source_id)
        if not isinstance(item['status'], str) or item['status'] not in ALLOWED_SOURCE_STATUSES:
            problems.append('invalid_source_status:' + source_id)
        paths = item['implementation_paths']
        if not isinstance(paths, list) or len(paths) > 16 or any(not isinstance(p, str) for p in paths):
            problems.append('invalid_implementation_paths:' + source_id)
        else:
            for p in paths:
                if not _valid_repo_path(root, p):
                    safe = (isinstance(p, str) and all(x not in ('.', '..', '')
                            for x in p.split('/')) and not p.startswith('/') and '\\' not in p)
                    if safe:
                        cursor = root
                        for part in p.split('/'):
                            cursor /= part
                            if cursor.is_symlink():
                                safe = False
                                break
                    problems.append(
                        ('implementation_missing:' if safe else 'unsafe_implementation_path:') + source_id
                    )
                    break
        gaps = item['remaining_gaps']
        if (
            not isinstance(gaps, list)
            or not gaps
            or len(gaps) > 32
            or any(not isinstance(g, str) or not 0 < len(g) <= 500 for g in gaps)
        ):
            problems.append('missing_limitations:' + source_id)
    if seen != {f'SOURCE-{point}' for point in EXPECTED_POINTS}:
        problems.append('source_identifier_set_mismatch')
    report['source_points'] = len(seen)
    if not problems:
        report['status'] = 'SOURCE_MAPPING_VALIDATED_ONLY'
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path('.'))
    args = parser.parse_args(argv)
    result = evaluate(args.repo)
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0 if result['status'] == 'SOURCE_MAPPING_VALIDATED_ONLY' else 1


if __name__ == '__main__':
    sys.exit(main())
