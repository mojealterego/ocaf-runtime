"""Stage 0.38 formal evidence gate: fake signatures are rejected; no CI self-approval."""
import copy
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[1] / 'stage038_production_gate.py'
SPEC = importlib.util.spec_from_file_location('stage038_production_gate', SOURCE)
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)
SHA = 'a' * 40


def openssl(*args):
    subprocess.run(['openssl', *args], check=True, stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL, timeout=10)


class ProductionGateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.bundle = self.root / 'bundle'
        self.bundle.mkdir()
        self.key_dir = self.root / 'trusted-keys'
        self.key_dir.mkdir()
        self.pubkeys = {}
        for role in ('operator', 'auditor'):
            private = self.key_dir / f'{role}.key'
            public = self.key_dir / f'{role}.pem'
            openssl('genpkey', '-algorithm', 'Ed25519', '-out', str(private))
            openssl('pkey', '-in', str(private), '-pubout', '-out', str(public))
            self.pubkeys[role] = public
        self.packet = {
            'schema_version': 1,
            'git_commit': SHA,
            'observed_at_utc': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
            'decision': 'ACCEPT',
            'hosts': [
                {'host_id': 'staging-a', 'machine_id_sha256': '1' * 64,
                 'failure_domain': 'rack-a', 'role': 'coordinator',
                 'systemd_pid1': True, 'cgroup_v2': True, 'ephemeral_ci': False},
                {'host_id': 'staging-b', 'machine_id_sha256': '2' * 64,
                 'failure_domain': 'rack-b', 'role': 'worker',
                 'systemd_pid1': True, 'cgroup_v2': True, 'ephemeral_ci': False}
            ],
            'assertions': {check: True for check in module.REQUIRED_ASSERTIONS},
            'evidence': {},
        }
        for key in module.REQUIRED_EVIDENCE:
            path = f'{key}.json'
            raw = (f'{{"synthetic":"{key}"}}\n').encode()
            (self.bundle / path).write_bytes(raw)
            self.packet['evidence'][key] = {'file': path, 'sha256': hashlib.sha256(raw).hexdigest()}
        self.write_and_sign()

    def write_and_sign(self):
        raw = json.dumps(self.packet, sort_keys=True, separators=(',', ':')).encode()
        (self.bundle / 'acceptance.json').write_bytes(raw)
        for role in ('operator', 'auditor'):
            source = self.root / f'{role}.payload'
            source.write_bytes(module.signature_input(role, raw))
            openssl('pkeyutl', '-sign', '-rawin', '-inkey',
                    str(self.key_dir / f'{role}.key'), '-in', str(source),
                    '-out', str(self.bundle / f'{role}.sig'))

    def evaluate(self, expected=SHA):
        return module.evaluate(self.bundle, expected, self.pubkeys['operator'],
                               self.pubkeys['auditor'])

    def test_signed_complete_multihost_packet_still_requires_human_change_control(self):
        result = self.evaluate()
        self.assertEqual(result['status'], 'CRYPTOGRAPHIC_PACKET_VALID_HUMAN_APPROVAL_REQUIRED')
        self.assertFalse(result['production_approved'])
        self.assertFalse(result['blockers'])
        self.assertEqual(result['validated_evidence_count'], len(module.REQUIRED_EVIDENCE))

    def test_missing_packet_is_no_go(self):
        (self.bundle / 'acceptance.json').unlink()
        self.assertEqual(self.evaluate()['status'], 'NO_GO')

    def test_tampered_artifact_is_no_go_even_with_valid_signed_packet(self):
        (self.bundle / 'two_tenant_isolation.json').write_text('tampered')
        self.assertIn('evidence_invalid:two_tenant_isolation', self.evaluate()['blockers'])

    def test_modified_claim_without_resigning_is_no_go(self):
        self.packet['assertions']['network_partition_recovery'] = False
        (self.bundle / 'acceptance.json').write_text(json.dumps(self.packet))
        self.assertEqual(self.evaluate()['status'], 'NO_GO')

    def test_one_host_same_host_or_ephemeral_runner_is_no_go(self):
        for mutation in ('one', 'duplicate', 'ci'):
            with self.subTest(mutation=mutation):
                original = copy.deepcopy(self.packet)
                if mutation == 'one':
                    self.packet['hosts'].pop()
                elif mutation == 'duplicate':
                    self.packet['hosts'][1]['machine_id_sha256'] = self.packet['hosts'][0]['machine_id_sha256']
                else:
                    self.packet['hosts'][1]['ephemeral_ci'] = True
                self.write_and_sign()
                self.assertEqual(self.evaluate()['status'], 'NO_GO')
                self.packet = original

    def test_mismatch_git_sha_is_no_go(self):
        self.assertIn('commit_mismatch', self.evaluate('b' * 40)['blockers'])

    def test_same_auditor_and_operator_public_key_is_no_go(self):
        self.pubkeys['auditor'] = self.pubkeys['operator']
        self.assertEqual(self.evaluate()['status'], 'NO_GO')

    def test_signature_from_untrusted_key_is_no_go(self):
        self.pubkeys['auditor'] = self.root / 'untrusted.pem'
        private = self.root / 'untrusted.key'
        openssl('genpkey', '-algorithm', 'Ed25519', '-out', str(private))
        openssl('pkey', '-in', str(private), '-pubout', '-out', str(self.pubkeys['auditor']))
        self.assertIn('signature_invalid:auditor', self.evaluate()['blockers'])

    def test_duplicate_json_keys_are_no_go(self):
        f = self.bundle / 'acceptance.json'
        f.write_bytes(f.read_bytes().replace(b'"schema_version":1', b'"schema_version":1,"schema_version":1'))
        self.assertEqual(self.evaluate()['status'], 'NO_GO')

    def test_public_keys_inside_untrusted_evidence_directory_are_refused(self):
        key = self.bundle / 'self_signed.pem'
        key.write_bytes(self.pubkeys['auditor'].read_bytes())
        self.pubkeys['auditor'] = key
        self.assertEqual(self.evaluate()['status'], 'NO_GO')


if __name__ == '__main__':
    unittest.main()
