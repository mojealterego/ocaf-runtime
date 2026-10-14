#!/usr/bin/env python3
"""Read-only, sanitized Azure staging subscription and VM capability summary."""
import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

def az_json(arguments):
    try:
        p = subprocess.run(["az", *arguments, "--output", "json", "--only-show-errors"],
                           capture_output=True, text=True, check=False, timeout=45)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if p.returncode or len(p.stdout) > 2000000:
        return None
    try:
        return json.loads(p.stdout)
    except (ValueError, TypeError):
        return None

def collect():
    account = az_json(["account", "show"])
    if not isinstance(account, dict):
        return {"status": "AZURE_IDENTITY_NOT_AUTHENTICATED",
                "stage_038_complete": False, "production_approved": False}
    vms = az_json(["vm", "list"])
    vms_count = len(vms) if isinstance(vms, list) else None
    report = {
        "schema_version": 1,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "subscription_enabled": account.get("state") == "Enabled",
        "tenant_and_subscription_ids_redacted": True,
        "vm_inventory_accessible": isinstance(vms, list),
        "existing_vm_count": vms_count,
        "location": "westeurope",
        "scope": "Azure ARM inventory (read-only, no resource creation)",
        "resources_created": 0,
        "billing_change_attempted": False,
        "vm_provisioning_authorized": False,
        "stage_038_complete": False,
        "production_approved": False,
        "status": "AZURE_READ_ONLY_OBSERVED" if isinstance(vms, list)
                  else "AZURE_VM_INVENTORY_UNAVAILABLE",
    }
    return report

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    report = collect()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": report["status"], "resources_created": 0, "production_approved": False}))
    return 0 if report["status"] == "AZURE_READ_ONLY_OBSERVED" else 2

if __name__ == "__main__":
    raise SystemExit(main())
