import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    'gate', Path(__file__).resolve().parents[1] / 'scripts' / 'ocaf_make_execution_gate.py'
)
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


class MakeEvidenceGateTest(unittest.TestCase):
    def test_roster_has_exactly_33_unique_groups(self):
        self.assertEqual(list(gate.SCENARIOS), list(range(1, 34)))
        self.assertEqual(len(set(gate.SCENARIOS.values())), 33)

    def test_missing_runs_are_not_success(self):
        state = gate.classify([])
        self.assertEqual(state, 'NEVER_EXECUTED')

    def test_make_success_is_not_git_commit(self):
        state = gate.classify([{'status': 1, 'executionId': 'abc'}])
        self.assertEqual(state, 'EXECUTED_NOT_DELIVERED')

    def test_error_and_pending(self):
        self.assertEqual(gate.classify([{'status': 3}]), 'FAILED')
        self.assertEqual(gate.classify([{'status': 'pending'}]), 'PENDING')
        self.assertEqual(gate.classify([{'status': 'garbage'}]), 'UNKNOWN')

    def test_latest_is_selected_by_run_timestamp(self):
        entries = [
            {'status': 3, 'startedAt': '2026-10-19T01:00:00Z'},
            {'status': 1, 'startedAt': '2026-10-19T02:00:00Z'},
        ]
        self.assertEqual(gate.classify(entries), 'EXECUTED_NOT_DELIVERED')

    def test_reject_malformed_api_response(self):
        with self.assertRaises(gate.GateError):
            gate.extract_logs({'scenarioLogs': 'bogus'})

    def test_require_explicit_complete_sha_bound_evidence(self):
        self.assertFalse(gate.is_delivered({'completion_state': 'SUCCESS'}))
        self.assertFalse(gate.is_delivered({'completion_state': 'PASS', 'base_sha': 'f' * 40}))
        self.assertTrue(gate.is_delivered({
            'completion_state': 'VERIFIED',
            'implementation_sha': 'a' * 40,
            'ci_run_id': '123456789',
            'ci_conclusion': 'success',
        }, verifier=lambda sha, run_id: True))
        self.assertFalse(gate.is_delivered({
            'completion_state': 'VERIFIED',
            'implementation_sha': 'a' * 40,
            'ci_run_id': '123',
            'ci_conclusion': 'failure',
        }))

    def test_network_fail_closed(self):
        with patch.object(gate, 'request_json', side_effect=gate.GateError('denied')):
            report = gate.audit('eu1.make.com', token='no_token', groups={1: 999})
            self.assertEqual(report['groups']['G1']['state'], 'API_ERROR')
            self.assertEqual(report['delivered'], 0)


if __name__ == '__main__':
    unittest.main()
