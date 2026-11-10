#!/usr/bin/env python3
"""Fail-closed static audit of native Builder push path filters (no YAML dependency)."""
from pathlib import Path
import argparse
import re
import sys

REQUIRED = {
    'agents-builder-native-desktop.yml': ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'),
    'agents-builder-native-mobile.yml': ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'),
    'agents-builder-native-linux.yml': ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'),
}


def push_paths(source: str) -> set[str]:
    """Read only the literal on.push.paths sequence, rejecting ambiguous YAML."""
    lines = source.splitlines()
    if not lines or not re.search(r'^on:\s*$', source, re.M):
        raise ValueError('missing top-level on mapping')
    # Reject later top-level trigger mappings, even after jobs/name sections.
    if sum(1 for line in lines if line == 'on:') != 1:
        raise ValueError('duplicate or ambiguous top-level on mapping')
    # A second push or paths key can override the audited list in a YAML loader.
    in_on = False
    in_push = False
    seen_push = False
    seen_paths = False
    for line in lines:
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        indent = len(line) - len(line.lstrip(' '))
        if indent == 0:
            in_on = line == 'on:'
            in_push = False
        elif in_on and indent == 2:
            in_push = line.strip() == 'push:'
            if in_push:
                if seen_push:
                    raise ValueError('duplicate push mapping')
                seen_push = True
        elif in_on and in_push and indent == 4 and line.strip() == 'paths:':
            if seen_paths:
                raise ValueError('duplicate push paths mapping')
            seen_paths = True
    state = 'seek_on'
    found = False
    result: set[str] = set()
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith('#'):
            continue
        indent = len(line) - len(line.lstrip(' '))
        if '\t' in line[:len(line)-len(line.lstrip())]:
            raise ValueError('tab indentation')
        if indent == 0:
            if stripped == 'on:':
                if found or state != 'seek_on':
                    raise ValueError('duplicate on mapping')
                state = 'on'
            elif state != 'seek_on':
                break
            continue
        if state == 'on' and indent == 2:
            if stripped == 'push:':
                state = 'push'
            continue
        if state == 'push' and indent == 2:
            break
        if state == 'push' and indent == 4:
            if stripped == 'paths:':
                state = 'paths'
                found = True
            continue
        if state == 'paths' and indent <= 4:
            break
        if state == 'paths':
            if indent != 6 or not stripped.startswith('- '):
                raise ValueError('unsupported paths representation')
            item = stripped[2:].strip()
            if len(item) < 2 or item[0] not in ('"', "'") or item[-1] != item[0]:
                raise ValueError('non-literal path')
            result.add(item[1:-1])
    if not found or not result:
        raise ValueError('missing or empty on.push.paths')
    return result


def audit(root: Path) -> dict[str, list[str]]:
    failures = {}
    for filename, expected in REQUIRED.items():
        path = root / '.github' / 'workflows' / filename
        try:
            actual = push_paths(path.read_text(encoding='utf-8'))
            missing = sorted(set(expected) - actual)
            if missing:
                failures[filename] = missing
        except (OSError, UnicodeError, ValueError) as exc:
            failures[filename] = [f'INVALID: {exc}']
    return failures


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args(argv)
    failures = audit(args.root)
    for filename, missing in failures.items():
        print(f'FAIL {filename}: {", ".join(missing)}')
    if failures:
        return 1
    print('PASS: all three native Builder workflows cover pinned Rust inputs')
    return 0


if __name__ == '__main__':
    sys.exit(main())
