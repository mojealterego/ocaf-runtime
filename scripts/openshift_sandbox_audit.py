#!/usr/bin/env python3
"""Read-only, namespace-scoped Red Hat Developer Sandbox diagnostic (NO-GO)."""
from __future__ import annotations

import argparse
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

NAMESPACE = "mojealterego-dev"
MAX_CAPTURE = 1024 * 1024
KINDS = ("virtualmachines.kubevirt.io", "virtualmachineinstances.kubevirt.io",
         "resourcequotas", "limitranges", "pods")

def command(argv: list[str]) -> tuple[bool, str]:
    """Never expose stderr or untrusted API content in public CI logs."""
    try:
        p = subprocess.run(["kubectl", "--request-timeout=15s", *argv],
                           capture_output=True, text=True, timeout=25, check=False)
        if len(p.stdout) > MAX_CAPTURE or len(p.stderr) > MAX_CAPTURE:
            return False, ""
        return p.returncode == 0, p.stdout.strip()
    except (OSError, subprocess.TimeoutExpired, UnicodeError):
        return False, ""

def authorization(verb: str, resource: str) -> str:
    ok, value = command(["auth", "can-i", verb, resource, "-n", NAMESPACE])
    return value if ok and value in ("yes", "no") else "unavailable"

def inventory(resource: str) -> dict[str, Any]:
    ok, content = command(["-n", NAMESPACE, "get", resource, "-o", "json"])
    if not ok:
        return {"accessible": False, "count": None}
    try:
        data = json.loads(content)
        items = data.get("items")
        if not isinstance(items, list) or len(items) > 20000:
            raise ValueError("invalid item list")
        result: dict[str, Any] = {"accessible": True, "count": len(items)}
        if resource == "virtualmachines.kubevirt.io":
            running = 0
            for item in items:
                if isinstance(item, dict) and item.get("status", {}).get("printableStatus") == "Running":
                    running += 1
            result["running"] = running
        return result
    except (ValueError, TypeError, AttributeError):
        return {"accessible": False, "count": None}

def classify_kubectl_error(stderr: str) -> str:
    """Return a strict enum; NEVER write or propagate untrusted stderr."""
    text = stderr.lower()
    if "unauthorized" in text or "you must be logged in" in text or "the server has asked for the client to provide credentials" in text:
        return "AUTHENTICATION_FAILED"
    if "forbidden" in text or "permission denied" in text:
        return "RBAC_DENIED"
    if "x509:" in text or "certificate signed by unknown authority" in text:
        return "TLS_VALIDATION_FAILED"
    if "couldn't get current server api group list" in text or "connection refused" in text or "no such host" in text or "i/o timeout" in text:
        return "NETWORK_OR_API_DISCOVERY"
    if "the server doesn't have a resource type" in text or "not found" in text:
        return "API_RESOURCE_UNAVAILABLE"
    return "OTHER_OR_UNCLASSIFIED"


def scoped_access_probe() -> dict[str, object]:
    """Read a fixed namespace's pods, exposing only a bounded diagnostic category."""
    try:
        p = subprocess.run(
            ["kubectl", "--request-timeout=15s", "-n", NAMESPACE, "get", "pods", "-o", "json"],
            capture_output=True, text=True, timeout=25, check=False,
        )
        if len(p.stdout) > MAX_CAPTURE or len(p.stderr) > MAX_CAPTURE:
            return {"ok": False, "failure_category": "OUTPUT_TOO_LARGE"}
        return {
            "ok": p.returncode == 0,
            "failure_category": "NONE" if p.returncode == 0 else classify_kubectl_error(p.stderr),
        }
    except (OSError, subprocess.TimeoutExpired, UnicodeError):
        return {"ok": False, "failure_category": "CLIENT_OR_TIMEOUT_ERROR"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    access_probe = scoped_access_probe()
    auth = {
        "read_vm": authorization("get", "virtualmachines.kubevirt.io"),
        "create_vm": authorization("create", "virtualmachines.kubevirt.io"),
        "read_vmi": authorization("get", "virtualmachineinstances.kubevirt.io"),
    }
    resources = {resource: inventory(resource) for resource in KINDS}
    report = {
        "schema_version": 1,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "namespace": NAMESPACE,
        "checks": auth,
        "access_probe": access_probe,
        "resources": resources,
        "scope": "read-only-kubernetes-api",
        "vm_created": False,
        "vm_started": False,
        "systemd_pid1_verified": False,
        "signed_wasi_accepted": False,
        "stage_038_complete": False,
        "production_approved": False,
        "status": "OBSERVED_NOT_ACCEPTED" if resources["virtualmachines.kubevirt.io"]["accessible"] else "ACCESS_NOT_CONFIRMED",
    }
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"status": report["status"], "report": str(args.report),
                      "namespace": NAMESPACE, "access_probe": access_probe,
                      "production_approved": False}))
    return 0 if resources["virtualmachines.kubevirt.io"]["accessible"] else 2

if __name__ == "__main__":
    raise SystemExit(main())
