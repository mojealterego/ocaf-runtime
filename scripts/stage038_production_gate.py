#!/usr/bin/env python3
"""Fail-closed offline verifier for OCAF 0.38 external approval evidence.

A signed packet is not proof of physically independent machines, qualified
reviewers, or change-control authorization. This tool never returns a
production-approved status; final release authorization stays external.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile

REQUIRED_EVIDENCE = (
    'host_a_systemd', 'host_b_systemd', 'multi_host_mtls_rpc',
    'two_tenant_isolation', 'network_partition_recovery',
    'sigkill_replay_fencing', 'publisher_tls_revocation', 'soak_100_cycles',
    'supply_chain_provenance', 'backup_restore',
    'independent_security_audit', 'operator_signoff',
)
REQUIRED_ASSERTIONS = (
    'multi_host_mtls_rpc', 'two_tenant_isolation',
    'network_partition_recovery', 'sigkill_replay_fencing',
    'publisher_tls_revocation', 'soak_100_cycles',
    'independent_audit_clear', 'formal_operator_approval',
)
REQUIRED_HOST_KEYS = frozenset({
    'host_id', 'machine_id_sha256', 'failure_domain', 'role',
    'systemd_pid1', 'cgroup_v2', 'ephemeral_ci',
})
REQUIRED_PACKET_KEYS = frozenset({
    'schema_version', 'git_commit', 'observed_at_utc', 'decision',
    'hosts', 'assertions', 'evidence',
})
SHA256 = re.compile(r'[0-9a-f]{64}\Z')
COMMIT = re.compile(r'[0-9a-f]{40}\Z')
LABEL = re.compile(r'[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}\Z')
MAX_PACKET = 256 * 1024
MAX_EVIDENCE = 10 * 1024 * 1024
MAX_AGE = timedelta(days=30)


def signature_input(role: str, payload: bytes) -> bytes:
    if role not in ('operator', 'auditor'):
        raise ValueError('unrecognized signer role')
    return b'OCAF038-ACCEPTANCE-V1\0' + role.encode('ascii') + b'\0' + payload


def _unique_keys(items: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in items:
        if key in result:
            raise ValueError('duplicate JSON key')
        result[key] = value
    return result


def _reject_constant(_value: str) -> None:
    raise ValueError('non-finite JSON constant')


def _safe_file_bytes(path: Path, maximum: int) -> bytes:
    if path.is_symlink():
        raise ValueError('symlink rejected')
    flags = os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_CLOEXEC', 0)
    fd = os.open(path, flags)
    with os.fdopen(fd, 'rb') as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or not 0 < info.st_size <= maximum:
            raise ValueError('invalid regular file size/links')
        data = source.read(maximum + 1)
        if not 0 < len(data) <= maximum:
            raise ValueError('file changed/too large')
        return data


def _is_bundle_file(bundle: Path, filename: str) -> bool:
    return isinstance(filename, str) and LABEL.fullmatch(filename) is not None and filename not in ('.', '..')


def _date_is_fresh(value: object, now: datetime) -> bool:
    if not isinstance(value, str) or not value.endswith('Z'):
        return False
    try:
        parsed = datetime.fromisoformat(value.replace('Z', '+00:00'))
    except ValueError:
        return False
    if parsed.tzinfo is None:
        return False
    age = now - parsed.astimezone(timezone.utc)
    return timedelta(0) <= age <= MAX_AGE


def _valid_hosts(hosts: object) -> bool:
    if not isinstance(hosts, list) or len(hosts) != 2:
        return False
    ids = set()
    machine_ids = set()
    domains = set()
    roles = set()
    for host in hosts:
        if not isinstance(host, dict) or frozenset(host) != REQUIRED_HOST_KEYS:
            return False
        if not all(isinstance(host[field], str) and LABEL.fullmatch(host[field])
                   for field in ('host_id', 'failure_domain')):
            return False
        if not isinstance(host['machine_id_sha256'], str) or not SHA256.fullmatch(host['machine_id_sha256']):
            return False
        if (host['systemd_pid1'] is not True or host['cgroup_v2'] is not True
                or host['ephemeral_ci'] is not False):
            return False
        if host['role'] not in ('coordinator', 'worker'):
            return False
        ids.add(host['host_id'])
        machine_ids.add(host['machine_id_sha256'])
        domains.add(host['failure_domain'])
        roles.add(host['role'])
    return len(ids) == 2 and len(machine_ids) == 2 and len(domains) == 2 and roles == {'coordinator', 'worker'}


def _public_der(path: Path) -> bytes:
    # Open the external trust anchor once. Never let OpenSSL re-read the
    # mutable path after the checked descriptor has been closed.
    pinned_pem = _safe_file_bytes(path, 16 * 1024)
    result = subprocess.run(
        ['openssl', 'pkey', '-pubin', '-inform', 'PEM', '-outform', 'DER'],
        input=pinned_pem, capture_output=True, check=False, timeout=10,
    )
    if result.returncode != 0 or not result.stdout:
        raise ValueError('unreadable pinned public key')
    return result.stdout


def _verify_signature(role: str, public_der: bytes, packet: bytes, signature: bytes) -> bool:
    if len(signature) != 64:
        return False
    with tempfile.TemporaryDirectory(prefix='ocaf038-verify-') as work:
        base = Path(work)
        payload_file = base / 'payload'
        signature_file = base / 'signature'
        public_file = base / 'public.der'
        payload_file.write_bytes(signature_input(role, packet))
        signature_file.write_bytes(signature)
        public_file.write_bytes(public_der)
        try:
            result = subprocess.run(
                ['openssl', 'pkeyutl', '-verify', '-rawin', '-pubin',
                 '-keyform', 'DER', '-inkey', str(public_file),
                 '-in', str(payload_file), '-sigfile', str(signature_file)],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                timeout=10, check=False,
            )
        except (OSError, subprocess.TimeoutExpired):
            return False
        return result.returncode == 0


def evaluate(bundle: Path, expected_commit: str, operator_key: Path,
             auditor_key: Path, now: datetime | None = None) -> dict:
    now = now or datetime.now(timezone.utc)
    blockers: list[str] = []
    validated = 0
    if now.tzinfo is None:
        raise ValueError('now must be timezone-aware')
    if not isinstance(expected_commit, str) or not COMMIT.fullmatch(expected_commit):
        blockers.append('expected_commit_invalid')
    try:
        if bundle.is_symlink() or not bundle.is_dir():
            raise ValueError('bundle directory invalid')
        bundle_path = bundle.resolve(strict=True)
        for role, key in [('operator', operator_key), ('auditor', auditor_key)]:
            if key.is_symlink() or not key.is_file() or key.resolve(strict=True).is_relative_to(bundle_path):
                blockers.append(f'untrusted_public_key_location:{role}')
        operator_der = _public_der(operator_key)
        auditor_der = _public_der(auditor_key)
        if operator_der == auditor_der:
            blockers.append('signer_keys_not_distinct')
        raw_packet = _safe_file_bytes(bundle / 'acceptance.json', MAX_PACKET)
        packet = json.loads(raw_packet, object_pairs_hook=_unique_keys, parse_constant=_reject_constant)
        if not isinstance(packet, dict) or frozenset(packet) != REQUIRED_PACKET_KEYS or type(packet.get('schema_version')) is not int or packet['schema_version'] != 1:
            raise ValueError('invalid packet schema')
        if packet.get('git_commit') != expected_commit:
            blockers.append('commit_mismatch')
        if not _date_is_fresh(packet.get('observed_at_utc'), now):
            blockers.append('packet_stale_or_future')
        if packet.get('decision') != 'ACCEPT':
            blockers.append('operator_decision_not_accept')
        if not _valid_hosts(packet.get('hosts')):
            blockers.append('two_distinct_hardened_staging_hosts_unverified')
        claims = packet.get('assertions')
        if not isinstance(claims, dict) or set(claims) != set(REQUIRED_ASSERTIONS) or any(claims[k] is not True for k in REQUIRED_ASSERTIONS):
            blockers.append('required_claims_missing')
        evidence = packet.get('evidence')
        if not isinstance(evidence, dict) or set(evidence) != set(REQUIRED_EVIDENCE):
            blockers.append('evidence_inventory_incomplete')
        else:
            names = set()
            for name in REQUIRED_EVIDENCE:
                entry = evidence[name]
                if (not isinstance(entry, dict) or set(entry) != {'file', 'sha256'}
                        or not _is_bundle_file(bundle, entry.get('file'))
                        or not isinstance(entry.get('sha256'), str)
                        or not SHA256.fullmatch(entry['sha256'])
                        or entry['file'] in names):
                    blockers.append(f'evidence_invalid:{name}')
                    continue
                names.add(entry['file'])
                try:
                    artifact = _safe_file_bytes(bundle / entry['file'], MAX_EVIDENCE)
                    if hashlib.sha256(artifact).hexdigest() != entry['sha256']:
                        raise ValueError('digest mismatch')
                except (OSError, ValueError):
                    blockers.append(f'evidence_invalid:{name}')
                    continue
                validated += 1
        for role, trusted_der in [('operator', operator_der), ('auditor', auditor_der)]:
            try:
                sig = _safe_file_bytes(bundle / f'{role}.sig', 1024)
            except (OSError, ValueError):
                blockers.append(f'signature_invalid:{role}')
                continue
            if not _verify_signature(role, trusted_der, raw_packet, sig):
                blockers.append(f'signature_invalid:{role}')
    except (OSError, ValueError, TypeError, json.JSONDecodeError, subprocess.TimeoutExpired):
        blockers.append('packet_or_trust_material_invalid')
    return {
        'schema_version': 1,
        'status': ('NO_GO' if blockers else 'CRYPTOGRAPHIC_PACKET_VALID_HUMAN_APPROVAL_REQUIRED'),
        'production_approved': False,
        'validated_evidence_count': validated,
        'required_evidence_count': len(REQUIRED_EVIDENCE),
        'blockers': sorted(set(blockers)),
        'limitations': (
            'Signed JSON asserts evidence integrity and two declared hosts, not physical independence, '
            'auditor credentials/independence, absence of vulnerabilities, or production authorization. '
            'Require separate human review and change-control approval.'
        ),
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--directory', type=Path, required=True)
    ap.add_argument('--expected-commit', required=True)
    ap.add_argument('--operator-public-key', type=Path, required=True)
    ap.add_argument('--auditor-public-key', type=Path, required=True)
    args = ap.parse_args(argv)
    report = evaluate(args.directory, args.expected_commit,
                      args.operator_public_key, args.auditor_public_key)
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write('\n')
    return 2 if report['blockers'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
