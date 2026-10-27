import hashlib
import json
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

from scripts.release_readiness import REQUIRED_EVIDENCE, REQUIRED_SCOPES, evaluate, apply_operator_override

NOW = datetime(2026, 10, 13, 12, 0, tzinfo=timezone.utc)


class ReleaseReadinessTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.evidence = self.root / 'private-evidence'
        self.evidence.mkdir()

    def write_dependency_lock(self):
        (self.repo / 'Cargo.lock').write_text('''version = 4
[[package]]
name = "ocaf_runtime"
version = "0.26.0"
[[package]]
name = "serde"
version = "1.0.200"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0123456789abcdef" * 4
'''.replace('"0123456789abcdef" * 4', '"' + '0123456789abcdef' * 4 + '"'), encoding='utf-8')

    def make_manifest(self, stale=False):
        checks = {}
        for name in REQUIRED_EVIDENCE:
            path = self.evidence / (name + '.txt')
            path.write_text('Independent operator evidence for ' + name, encoding='utf-8')
            observed = NOW - (timedelta(days=60) if stale else timedelta(days=1))
            checks[name] = {
                'path': path.name,
                'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                'observed_at_utc': observed.isoformat().replace('+00:00', 'Z'),
                'scope': REQUIRED_SCOPES[name],
            }
        manifest = self.evidence / 'manifest.json'
        manifest.write_text(json.dumps({'schema_version': 2, 'checks': checks}), encoding='utf-8')
        return manifest

    def test_missing_evidence_and_lockfile_fail_closed(self):
        report = evaluate(self.repo, None, NOW)
        self.assertEqual(report['status'], 'NO_GO')
        self.assertIn('cargo_lock_missing', report['blockers'])
        self.assertIn('evidence_manifest_missing', report['blockers'])

    def test_fresh_complete_evidence_is_not_a_production_certification(self):
        self.write_dependency_lock()
        report = evaluate(self.repo, self.make_manifest(), NOW)
        self.assertEqual(report['status'], 'EVIDENCE_COMPLETE_UNVERIFIED')
        self.assertEqual(report['blockers'], [])
        self.assertFalse(report['production_approved'])

    def test_tampered_artifact_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        (self.evidence / (REQUIRED_EVIDENCE[0] + '.txt')).write_text('tampered', encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertEqual(report['status'], 'NO_GO')
        self.assertIn('evidence_hash_mismatch:' + REQUIRED_EVIDENCE[0], report['blockers'])

    def test_stale_evidence_rejected(self):
        self.write_dependency_lock()
        report = evaluate(self.repo, self.make_manifest(stale=True), NOW)
        self.assertEqual(report['status'], 'NO_GO')
        self.assertTrue(any(x.startswith('evidence_stale:') for x in report['blockers']))

    def test_path_escape_rejected_even_when_hash_matches(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text(encoding='utf-8'))
        data['checks'][REQUIRED_EVIDENCE[0]]['path'] = '../escape.txt'
        (self.root / 'escape.txt').write_text('external', encoding='utf-8')
        data['checks'][REQUIRED_EVIDENCE[0]]['sha256'] = hashlib.sha256(b'external').hexdigest()
        manifest.write_text(json.dumps(data), encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_path_unsafe:' + REQUIRED_EVIDENCE[0], report['blockers'])

    def test_duplicate_manifest_keys_are_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        manifest.write_text('{"schema_version": 1, "schema_version": 1, "checks": {}}', encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_manifest_invalid', report['blockers'])

    def test_unknown_checks_are_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text(encoding='utf-8'))
        data['checks']['fake_test'] = data['checks'][REQUIRED_EVIDENCE[0]]
        manifest.write_text(json.dumps(data), encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_manifest_invalid', report['blockers'])

    def test_old_manifest_version_requires_explicit_migration(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text())
        data['schema_version'] = 1
        manifest.write_text(json.dumps(data), encoding='utf-8')
        self.assertIn('evidence_manifest_invalid', evaluate(self.repo, manifest, NOW)['blockers'])

    def test_lab_scope_cannot_satisfy_real_host_gate(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text())
        data['checks']['backup_restore_rehearsal']['scope'] = 'ephemeral_lab'
        manifest.write_text(json.dumps(data), encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_scope_invalid:backup_restore_rehearsal', report['blockers'])

    def test_ephemeral_cli_rollback_cannot_satisfy_stateful_host_gate(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text())
        data['checks']['stateful_release_rollback']['scope'] = 'ephemeral_lab'
        manifest.write_text(json.dumps(data), encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_scope_invalid:stateful_release_rollback', report['blockers'])

    def test_one_artifact_cannot_satisfy_multiple_checks(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text())
        first, second = REQUIRED_EVIDENCE[:2]
        data['checks'][second]['path'] = data['checks'][first]['path']
        data['checks'][second]['sha256'] = data['checks'][first]['sha256']
        manifest.write_text(json.dumps(data), encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_reused:' + second, report['blockers'])

    def test_symlink_evidence_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        filename = self.evidence / (REQUIRED_EVIDENCE[0] + '.txt')
        target = self.evidence / 'target.txt'
        target.write_bytes(filename.read_bytes())
        filename.unlink()
        filename.symlink_to(target)
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_path_unsafe:' + REQUIRED_EVIDENCE[0], report['blockers'])


    def operator_policy(self):
        dest = self.root / 'directive.json'
        policy = {
            "schema_version": 1,
            "git_branch": "main",
            "operator_directive_date": "2026-10-15",
            "operator_authorization_source": "explicit user instruction in ChatGPT conversation",
            "operator_decision": "UNBLOCK_PRODUCTION_DEPLOYMENT",
            "administrative_release_blocked": False,
            "stage_038_administrative_status": "CLOSED_BY_OPERATOR",
            "stage_039_development": "AUTHORIZED",
            "safety_acceptance_a01_a12": "INCOMPLETE",
            "third_party_audit_evidence": "INCOMPLETE",
            "verified_production_readiness": False,
            "production_deployed": False,
            "risk_waiver_notes": "Operator explicitly accepts administrative release authorization, without falsely claiming evidence completion."
        }
        dest.write_text(json.dumps(policy), encoding='utf-8')
        return dest

    def test_operator_explicit_unblock_preserves_technical_failures(self):
        raw = evaluate(self.repo, None, NOW)
        self.assertIn('evidence_manifest_missing', raw['blockers'])
        authorized = apply_operator_override(raw, self.operator_policy(), acknowledged=True)
        self.assertTrue(authorized['administrative_release_unblocked'])
        self.assertEqual(authorized['status'], 'OPERATOR_AUTHORIZED_UNVERIFIED')
        self.assertFalse(authorized['production_approved'])
        self.assertFalse(authorized['technical_evidence_passed'])
        self.assertIn('evidence_manifest_missing', authorized['blockers'])

    def test_operator_unblock_requires_own_explicit_acknowledgement(self):
        raw = evaluate(self.repo, None, NOW)
        with self.assertRaises(ValueError):
            apply_operator_override(raw, self.operator_policy(), acknowledged=False)

    def test_operator_unblock_rejects_malformed_or_unsupported_policy(self):
        raw = evaluate(self.repo, None, NOW)
        target = self.operator_policy()
        for change in ({'operator_decision': 'BLOCKED'},
                       {'verified_production_readiness': True},
                       {'production_deployed': True},
                       {'administrative_release_blocked': True},
                       {'extra_unknown': 1}):
            with self.subTest(change=change):
                data = json.loads(target.read_text())
                data.update(change)
                target.write_text(json.dumps(data), encoding='utf-8')
                with self.assertRaises(ValueError):
                    apply_operator_override(raw, target, acknowledged=True)
                target = self.operator_policy()

    def test_operator_unblock_does_not_accept_policy_symlink(self):
        raw = evaluate(self.repo, None, NOW)
        target = self.operator_policy()
        aliased = self.root / 'aliased-policy.json'
        aliased.symlink_to(target)
        with self.assertRaises(ValueError):
            apply_operator_override(raw, aliased, acknowledged=True)


if __name__ == '__main__':
    unittest.main()
