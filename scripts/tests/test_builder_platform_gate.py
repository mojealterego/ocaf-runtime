import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import verify_builder_platform_gate as gate


class PlatformGateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo'
        self.release = self.root / 'release'
        self.repo.mkdir()
        self.release.mkdir()
        self.matrix = self.repo / gate.MATRIX
        self.matrix.parent.mkdir(parents=True)
        self.matrix.write_text(json.dumps({'schema_version': 1, 'platform_target_count': 8,
            'mode_switch': ['Code', 'NoCode'], 'workspace_switch': ['Local', 'Cloud'],
            'cloud': {'separate_native_application': False},
            'targets': [{'platform_id': p, 'distribution': d} for p, d in gate.PLATFORMS.items()]}))
        self.git('init', '-q', '-b', 'main')
        self.git('add', '.')
        self.commit()
        self.head = self.git('rev-parse', 'HEAD').strip()
        self.key = Ed25519PrivateKey.from_private_bytes(bytes(range(32)))
        self.public = self.key.public_key().public_bytes_raw().hex()
        self.records = []
        for platform, distribution in gate.PLATFORMS.items():
            binary = self.payload(platform + '/package', ('fixture ' + platform).encode())
            basis = {'platform': platform, 'git_commit_sha': self.head,
                     'binary_sha256': binary['sha256'], 'status': 'PASS'}
            install = self.payload(platform + '/install.json', {**basis, 'distribution': distribution,
                'runner_id': 'runner-' + platform, 'device_id': 'device-' + platform})
            sbom = self.payload(platform + '/sbom.json', {'bomFormat': 'CycloneDX', 'specVersion': '1.6',
                'components': [{'name': 'ocaf', 'version': '1'}], 'version': 1})
            suites = []
            for mode, workspace in gate.COMBINATIONS:
                log = self.payload(platform + '/' + mode + '-' + workspace + '.json', {**basis,
                    'mode': mode, 'workspace': workspace, 'runner_id': 'runner-' + platform,
                    'steps': [{'step': step, 'status': 'PASS'} for step in gate.steps_for(workspace)]})
                suites.append({'mode': mode, 'workspace': workspace, 'log': log})
            self.records.append({'platform': platform, 'distribution': distribution, 'binary': binary,
                                 'install': install, 'sbom': sbom, 'suites': suites})
        self.sign()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], text=True)

    def commit(self):
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture')

    def payload(self, relative, data):
        path = self.release / relative
        path.parent.mkdir(exist_ok=True, parents=True)
        raw = data if isinstance(data, bytes) else json.dumps(data).encode()
        path.write_bytes(raw)
        return {'path': relative, 'sha256': hashlib.sha256(raw).hexdigest()}

    def sign(self):
        value = {'schema_version': 2, 'git_commit_sha': self.head, 'platforms': self.records}
        signed = {**value, 'signature_hex': self.key.sign(gate.canonical(value)).hex()}
        (self.release / 'PLATFORM_EVIDENCE.json').write_bytes(gate.canonical(signed))

    def verify(self):
        return gate.verify(self.repo, self.release, self.head, self.public)

    def test_complete_matrix_is_integrity_only(self):
        result = self.verify()
        self.assertEqual(result['platforms_verified'], 8)
        self.assertEqual(result['suites_verified'], 32)
        self.assertFalse(result['production_approved'])
        self.assertFalse(result['native_builds_executed_by_verifier'])

    def test_missing_platform_and_missing_cloud_mode_fail(self):
        saved = self.records.pop()
        self.sign()
        with self.assertRaises(gate.GateError): self.verify()
        self.records.append(saved)
        saved['suites'].pop()
        self.sign()
        with self.assertRaises(gate.GateError): self.verify()

    def test_wrong_key_and_altered_binary_fail(self):
        with self.assertRaises(gate.GateError):
            gate.verify(self.repo, self.release, self.head, Ed25519PrivateKey.generate().public_key().public_bytes_raw().hex())
        (self.release / self.records[0]['binary']['path']).write_bytes(b'changed')
        with self.assertRaises(gate.GateError): self.verify()

    def test_signed_cloud_log_without_authentication_fails(self):
        suite = next(s for s in self.records[0]['suites'] if s['workspace'] == 'Cloud')
        log = json.loads((self.release / suite['log']['path']).read_bytes())
        log['steps'] = [x for x in log['steps'] if x['step'] != 'authenticate']
        suite['log'] = self.payload(suite['log']['path'], log)
        self.sign()
        with self.assertRaises(gate.GateError): self.verify()

    def test_symlink_and_wrong_sha_fail(self):
        with self.assertRaises(gate.GateError):
            gate.verify(self.repo, self.release, '0' * 40, self.public)
        path = self.release / self.records[0]['binary']['path']
        target = self.root / 'outside'
        target.write_bytes(path.read_bytes())
        path.unlink()
        path.symlink_to(target)
        with self.assertRaises(gate.GateError): self.verify()

    def test_hidden_matrix_edit_fails(self):
        self.git('update-index', '--assume-unchanged', gate.MATRIX)
        self.matrix.write_text('{}')
        with self.assertRaises(gate.GateError): self.verify()

    def test_replacement_commit_cannot_substitute_pinned_matrix(self):
        original = self.matrix.read_bytes()
        matrix = json.loads(original)
        matrix['targets'][0]['status'] = 'FORGED_STATUS'
        altered = json.dumps(matrix).encode()
        self.matrix.write_bytes(altered)
        self.git('add', gate.MATRIX)
        self.commit()
        replacement = self.git('rev-parse', 'HEAD').strip()
        self.git('reset', '--hard', self.head)
        self.git('replace', self.head, replacement)
        self.matrix.write_bytes(altered)
        with self.assertRaises(gate.GateError): self.verify()
        self.matrix.write_bytes(original)
        self.assertEqual(self.verify()['platforms_verified'], 8)

    def test_git_environment_cannot_redirect_provenance_to_other_repo(self):
        mirror = self.root / 'mirror'
        self.git('clone', '--quiet', '--no-hardlinks', str(self.repo), str(mirror))
        (self.repo / 'marker').write_text('new revision')
        self.git('add', 'marker')
        self.commit()
        with patch.dict(os.environ, {'GIT_DIR': str(mirror / '.git'),
                                    'GIT_WORK_TREE': str(mirror)}):
            with self.assertRaises(gate.GateError): self.verify()

    def test_nested_directory_is_not_the_pinned_repository_root(self):
        nested = self.repo / 'nested'
        matrix = nested / gate.MATRIX
        matrix.parent.mkdir(parents=True)
        matrix.write_bytes(self.matrix.read_bytes())
        with self.assertRaises(gate.GateError):
            gate.verify(nested, self.release, self.head, self.public)

    def test_duplicate_platform_or_evidence_path_fails(self):
        record = self.records[-1]
        record['platform'] = self.records[0]['platform']
        self.sign()
        with self.assertRaises(gate.GateError): self.verify()
        record['platform'] = 'kaios'
        record['binary'] = self.records[0]['binary']
        self.sign()
        with self.assertRaises(gate.GateError): self.verify()

    def test_duplicate_json_and_untrusted_fields_fail(self):
        path = self.release / 'PLATFORM_EVIDENCE.json'
        path.write_bytes(path.read_bytes().replace(b'"schema_version":2', b'"schema_version":2,"schema_version":2'))
        with self.assertRaises(gate.GateError): self.verify()
        self.records[0]['production_approved'] = True
        self.sign()
        with self.assertRaises(gate.GateError): self.verify()


if __name__ == '__main__':
    unittest.main()
