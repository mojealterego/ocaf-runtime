"""Git-only local fixtures; tests never modify a production repository."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts.ocaf_branch_guardian import evaluate


class BranchGuardianTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / 'checkout'
        self.repo.mkdir()
        self.run_git('init', '-q', '-b', 'main')
        self.run_git('config', 'user.name', 'Branch Guard Test')
        self.run_git('config', 'user.email', 'tests@example.invalid')
        (self.repo / 'hello.txt').write_text('initial\n', encoding='utf-8')
        self.run_git('add', '.')
        self.run_git('commit', '-qm', 'initial')
        self.baseline_main = self.git_output('rev-parse', 'HEAD')
        self.run_git('branch', 'historical/first')
        self.baseline = self.root / 'operator-pinned.json'
        self.manifest = {
            'schema_version': 1,
            'repository': 'mojealterego/ocaf-runtime',
            'baseline_main': self.baseline_main,
            'historical_refs': {'refs/heads/historical/first': self.baseline_main},
        }
        self.write_manifest()

    def run_git(self, *args):
        subprocess.run(['git', '-C', str(self.repo), *args], check=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)

    def git_output(self, *args):
        return subprocess.check_output(
            ['git', '-C', str(self.repo), *args], text=True, timeout=5
        ).strip()

    def write_manifest(self):
        self.baseline.write_text(json.dumps(self.manifest, sort_keys=True), encoding='utf-8')
        self.pin = hashlib.sha256(self.baseline.read_bytes()).hexdigest()

    def check(self, pinned=None):
        return evaluate(self.repo, self.baseline, pinned if pinned is not None else self.pin)

    def test_intact_refs_pass_as_local_observation_not_production(self):
        result = self.check()
        self.assertEqual(result['status'], 'LOCAL_REFS_PRESERVED_UNATTESTED')
        self.assertEqual(result['historical_refs_checked'], 1)
        self.assertFalse(result['production_approved'])
        self.assertFalse(result['remote_refs_verified'])
        self.assertFalse(result['workflow_safety_verified'])
        self.assertEqual(result['blockers'], [])

    def test_fast_forward_on_main_is_allowed(self):
        (self.repo / 'hello.txt').write_text('second\n', encoding='utf-8')
        self.run_git('commit', '-qam', 'fast forward')
        self.assertEqual(self.check()['status'], 'LOCAL_REFS_PRESERVED_UNATTESTED')

    def test_wrong_external_sha256_pin_is_denied(self):
        result = self.check('0' * 64)
        self.assertEqual(result['status'], 'NO_GO')
        self.assertIn('baseline_pin_mismatch', result['blockers'])

    def test_modifying_manifest_after_pinning_is_denied(self):
        pinned = self.pin
        self.manifest['repository'] = 'other/other'
        self.write_manifest()
        self.assertIn('baseline_pin_mismatch', self.check(pinned)['blockers'])

    def test_deleted_historical_ref_is_denied(self):
        self.run_git('branch', '-D', 'historical/first')
        self.assertIn('missing_historical_ref:refs/heads/historical/first', self.check()['blockers'])

    def test_rewritten_historical_ref_is_denied(self):
        (self.repo / 'hello.txt').write_text('new\n', encoding='utf-8')
        self.run_git('commit', '-qam', 'new')
        self.run_git('branch', '-f', 'historical/first', 'HEAD')
        self.assertIn('historical_ref_changed:refs/heads/historical/first', self.check()['blockers'])

    def test_non_fast_forward_main_rewrite_is_denied(self):
        self.run_git('checkout', '-q', '--orphan', 'rewritten')
        (self.repo / 'hello.txt').write_text('unrelated\n', encoding='utf-8')
        self.run_git('add', '.')
        self.run_git('commit', '-qm', 'unrelated')
        self.run_git('checkout', '-q', '-B', 'main')
        self.assertIn('main_not_descendant_of_baseline', self.check()['blockers'])

    def test_detached_head_is_denied(self):
        self.run_git('checkout', '--detach', '-q')
        self.assertIn('not_on_main', self.check()['blockers'])

    def test_duplicate_json_keys_fail_closed(self):
        self.baseline.write_text('{"schema_version":1,"schema_version":1}', encoding='utf-8')
        pin = hashlib.sha256(self.baseline.read_bytes()).hexdigest()
        self.assertIn('baseline_schema_invalid', self.check(pin)['blockers'])

    def test_symlink_manifest_is_denied(self):
        alt = self.root / 'valid.json'
        self.baseline.rename(alt)
        self.baseline.symlink_to(alt.name)
        self.assertIn('baseline_invalid', self.check()['blockers'])

    def test_unsafe_historical_ref_name_is_denied(self):
        self.manifest['historical_refs'] = {'refs/heads/../../main': self.baseline_main}
        self.write_manifest()
        self.assertIn('baseline_schema_invalid', self.check()['blockers'])

    def test_duplicate_main_is_not_a_historical_ref(self):
        self.manifest['historical_refs'] = {'refs/heads/main': self.baseline_main}
        self.write_manifest()
        self.assertIn('baseline_schema_invalid', self.check()['blockers'])

    def test_zero_historical_refs_can_be_explicit_without_fabricated_count(self):
        self.manifest['historical_refs'] = {}
        self.write_manifest()
        result = self.check()
        self.assertEqual(result['historical_refs_checked'], 0)
        self.assertEqual(result['status'], 'LOCAL_REFS_PRESERVED_UNATTESTED')

    def test_repository_identity_must_match_checked_project(self):
        self.manifest['repository'] = 'other/different'
        self.write_manifest()
        self.assertIn('baseline_schema_invalid', self.check()['blockers'])

    def test_git_dir_environment_cannot_redirect_to_foreign_repo(self):
        with patch.dict(os.environ, {'GIT_DIR': str(self.root), 'GIT_WORK_TREE': str(self.root)}):
            self.assertEqual(self.check()['status'], 'LOCAL_REFS_PRESERVED_UNATTESTED')

    def test_no_git_repository_is_denied(self):
        self.assertIn('not_a_git_checkout', evaluate(self.root, self.baseline, self.pin)['blockers'])

    def test_invalid_pinned_digest_is_denied(self):
        self.assertIn('invalid_trusted_pin', self.check('000bad')['blockers'])

    def test_untrusted_tag_as_ref_sha_is_denied_by_schema(self):
        self.manifest['historical_refs'] = {'refs/heads/historical/first': 'not-a-sha'}
        self.write_manifest()
        self.assertIn('baseline_schema_invalid', self.check()['blockers'])


if __name__ == '__main__':
    unittest.main()
