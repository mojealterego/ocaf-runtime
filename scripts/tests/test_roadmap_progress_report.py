"""Regression tests for conservative, read-only OCAF roadmap progress accounting."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts.roadmap_progress_report import ReportError, evaluate


class RoadmapProgressTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'docs/roadmap').mkdir(parents=True)
        self.path = self.root / 'docs/roadmap/canonical_v11_inventory.json'
        main_states = (
            ['PARTIALLY_VERIFIED'] * 2
            + ['COMPLETED_BY_OPERATOR_TECHNICAL_ACCEPTANCE_OPEN']
            + ['CLOSED_BY_OPERATOR_SIMULATION_TECHNICAL_OPEN']
            + ['ACTIVE_TECHNICAL_OPEN'] * 3
            + ['PRODUCTION_ACCEPTED_SIMULATION']
            + ['UNSPECIFIED_IN_CURRENT_REPO'] * 11
            + ['HISTORICAL_NEEDS_VERIFICATION'] * 25
            + ['PLANNED_OR_UNVERIFIED'] * 56
            + ['PLANNED'] * 234
        )
        tech_states = (
            ['PARTIALLY_IMPLEMENTED_ACCEPTANCE_OPEN',
             'IMPLEMENTED_LAB_EVIDENCE_ACCEPTANCE_OPEN',
             'IMPLEMENTED_TEST_HARNESS_ACCEPTANCE_OPEN',
             '**PASS 12 tests**, CI host NO-GO',
             '**PASS 17 tests**',
             '**PASS 10 tests**',
             '**REAL KVM API 12 and CREATE_VM PASS as root, run 38704086975**',
             '**Static chroot config + 6 tests PASS; live jailer / TOCTOU runtime proof OPEN**',
             '**LIVE EXECUTOR WRITTEN; 6 contract tests PASS; host KVM NO-GO**',
             '**REAL kernel Aya attach and A/B negative+recovery PASS, run 38704086975**']
            + ['RUNNER_IMPLEMENTED_LIVE_TWO_KERNEL_EVIDENCE_PENDING'] * 2
            + ['PARTIAL_QUORUM_CERTIFICATE_HARDENING_THREE_HOST_EVIDENCE_PENDING']
            + ['UNVERIFIED_ACCEPTANCE_OPEN']
            + ['**NO-GO / hosts unavailable**']
            + ['**NO-GO**'] * 2
            + ['HISTORICAL_LAB_SCOPE_LIMITED'] * 13
            + ['PLANNED'] * 34
        )
        self.data = {
            'schema_version': 1,
            'version': 'v11-original-builder-scope-restored',
            'repository': 'mojealterego/ocaf-runtime', 'branch': 'main',
            'main_count': 334, 'technical_count': 64, 'reference_count': 33,
            'production_approved': False,
            'main': [{'id': f'm{i}', 'title': f'Main {i}', 'status': s}
                     for i, s in enumerate(main_states)],
            'technical': [{'id': f't{i}', 'title': f'Tech {i}', 'status': s}
                          for i, s in enumerate(tech_states)],
        }
        self.save()

    def save(self):
        self.path.write_text(json.dumps(self.data), encoding='utf-8')

    def test_accounting_separates_progress_from_production(self):
        report = evaluate(self.root)
        self.assertEqual(report['total_stages'], 398)
        self.assertEqual(report['main_with_recorded_progress'], 8)
        self.assertEqual(report['technical_with_recorded_progress'], 13)
        self.assertEqual(report['total_with_recorded_progress'], 21)
        self.assertEqual(report['production_approved'], False)
        self.assertEqual(report['production_verification'], 'NOT_EVALUATED')
        self.assertEqual(report['evidence_level'], 'MANIFEST_STATUS_ONLY')
        self.assertIn('m7', report['main_progress_ids'])
        self.assertEqual(len(report['technical_progress_ids']), 13)
        self.assertEqual(report['main_status_counts']['PLANNED'], 234)
        self.assertEqual(report['technical_status_counts']['HISTORICAL_LAB_SCOPE_LIMITED'], 13)

    def test_digest_is_bytes_of_actual_input(self):
        report = evaluate(self.root)
        self.assertEqual(report['inventory_sha256'], hashlib.sha256(self.path.read_bytes()).hexdigest())

    def test_unknown_status_fails_closed(self):
        self.data['main'][0]['status'] = 'COMPLETE_VERIFIED'
        self.save()
        with self.assertRaisesRegex(ReportError, 'unclassified status'):
            evaluate(self.root)

    def test_duplicate_stage_id_fails_closed(self):
        self.data['technical'][2]['id'] = self.data['technical'][1]['id']
        self.save()
        with self.assertRaisesRegex(ReportError, 'duplicate stage id'):
            evaluate(self.root)

    def test_missing_stage_fails_closed(self):
        self.data['technical'].pop()
        self.save()
        with self.assertRaisesRegex(ReportError, 'count mismatch'):
            evaluate(self.root)

    def test_production_flag_cannot_be_promoted(self):
        self.data['production_approved'] = True
        self.save()
        with self.assertRaisesRegex(ReportError, 'production approval'):
            evaluate(self.root)

    def test_duplicate_json_keys_fail_closed(self):
        self.path.write_text('{"schema_version":1,"schema_version":1}', encoding='utf-8')
        with self.assertRaisesRegex(ReportError, 'duplicate JSON key'):
            evaluate(self.root)

    def test_symlink_manifest_fail_closed(self):
        target = self.root / 'elsewhere.json'
        self.path.rename(target)
        self.path.symlink_to(target)
        with self.assertRaisesRegex(ReportError, 'unsafe inventory'):
            evaluate(self.root)

    def test_wrong_repository_fails_closed(self):
        self.data['repository'] = 'attacker/other'
        self.save()
        with self.assertRaisesRegex(ReportError, 'repository identity'):
            evaluate(self.root)

    def test_empty_title_fails_closed(self):
        self.data['main'][0]['title'] = '  '
        self.save()
        with self.assertRaisesRegex(ReportError, 'invalid stage'):
            evaluate(self.root)

    def test_boolean_count_fails_closed(self):
        self.data['main_count'] = True
        self.save()
        with self.assertRaisesRegex(ReportError, 'count mismatch'):
            evaluate(self.root)

    def test_cli_produces_machine_readable_json(self):
        script = Path(__file__).resolve().parents[1] / 'roadmap_progress_report.py'
        p = subprocess.run([sys.executable, str(script), '--repo', str(self.root)],
                           capture_output=True, text=True, timeout=5)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(json.loads(p.stdout)['total_with_recorded_progress'], 21)


if __name__ == '__main__':
    unittest.main()
