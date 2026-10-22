#!/usr/bin/env python3
"""Validate the OCAF/OMAs/OMAt Zenoh routing and health contract."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
from typing import Any

class ContractError(ValueError):
    pass

def validate(document: Any) -> dict[str, Any]:
    if not isinstance(document, dict) or document.get("schema_version") != 1:
        raise ContractError("unsupported or malformed contract")
    if document.get("transport") != "zenoh":
        raise ContractError("transport must be Zenoh")
    services = document.get("services")
    if not isinstance(services, dict) or set(services) != {"ocaf", "omas", "omat"}:
        raise ContractError("exactly OCAF, OMAs and OMAt services are required")
    for name, service in services.items():
        if not isinstance(service, dict) or not service.get("namespace") or not service.get("health_key"):
            raise ContractError(f"service {name} lacks namespace or health key")
    allowed = document.get("allowed_links")
    pairs = {(item.get("from"), item.get("to")) for item in allowed if isinstance(item, dict)} if isinstance(allowed, list) else set()
    if pairs != {("ocaf", "omas"), ("omas", "omat"), ("omat", "omas")}:
        raise ContractError("routing must be OCAF↔OMAs↔OMAt without a bypass")
    forbidden = document.get("forbidden_links")
    if not any(isinstance(item, dict) and item.get("from") == "ocaf" and item.get("to") == "omat" for item in forbidden or []):
        raise ContractError("direct OCAF to OMAt route must be forbidden")
    health = document.get("health")
    if not isinstance(health, dict) or health.get("ostc_enabled") is not False or health.get("active_admission") != "DENY":
        raise ContractError("OSTc/ACTIVE gate is not fail-closed")
    required = health.get("required")
    if not isinstance(required, list) or set(required) != {"zenoh-session", "omas", "omat", "tenant-policy", "replay-store"}:
        raise ContractError("health dependencies are incomplete")
    negatives = document.get("negative_acceptance")
    if not isinstance(negatives, list) or len(negatives) < 6:
        raise ContractError("negative acceptance coverage is incomplete")
    return {"status": "PASS", "services": sorted(services), "allowed_links": len(pairs), "active_admission": "DENY"}

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = validate(json.loads(args.contract.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError, ContractError) as exc:
        print(json.dumps({"status": "FAIL", "reason": str(exc)}, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
