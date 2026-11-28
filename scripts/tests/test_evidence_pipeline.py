"""Fail-closed structural evidence admission; no execution claims are made."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'scripts/evidence_pipeline.py'
SPEC = importlib.util.spec_from_file_location('fleet_evidence', SCRIPT)
assert SPEC and SPEC.loader
pipeline = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pipeline)


def pending():
    return {'evidence_status': 'PENDING_EVIDENCE', 'source_commit': 'a' * 40,
            'build_command': 'NOT_RUN', 'build_result': 'NOT_RUN',
            'test_command': 'NOT_RUN', 'test_result': 'NOT_RUN',
            'artifact_digest': None, 'receipt_id': None, 'blockers': ['not run']}


def record():
    return {'schema_version': 1, 'status': 'PENDING_EVIDENCE',
            'counts': {'agents': 111, 'swarms': 33},
            'agents': [dict(pending(), agent_id=f'OCAF-AGENT-{i:03d}') for i in range(1, 112)],
            'swarms': [dict(pending(), swarm_id=f'OCAF-SWARM-{i:02d}') for i in range(1, 34)]}


def completed():
    return {'evidence_status': 'COMPLETE_VERIFIED', 'source_commit': 'a' * 40,
            'build_command': 'cargo build --locked', 'build_result': 'PASS',
            'test_command': 'cargo test --locked', 'test_result': 'PASS',
            'artifact_digest': 'sha256:' + 'b' * 64, 'receipt_id': 'receipt-001', 'blockers': []}


class EvidenceAdmissionTests(unittest.TestCase):
    def test_pending_and_complete_structure(self):
        data = record()
        self.assertEqual(pipeline.validate(data), [])
        data['agents'][0].update(completed())
        self.assertEqual(pipeline.validate(data), [])

    def test_not_run_or_failed_is_not_completion(self):
        for field in ('build_result', 'test_result'):
            for value in ('NOT_RUN', 'FAIL', 'BLOCKED', True):
                with self.subTest(field=field, value=value):
                    data = record()
                    data['agents'][0].update(completed())
                    data['agents'][0][field] = value
                    self.assertTrue(pipeline.validate(data))

    def test_invalid_evidence_fields(self):
        for field, value in [('source_commit', 'bogus'), ('artifact_digest', 'sha256:'),
                             ('artifact_digest', 'sha256:' + 'z' * 64),
                             ('build_command', 'NOT_RUN'), ('test_command', '  '),
                             ('receipt_id', 'NOT_RUN'), ('receipt_id', 123),
                             ('blockers', ['unresolved']), ('blockers', None)]:
            with self.subTest(field=field, value=value):
                data = record()
                data['agents'][0].update(completed())
                data['agents'][0][field] = value
                self.assertTrue(pipeline.validate(data))

    def test_duplicate_swarm_and_unknown_agent_rejected(self):
        data = record()
        data['swarms'][1]['swarm_id'] = data['swarms'][0]['swarm_id']
        self.assertTrue(pipeline.validate(data))
        data = record()
        data['agents'][0]['agent_id'] = 'invented'
        self.assertTrue(pipeline.validate(data))

    def test_malformed_shape_returns_errors(self):
        for data in (None, [], {'counts': None}, {'agents': None},
                     dict(record(), agents=[None] * 111),
                     dict(record(), counts={'agents': 111.0, 'swarms': 33})):
            with self.subTest(data_type=type(data).__name__):
                self.assertTrue(pipeline.validate(data))

    def test_summary_cannot_override_pending_rows(self):
        data = record()
        data['status'] = 'COMPLETE_VERIFIED'
        self.assertTrue(pipeline.validate(data))
        data = record()
        data['agents'][0]['evidence_status'] = 'PASS'
        self.assertTrue(pipeline.validate(data))

    def test_cli_rejects_duplicate_json_keys_without_traceback(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'evidence.json'
            raw = json.dumps(record())
            path.write_text(raw.replace('"schema_version": 1', '"schema_version": 2, "schema_version": 1', 1))
            result = subprocess.run([sys.executable, str(SCRIPT), 'validate', '--output', str(path)],
                                    capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 1)
            self.assertNotIn('Traceback', result.stderr)


if __name__ == '__main__':
    unittest.main()
