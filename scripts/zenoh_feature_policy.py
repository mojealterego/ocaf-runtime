#!/usr/bin/env python3
"""Verify resolved Zenoh features rather than trusting Cargo.toml alone.

This does NOT override RustSec advisories or certify a secure runtime.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

NAMES = ("zenoh", "zenoh-transport")
REQUIRED = frozenset(("transport_tcp", "transport_tls"))
FORBIDDEN = frozenset(
    ("auth_pubkey", "auth_usrpwd", "transport_multilink", "transport_compression")
)
MAX_METADATA_BYTES = 16 * 1024 * 1024


class PolicyError(ValueError):
    """Unverified, unsafe or malformed Cargo feature resolution."""


def inspect(metadata: Any) -> dict[str, Any]:
    if not isinstance(metadata, dict):
        raise PolicyError("invalid Cargo metadata document")
    packages = metadata.get("packages")
    resolution = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolution, dict):
        raise PolicyError("Cargo metadata lacks a resolved package graph")
    nodes = resolution.get("nodes")
    if not isinstance(nodes, list):
        raise PolicyError("Cargo metadata lacks resolved feature nodes")

    selected: dict[str, str] = {}
    for package in packages:
        if not isinstance(package, dict):
            raise PolicyError("malformed Cargo package metadata")
        name = package.get("name")
        if name in NAMES:
            package_id = package.get("id")
            if not isinstance(package_id, str) or not package_id or name in selected:
                raise PolicyError("Zenoh dependency identity is missing or ambiguous")
            selected[name] = package_id
    if set(selected) != set(NAMES):
        raise PolicyError("Zenoh and its transport must both be present")

    resolved: dict[str, list[str]] = {}
    for node in nodes:
        if not isinstance(node, dict):
            raise PolicyError("malformed Cargo resolve node")
        node_id = node.get("id")
        if node_id not in selected.values():
            continue
        package_name = next(
            name for name, identity in selected.items() if identity == node_id
        )
        if package_name in resolved:
            raise PolicyError("duplicate Zenoh resolution node")
        features = node.get("features")
        if not isinstance(features, list) or any(
            not isinstance(value, str) for value in features
        ):
            raise PolicyError("invalid Zenoh resolved features")
        active = set(features)
        if not REQUIRED.issubset(active):
            raise PolicyError("required Zenoh TCP/TLS feature missing")
        if active & FORBIDDEN:
            raise PolicyError("unsafe Zenoh compression, auth or multilink feature enabled")
        if any(f.startswith("transport_") and f not in REQUIRED for f in active):
            raise PolicyError("unreviewed Zenoh transport enabled")
        resolved[package_name] = sorted(active)
    if set(resolved) != set(NAMES):
        raise PolicyError("Zenoh transport graph is not resolved")
    return {
        "status": "PASS",
        "packages_checked": len(resolved),
        "allowed_transports": sorted(REQUIRED),
        "scope": "compiled-feature-policy-only-not-vulnerability-clearance",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--locked", "--all-features"],
            cwd=args.repo,
            capture_output=True,
            timeout=120,
            check=False,
        )
        if result.returncode or len(result.stdout) > MAX_METADATA_BYTES:
            raise PolicyError("Cargo dependency graph unavailable or oversized")
        try:
            metadata = json.loads(result.stdout)
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise PolicyError("invalid Cargo metadata JSON") from exc
        report = inspect(metadata)
        code = 0
    except (OSError, subprocess.TimeoutExpired, PolicyError):
        report = {
            "status": "FAIL",
            "reason": "Zenoh feature policy could not be verified",
        }
        code = 1
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return code


if __name__ == "__main__":
    sys.exit(main())
