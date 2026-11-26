#!/usr/bin/env python3
"""OCAF structural 111-agent package gate, independent of Python -O.

This gate proves only structural input consistency; it cannot certify runtime,
OSTc, cryptographic provenance, platform builds or production readiness.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import stat
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FLEET_REL = Path("OCAF Agents MOJEALTEREGO") / "AGENT-FLEET-111"
REGISTRY_REL = Path("docs/agents/agent_registry_111.json")
MAX_REGISTRY_BYTES = 512 * 1024
MAX_SOURCE_BYTES = 1024 * 1024
PACKAGE = re.compile(r"OCAF-AGENT-[0-9]{3}-[a-z0-9][a-z0-9-]{0,120}\Z")
REQUIRED = (
    "Cargo.toml", "src/main.rs", "src/lib.rs", "config/agent.toml",
    "config/autonomy.toml", "config/tool_policy.toml", "docs/README.md",
    "docs/ARCHITECTURE.md", "docs/USER_MANUAL.md", "tests/security.rs",
    "packaging/install.sh", "packaging/uninstall.sh",
    "packaging/INSTALLER_CONTRACT.md", "packaging/BUILD_EVIDENCE.md",
    "certification/CERTIFICATE_RECORD.md",
)


class FleetValidationError(ValueError):
    """A source record or file cannot prove structural compliance."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise FleetValidationError(message)


def unique_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def safe_path(root: Path, relative: Path, *, directory: bool = False) -> Path:
    require(not relative.is_absolute() and '..' not in relative.parts,
            "unsafe relative path")
    current = root
    for part in relative.parts:
        require(part not in ('', '.', '..'), "unsafe path component")
        current = current / part
        require(not current.is_symlink(), f"symlinked path: {relative}")
    mode = current.stat().st_mode
    require(stat.S_ISDIR(mode) if directory else stat.S_ISREG(mode),
            f"missing {'directory' if directory else 'regular file'}: {relative}")
    return current


def safe_text(root: Path, relative: Path, max_size: int = MAX_SOURCE_BYTES) -> str:
    target = safe_path(root, relative)
    require(0 < target.stat().st_size <= max_size, f"unsafe size: {relative}")
    # Bounded read; source packages are not a hostile concurrent writer sandbox.
    with target.open('rb') as handle:
        raw = handle.read(max_size + 1)
    require(0 < len(raw) <= max_size, f"unsafe read: {relative}")
    return raw.decode('utf-8')


def validate(root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve(strict=True)
    require(root.is_dir(), "root directory missing")
    data = json.loads(safe_text(root, REGISTRY_REL, MAX_REGISTRY_BYTES),
                      object_pairs_hook=unique_keys)
    require(isinstance(data, dict), "registry is not an object")
    require(data.get('schema_version') == 1 and data.get('fleet_id') == 'OCAF-FLEET-111',
            "registry identity mismatch")
    require(type(data.get('declared_agent_count')) is int and data['declared_agent_count'] == 111,
            "declared agent count mismatch")
    require(type(data.get('operational_swarm_count')) is int and data['operational_swarm_count'] == 33,
            "swarm count mismatch")
    require(type(data.get('active_runtime_agent_count')) is int and data['active_runtime_agent_count'] == 0,
            "active runtime must be zero until separately certified")
    naming = data.get('naming_contract')
    require(isinstance(naming, dict) and naming.get('display_name_field') == 'display_name'
            and naming.get('technical_id_field') == 'agent_id'
            and naming.get('package_name_field') == 'package_name'
            and naming.get('names_unique') is True, "naming contract mismatch")
    agents = data.get('agents')
    require(isinstance(agents, list) and len(agents) == 111,
            "exactly 111 registered agents required")
    safe_path(root, FLEET_REL, directory=True)
    seen_names: set[str] = set()
    seen_packages: set[str] = set()
    for i, entry in enumerate(agents, 1):
        require(isinstance(entry, dict), f"invalid agent at index {i}")
        agent_id = f'OCAF-AGENT-{i:03d}'
        require(entry.get('agent_id') == agent_id, f"agent ID or order mismatch: {i}")
        display = entry.get('display_name')
        package_name = entry.get('package_name')
        require(isinstance(display, str) and display.strip() and len(display) <= 160
                and not any(ord(ch) < 32 for ch in display) and display not in seen_names,
                f"invalid or duplicate display name: {agent_id}")
        require(isinstance(package_name, str) and re.fullmatch(r'[a-z0-9][a-z0-9-]{0,127}', package_name)
                and package_name not in seen_packages, f"invalid package name: {agent_id}")
        group = entry.get('group_id')
        require(isinstance(group, str) and re.fullmatch(r'G(?:0[1-9]|[12][0-9]|3[0-3])', group),
                f"invalid group: {agent_id}")
        for key, expected in (
            ('lifecycle', 'DRAFT'), ('deployment_status', 'NOT_DEPLOYED'),
            ('artifact_status', 'NOT_BUILT'), ('model_backend', 'UNASSIGNED'),
            ('source_access', 'disabled'), ('active_admission', 'DENY'),
            ('ostc_enabled', False),
        ):
            require(type(entry.get(key)) is type(expected) and entry[key] == expected,
                    f"unverified agent admission status: {agent_id}/{key}")
        package_path = entry.get('package_path')
        require(isinstance(package_path, str) and package_path.startswith('AGENT-FLEET-111/'),
                f"invalid package path: {agent_id}")
        subdir = package_path.removeprefix('AGENT-FLEET-111/')
        require(bool(PACKAGE.fullmatch(subdir)) and subdir.startswith(agent_id + '-')
                and display.lower().replace(' ', '-') in subdir,
                f"unsafe package path: {agent_id}")
        package = FLEET_REL / subdir
        safe_path(root, package, directory=True)
        for filename in REQUIRED:
            safe_path(root, package / filename)
        config = safe_text(root, package / 'config/agent.toml')
        cargo = safe_text(root, package / 'Cargo.toml')
        source = safe_text(root, package / 'src/lib.rs')
        for line in (
            f'agent_id = "{agent_id}"',
            f'display_name = "{display}"',
            'ostc_enabled = false',
            'source_access = "disabled"',
            'active_admission = "DENY"',
        ):
            require(line in config.splitlines(), f"agent config mismatch: {agent_id}/{line}")
        require(f'name = "{package_name}"' in cargo.splitlines(),
                f"Cargo package mismatch: {agent_id}")
        require(f'pub const DISPLAY_NAME: &str = "{display}";' in source.splitlines(),
                f"Rust source display name mismatch: {agent_id}")
        seen_names.add(display)
        seen_packages.add(package_name)
    return {'status': 'PASS_STRUCTURE_ONLY', 'registered_agents': 111,
            'registered_swarms': 33, 'active_runtime_agents': 0,
            'implementation_verified': False, 'production_approved': False}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    args = parser.parse_args(argv)
    try:
        result = validate(args.root)
    except (FleetValidationError, OSError, UnicodeError, json.JSONDecodeError,
            TypeError, ValueError, RecursionError) as exc:
        print(f"agent_fleet_111: FAIL: {exc}", file=sys.stderr)
        return 1
    print(f"agent_fleet_111: {result['status']} ({result['registered_agents']} separate packages, "
          f"{result['registered_swarms']} swarms; ACTIVE denied)")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
