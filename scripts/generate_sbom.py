#!/usr/bin/env python3
"""Generate a deterministic CycloneDX SBOM from Cargo.lock.

The generator does not claim license approval.  Cargo.lock provides package
identity, source and checksums; license review remains an explicit gate and is
represented as ``NOASSERTION`` until independently verified.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path


def git_sha(repo: Path) -> str:
    configured = os.environ.get("OCAF_GIT_SHA", "").strip()
    if configured:
        return configured
    try:
        return subprocess.check_output(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "UNKNOWN"


def purl(name: str, version: str) -> str:
    return f"pkg:cargo/{name}@{version}"


def build_bom(lockfile: Path, repo: Path) -> dict:
    data = tomllib.loads(lockfile.read_text(encoding="utf-8"))
    components = []
    for package in sorted(data.get("package", []), key=lambda item: (item["name"], item["version"], item.get("source", ""))):
        name = package["name"]
        version = package["version"]
        component = {
            "type": "library",
            "bom-ref": purl(name, version),
            "group": "crates.io" if package.get("source", "").startswith("registry+") else None,
            "name": name,
            "version": version,
            "purl": purl(name, version),
            "licenses": [{"license": {"id": "NOASSERTION"}}],
        }
        source = package.get("source")
        if source:
            component["externalReferences"] = [{"type": "distribution", "url": source}]
        checksum = package.get("checksum")
        if checksum:
            component["hashes"] = [{"alg": "SHA-256", "content": checksum}]
        components.append({key: value for key, value in component.items() if value is not None})

    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": "urn:uuid:ocaf-runtime-cargo-lock",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": "ocaf-runtime",
                "name": "ocaf-runtime",
                "version": git_sha(repo),
            },
            "properties": [
                {"name": "ocaf.git.sha", "value": git_sha(repo)},
                {"name": "ocaf.license.review", "value": "NOASSERTION"},
                {"name": "ocaf.source.lockfile", "value": str(lockfile.relative_to(repo))},
            ],
        },
        "components": components,
    }
    return bom


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--lockfile", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve()
    lockfile = (args.lockfile or repo / "Cargo.lock").resolve()
    if not lockfile.is_file():
        print(f"missing Cargo.lock: {lockfile}", file=sys.stderr)
        return 2
    bom = build_bom(lockfile, repo)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(bom, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    digest = hashlib.sha256(args.output.read_bytes()).hexdigest()
    print(json.dumps({"components": len(bom["components"]), "sha256": digest, "output": str(args.output)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
