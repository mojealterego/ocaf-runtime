#!/usr/bin/env python3
"""Fail-closed checks for the generated SBOM and its source commit binding."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("sbom", type=Path)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--expected-sha", required=True)
    args = parser.parse_args()
    try:
        bom = json.loads(args.sbom.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"invalid SBOM: {exc}", file=sys.stderr)
        return 2
    if bom.get("bomFormat") != "CycloneDX" or bom.get("specVersion") != "1.5":
        print("unsupported SBOM format", file=sys.stderr)
        return 3
    props = {item.get("name"): item.get("value") for item in bom.get("metadata", {}).get("properties", [])}
    if props.get("ocaf.git.sha") != args.expected_sha:
        print("SBOM commit binding mismatch", file=sys.stderr)
        return 4
    components = bom.get("components")
    if not isinstance(components, list) or not components:
        print("SBOM has no components", file=sys.stderr)
        return 5
    if any(item.get("licenses") != [{"license": {"id": "NOASSERTION"}}] for item in components):
        print("unexpected license assertion; independent review required", file=sys.stderr)
        return 6
    digest = hashlib.sha256(args.sbom.read_bytes()).hexdigest()
    print(json.dumps({"status": "PASS", "sha256": digest, "components": len(components), "license_review": "OPEN"}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
