#!/usr/bin/env python3
"""OCAF Oracle Cloud account discovery and Always Free planning (zero writes).

This utility intentionally cannot upgrade subscriptions, edit payment details,
launch instances, authorize IAM, or provision other chargeable resources.
It only queries OCI public APIs through an already-authenticated OCI CLI.
It writes a compact, sanitized, create-only report when --output is specified.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from typing import Callable


class ReadError(Exception):
    """Fail-closed remote read or parsing error."""


OCID_PATTERN = re.compile(r'^ocid1\.(?P<kind>tenancy|compartment)\.oc[0-9]+\.[a-zA-Z0-9._-]+$')
REGION_PATTERN = re.compile(r'^[a-z]{2}-[a-z0-9-]+-[1-9][0-9]*$')


def validate_id(ocid: str, kind: str) -> str:
    match = OCID_PATTERN.fullmatch(ocid)
    if not match or match.group('kind') != kind:
        raise ValueError(f'expected OCI {kind} OCID')
    return ocid


def validate_region(region: str) -> str:
    if not REGION_PATTERN.fullmatch(region):
        raise ValueError('expected OCI region code such as eu-frankfurt-1')
    return region


def build_plan(instances: int = 2, ocpus_each: int = 1,
               memory_gb_each: int = 6, boot_gb_each: int = 50) -> dict:
    """Conservative resource request; it is NOT a statement of actual eligibility.

    OCI home-region entitlement and existing VM/volume allocation must still
    be checked by the owner. A plan never calls the OCI API.
    """
    values = (instances, ocpus_each, memory_gb_each, boot_gb_each)
    if any(type(x) is not int for x in values):
        raise ValueError('all plan fields must be integers')
    if not 1 <= instances <= 2 or not 1 <= ocpus_each <= 2:
        raise ValueError('Always Free planning allows one or two A1 VMs')
    if memory_gb_each < 1 or boot_gb_each < 50:
        raise ValueError('memory must be positive; boot allocation must be >= 50 GB')
    if instances * ocpus_each > 2:
        raise ValueError('A1 CPU request exceeds conservative 2 OCPU budget')
    if instances * memory_gb_each > 12:
        raise ValueError('A1 RAM request exceeds conservative 12 GB budget')
    if instances * boot_gb_each > 200:
        raise ValueError('boot allocation exceeds total conservative 200 GB storage budget')
    return {
        'schema_version': 1,
        'project': 'ocaf-runtime',
        'purpose': 'Oracle Always Free ARM planning only',
        'shape': 'VM.Standard.A1.Flex',
        'architecture': 'aarch64',
        'instances': instances,
        'ocpus_per_instance': ocpus_each,
        'memory_gb_per_instance': memory_gb_each,
        'boot_gb_per_instance': boot_gb_each,
        'allocated_a1_ocpu': instances * ocpus_each,
        'allocated_a1_memory_gb': instances * memory_gb_each,
        'boot_gb': instances * boot_gb_each,
        'provisioning': 'disabled',
        'payment_change': 'manual_only',
        'must_check_before_manual_approval': [
            'OCI home region and A1 capacity',
            'actual tenant Always Free entitlement and other existing resources',
            'boot volumes, snapshots and block storage already allocated',
            'network resources and potential billable services',
            'ARM64 cannot pass current x86_64 pinned Wasmtime gate',
            'no automatic Pay As You Go upgrade or saved payment instrument use'
        ],
    }


def _extract_array(data: object, label: str) -> list[dict]:
    if not isinstance(data, dict) or not isinstance(data.get('data'), list):
        raise ReadError(f'{label}: OCI response is not a data list')
    items = data['data']
    if any(not isinstance(x, dict) for x in items):
        raise ReadError(f'{label}: OCI response contains non-objects')
    return items


def _real_oci(args: list[str]) -> dict:
    binary = shutil.which('oci')
    if not binary:
        raise ReadError('OCI CLI not installed; run from Oracle Cloud Shell or install OCI CLI')
    cmd = [binary, *args]
    try:
        completed = subprocess.run(
            cmd, capture_output=True, text=True, check=False, timeout=45,
            stdin=subprocess.DEVNULL,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise ReadError(f'OCI read command unavailable: {type(exc).__name__}') from None
    if completed.returncode != 0:
        # Intentionally do not echo OCI stderr: it may include identifiers
        # or tokens. Exit code and command label suffice for triage.
        raise ReadError('OCI read command failed (check Cloud Shell IAM, profile and region)')
    if len(completed.stdout) > 4_000_000:
        raise ReadError('OCI response exceeds safety limit')
    try:
        data = json.loads(completed.stdout)
    except json.JSONDecodeError:
        raise ReadError('OCI read returned non-JSON output') from None
    if not isinstance(data, dict):
        raise ReadError('OCI read returned unexpected response type')
    return data


def inspect(tenancy_id: str, compartment_id: str, region: str,
            runner: Callable[[list[str]], dict] = _real_oci) -> dict:
    """Read-only, scoped inventory: home region, instances, VCNs, budgets.

    Strict call list: no self-mutating operations or all-compartment traversal.
    No names, OCIDs, IP addresses, credential fields or raw OCI API payloads
    are returned in evidence or logs.
    """
    validate_id(tenancy_id, 'tenancy')
    validate_id(compartment_id, 'compartment')
    validate_region(region)
    base = ['--region', region, '--output', 'json']
    requests = (
        ('region-subscriptions', ['iam', 'region-subscription', 'list', '--tenancy-id', tenancy_id, '--all', *base]),
        ('instances', ['compute', 'instance', 'list', '--compartment-id', compartment_id, '--all', *base]),
        ('networks', ['network', 'vcn', 'list', '--compartment-id', compartment_id, '--all', *base]),
        ('budgets', ['budgets', 'budget', 'budget', 'list', '--compartment-id', tenancy_id, '--target-type', 'ALL', '--all', *base]),
    )
    data = {}
    for name, cmd in requests:
        result = runner(cmd)
        data[name] = _extract_array(result, name)
    regions = data['region-subscriptions']
    homes = [r.get('region-name') for r in regions if r.get('is-home-region') is True]
    home = homes[0] if len(homes) == 1 and isinstance(homes[0], str) else None
    instances = [r for r in data['instances'] if r.get('lifecycle-state') != 'TERMINATED']
    running = [r for r in instances if r.get('lifecycle-state') == 'RUNNING']
    a1_ocpu = 0.0
    a1_ram = 0.0
    a1_unknown = 0
    for item in instances:
        if item.get('shape') != 'VM.Standard.A1.Flex':
            continue
        shape = item.get('shape-config')
        if not isinstance(shape, dict):
            a1_unknown += 1
            continue
        cpu, ram = shape.get('ocpus'), shape.get('memory-in-gbs')
        if type(cpu) not in (int, float) or type(ram) not in (int, float) or cpu < 0 or ram < 0:
            a1_unknown += 1
            continue
        a1_ocpu += cpu
        a1_ram += ram
    return {
        'schema_version': 1,
        'purpose': 'OCAF read-only OCI inventory; not payment verification',
        'generated_at_utc': datetime.now(timezone.utc).isoformat(),
        'queried_region': region,
        'home_region': home,
        'in_home_region': bool(home and home == region),
        'region_subscriptions_count': len(regions),
        'instance_count': len(instances),
        'running_instances': len(running),
        'a1_instance_count': sum(r.get('shape') == 'VM.Standard.A1.Flex' for r in instances),
        'a1_ocpus_reported': a1_ocpu,
        'a1_memory_gb_reported': a1_ram,
        'a1_capacity_unknown_for_instances': a1_unknown,
        'e2_micro_instance_count': sum(r.get('shape') == 'VM.Standard.E2.1.Micro' for r in instances),
        'network_count': len(data['networks']),
        'budget_count': len(data['budgets']),
        'budget_enforcement': 'soft alerts only; not a hard cost cap',
        'credit_balance': 'not_verified_by_this_tool',
        'account_payment_plan': 'not_verified_by_this_tool',
        'account_upgrade': 'disabled',
        'billing_upgrade_requires_manual_confirmation': True,
        'warning': 'Inventory covers only specified compartment; other compartments and storage may consume Always Free allocations.',
    }


def save_report(destination: Path, payload: dict) -> None:
    """Write confidential account summary once, private by default."""
    previous = os.umask(0o077)
    try:
        with destination.open('x', encoding='utf-8') as f:
            json.dump(payload, f, indent=2, sort_keys=True)
            f.write('\n')
    finally:
        os.umask(previous)


def make_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    planned = sub.add_parser('plan', help='offline Always Free ARM envelope; no OCI calls')
    planned.add_argument('--instances', type=int, default=2)
    planned.add_argument('--ocpus-each', type=int, default=1)
    planned.add_argument('--memory-gb-each', type=int, default=6)
    planned.add_argument('--boot-gb-each', type=int, default=50)
    planned.add_argument('--output', type=Path, help='create-only JSON report')
    audited = sub.add_parser('inspect', help='read-only OCI CLI inventory')
    audited.add_argument('--tenancy-id', required=True, help='OCID, not secret')
    audited.add_argument('--compartment-id', required=True, help='OCID, not secret')
    audited.add_argument('--region', default='eu-frankfurt-1')
    audited.add_argument('--output', type=Path, help='create-only private JSON report')
    return parser


def main(argv: list[str] | None = None) -> int:
    args = make_parser().parse_args(argv)
    try:
        if args.command == 'plan':
            result = build_plan(args.instances, args.ocpus_each,
                                args.memory_gb_each, args.boot_gb_each)
        else:
            result = inspect(args.tenancy_id, args.compartment_id, args.region)
        if args.output:
            save_report(args.output, result)
            print('OCAF Oracle read-only report created (owner-only).')
        else:
            print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    except (ValueError, ReadError, OSError) as exc:
        print(f'OCAF Oracle read-only gate: {type(exc).__name__}: {exc}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
