import hashlib
import io
import json
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from datetime import datetime, timedelta, timezone
from pathlib import Path

from scripts.release_readiness import (REQUIRED_EVIDENCE, REQUIRED_SCOPES, evaluate,
                                       apply_operator_override, main)

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
        self.workflows = self.repo / '.github' / 'workflows'
        self.workflows.mkdir(parents=True)
        subprocess.run(['git', '-C', str(self.repo), 'init', '-q'], check=True)
        subprocess.run(['git', '-C', str(self.repo), 'config', 'user.name', 'Release Test'], check=True)
        subprocess.run(['git', '-C', str(self.repo), 'config', 'user.email', 'release@example.invalid'], check=True)
        for name in REQUIRED_EVIDENCE:
            (self.workflows / (name + '.yml')).write_text(
                '# release-evidence-scope: ' + REQUIRED_SCOPES[name] + '\n'
                'name: ' + name + '\n'
                'on: workflow_dispatch\n', encoding='utf-8')

    def commit(self):
        subprocess.run(['git', '-C', str(self.repo), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.repo), 'commit', '-qm', 'test candidate'], check=True)

    def source_commit(self):
        return subprocess.check_output(['git', '-C', str(self.repo), 'rev-parse', 'HEAD'],
                                       text=True).strip()

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
        (self.repo / 'README.md').write_text('candidate source\n', encoding='utf-8')
        self.commit()

    def make_manifest(self, stale=False):
        checks = {}
        source_commit = self.source_commit()
        for index, name in enumerate(REQUIRED_EVIDENCE):
            path = self.evidence / (name + '.txt')
            path.write_text('Independent operator evidence for ' + name, encoding='utf-8')
            observed = NOW - (timedelta(days=60) if stale else timedelta(days=1))
            workflow = '.github/workflows/' + name + '.yml'
            scope = REQUIRED_SCOPES[name]
            if scope == 'independent_real_hosts':
                host_class, host_count = 'durable_real_host', 2
            elif scope == 'real_guest_host':
                host_class, host_count = 'durable_real_guest_host', 1
            elif scope == 'stateful_real_host':
                host_class, host_count = 'durable_real_host', 1
            else:
                host_class, host_count = '', 0
            checks[name] = {
                'path': path.name,
                'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                'observed_at_utc': observed.isoformat().replace('+00:00', 'Z'),
                'scope': scope,
                'source_commit': source_commit,
                'workflow': workflow,
                'workflow_sha256': hashlib.sha256((self.repo / workflow).read_bytes()).hexdigest(),
                'run_id': str(1000 + index),
                'run_attempt': 1,
                'artifact_id': 'artifact-' + name,
                'hosts': [
                    {'host_id': f'host-{index:02d}-{host_index:02d}',
                     'boot_id': f'00000000-0000-0000-0000-{index * 2 + host_index + 1:012x}',
                     'host_class': host_class}
                    for host_index in range(host_count)
                ],
            }
        manifest = self.evidence / 'manifest.json'
        manifest.write_text(json.dumps({'schema_version': 3, 'source_commit': source_commit,
                                        'checks': checks}), encoding='utf-8')
        return manifest

    def edit_manifest(self, manifest, edit):
        data = json.loads(manifest.read_text(encoding='utf-8'))
        edit(data)
        manifest.write_text(json.dumps(data), encoding='utf-8')

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
        for version in (1, 2):
            data['schema_version'] = version
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

    def test_manifest_and_each_check_are_bound_to_exact_source_commit(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: d.update(source_commit='a' * 40))
        self.assertIn('evidence_source_commit_mismatch',
                      evaluate(self.repo, manifest, NOW)['blockers'])
        manifest = self.make_manifest()
        name = REQUIRED_EVIDENCE[0]
        self.edit_manifest(manifest, lambda d: d['checks'][name].update(source_commit='a' * 40))
        self.assertIn('evidence_source_commit_mismatch:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])

    def test_workflow_must_be_tracked_at_source_and_unchanged(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        name = REQUIRED_EVIDENCE[0]
        self.edit_manifest(manifest, lambda d: d['checks'][name].update(
            workflow='.github/workflows/untracked.yml'))
        self.assertIn('evidence_workflow_untracked:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        manifest = self.make_manifest()
        (self.workflows / (name + '.yml')).write_text(
            '# release-evidence-scope: independent_real_hosts\nname: modified\n', encoding='utf-8')
        self.assertIn('evidence_workflow_untracked:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])

    def test_workflow_digest_and_declared_scope_must_match(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        name = REQUIRED_EVIDENCE[0]
        self.edit_manifest(manifest, lambda d: d['checks'][name].update(
            workflow_sha256='0' * 64))
        self.assertIn('evidence_workflow_hash_mismatch:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        (self.workflows / (name + '.yml')).write_text(
            '# release-evidence-scope: ephemeral_lab\nname: lab only\n', encoding='utf-8')
        self.commit()
        manifest = self.make_manifest()
        self.assertIn('evidence_workflow_scope_invalid:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        (self.workflows / (name + '.yml')).write_text(
            'name: misplaced declaration\n'
            '# release-evidence-scope: independent_real_hosts\n',
            encoding='utf-8')
        self.commit()
        manifest = self.make_manifest()
        self.assertIn('evidence_workflow_scope_invalid:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])

    def test_lab_and_reused_host_identities_cannot_satisfy_real_host_check(self):
        self.write_dependency_lock()
        name = 'multi_host_mtls_failover'
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: d['checks'][name]['hosts'][0].update(
            host_class='ephemeral_ci'))
        self.assertIn('evidence_host_identity_invalid:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: d['checks'][name]['hosts'][1].update(
            boot_id=d['checks'][name]['hosts'][0]['boot_id']))
        self.assertIn('evidence_host_identity_invalid:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: d['checks'][name].update(
            hosts=d['checks'][name]['hosts'][:1]))
        self.assertIn('evidence_host_identity_invalid:' + name,
                      evaluate(self.repo, manifest, NOW)['blockers'])

    def test_run_and_artifact_identity_are_required_and_unique(self):
        self.write_dependency_lock()
        first, second = REQUIRED_EVIDENCE[:2]
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: d['checks'][first].update(run_id='0001'))
        self.assertIn('evidence_run_invalid:' + first,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: d['checks'][first].update(artifact_id=''))
        self.assertIn('evidence_artifact_id_invalid:' + first,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        manifest = self.make_manifest()
        def reuse_identity(data):
            for field in ('workflow', 'workflow_sha256', 'run_id', 'run_attempt', 'artifact_id'):
                data['checks'][second][field] = data['checks'][first][field]
        self.edit_manifest(manifest, reuse_identity)
        self.assertIn('evidence_reused:' + second,
                      evaluate(self.repo, manifest, NOW)['blockers'])
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: d['checks'][second].update(
            artifact_id=d['checks'][first]['artifact_id']))
        self.assertIn('evidence_reused:' + second,
                      evaluate(self.repo, manifest, NOW)['blockers'])

    def test_uncommitted_lockfile_is_not_release_source(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        with (self.repo / 'Cargo.lock').open('a', encoding='utf-8') as handle:
            handle.write('\n# local edit\n')
        blockers = evaluate(self.repo, manifest, NOW)['blockers']
        self.assertIn('cargo_lock_uncommitted', blockers)
        self.assertIn('source_tree_uncommitted', blockers)

    def test_uncommitted_source_change_blocks_complete_inventory(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        (self.repo / 'README.md').write_text('changed candidate source\n', encoding='utf-8')
        blockers = evaluate(self.repo, manifest, NOW)['blockers']
        self.assertEqual(blockers, ['source_tree_uncommitted'])

    def test_production_gate_refuses_even_complete_offline_inventory(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        self.edit_manifest(manifest, lambda d: [item.update(
            observed_at_utc=datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'))
            for item in d['checks'].values()])
        output = io.StringIO()
        with redirect_stdout(output):
            code = main(['--repo', str(self.repo), '--evidence', str(manifest),
                         '--require-production-approval'])
        self.assertEqual(code, 2)
        report = json.loads(output.getvalue())
        self.assertEqual(report['status'], 'EVIDENCE_COMPLETE_UNVERIFIED')
        self.assertFalse(report['production_approved'])

    def test_operator_exception_cannot_pass_production_approval_gate(self):
        output = io.StringIO()
        with redirect_stdout(output):
            code = main(['--repo', str(self.repo),
                         '--operator-override-policy', str(self.operator_policy()),
                         '--ack-unverified-production-risk',
                         '--require-production-approval'])
        self.assertEqual(code, 2)
        report = json.loads(output.getvalue())
        self.assertTrue(report['administrative_release_unblocked'])
        self.assertFalse(report['production_approved'])

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
