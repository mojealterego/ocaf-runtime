#!/usr/bin/env python3
"""Verify CycloneDX 1.5 covers every reviewed Cargo workspace member.

This checks the generated SBOM bytes before they are moved into the release
bundle. Signature verification remains the job of the attestation gate.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from .supply_chain import (MAX_MANIFEST_BYTES, REVIEWED_WORKSPACE_MEMBERS,
                           PolicyError, _read_regular, _toml, _workspace_members)

MAX_SBOM_BYTES = 16 * 1024 * 1024
# The standalone G01 binary is a workspace member without a root package
# dependency. Its own BOM must show the reviewed local library it executes.
STANDALONE_BOM_DEPENDENCIES = {
    "swarms/g01": ("ocaf_stage_ledger", "0.1.0"),
}


class SbomError(ValueError):
    """Missing or incomplete workspace SBOM evidence."""


def _unique_fields(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    fields: dict[str, Any] = {}
    for key, value in pairs:
        if key in fields:
            raise SbomError("duplicate CycloneDX JSON field")
        fields[key] = value
    return fields


def _read_bom(path: Path) -> dict[str, Any]:
    try:
        bom = json.loads(_read_regular(path, MAX_SBOM_BYTES).decode("utf-8"),
                         object_pairs_hook=_unique_fields)
    except (PolicyError, UnicodeError, ValueError) as exc:
        raise SbomError("invalid workspace SBOM") from exc
    if not isinstance(bom, dict) or bom.get("bomFormat") != "CycloneDX" or bom.get("specVersion") != "1.5":
        raise SbomError("unexpected CycloneDX version")
    return bom


def _component_identity(bom: dict[str, Any]) -> tuple[str, str]:
    metadata = bom.get("metadata")
    component = metadata.get("component") if isinstance(metadata, dict) else None
    if not isinstance(component, dict):
        raise SbomError("workspace SBOM lacks package identity")
    name, version = component.get("name"), component.get("version")
    if not isinstance(name, str) or not isinstance(version, str):
        raise SbomError("workspace SBOM has invalid package identity")
    return name, version


def _component_occurrences(components: list[Any], name: str, version: str) -> int:
    return sum(
        isinstance(component, dict) and
        (component.get("name"), component.get("version")) == (name, version)
        for component in components
    )


def verify(repository: Path) -> dict[str, Any]:
    root = repository.absolute()
    try:
        manifest = _toml(_read_regular(root / "Cargo.toml", MAX_MANIFEST_BYTES))
        members = _workspace_members(root, manifest)
    except PolicyError as exc:
        raise SbomError("unreviewed Cargo workspace") from exc
    package = manifest.get("package")
    if not isinstance(package, dict):
        raise SbomError("missing root Cargo package")
    expected_root = (package.get("name"), package.get("version"))
    aggregate = _read_bom(root / "bom.json")
    if _component_identity(aggregate) != expected_root:
        raise SbomError("aggregate SBOM describes another root")
    components = aggregate.get("components")
    if not isinstance(components, list) or not components:
        raise SbomError("aggregate SBOM has no components")
    for name, version in members.items():
        if _component_occurrences(components, name, version) != 1:
            raise SbomError("aggregate SBOM omits or duplicates a workspace member")
    for member_path in manifest.get("workspace", {}).get("members", [])[1:]:
        name, version = REVIEWED_WORKSPACE_MEMBERS[member_path]
        member_bom = _read_bom(root / member_path / "bom.json")
        if _component_identity(member_bom) != (name, version):
            raise SbomError("member SBOM package identity mismatch")
        if member_path in STANDALONE_BOM_DEPENDENCIES:
            dependency_name, dependency_version = STANDALONE_BOM_DEPENDENCIES[member_path]
            member_components = member_bom.get("components")
            if (not isinstance(member_components, list) or
                    _component_occurrences(member_components, dependency_name,
                                           dependency_version) != 1):
                raise SbomError("standalone member SBOM omits or duplicates reviewed dependency")
    return {"status": "PASS", "workspace_members": members,
            "aggregate_components": len(components),
            "scope": "workspace-cyclonedx-content-not-signature-verification"}
def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        report = verify(args.repo)
        code = 0
    except SbomError as exc:
        report = {"status": "FAIL", "reason": str(exc)}
        code = 1
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return code


if __name__ == "__main__":
    sys.exit(main())
