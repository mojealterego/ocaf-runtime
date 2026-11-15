"""Regression: Git grafts must not spoof the ancestor check."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts.ocaf_branch_guardian import evaluate


class GitGraftsRegression(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name) / 'checkout'
        self.repo.mkdir()
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.name', 'OCAF Test')
        self.git('config', 'user.email', 'test@example.invalid')
        (self.repo / 'initial.txt').write_text('initial\n')
        self.git('add', '.')
        self.git('commit', '-qm', 'initial')
        self.baseline_main = self.output('rev-parse', 'HEAD')
        self.git('branch', 'historical/first')
        self.baseline = Path(self.temp.name) / 'operator-pinned.json'
        self.baseline.write_text(json.dumps({
            'schema_version': 1,
            'repository': 'mojealterego/ocaf-runtime',
            'baseline_main': self.baseline_main,
            'historical_refs': {'refs/heads/historical/first': self.baseline_main},
        }))
        self.pin = hashlib.sha256(self.baseline.read_bytes()).hexdigest()

    def git(self, *args):
        subprocess.run(['git', '-C', str(self.repo), *args], check=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)

    def output(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], text=True).strip()

    def grafts_path(self):
        path = Path(self.output('rev-parse', '--git-path', 'info/grafts'))
        return path if path.is_absolute() else self.repo / path

    def test_unrelated_rewritten_main_cannot_be_hidden_with_graft(self):
        self.git('checkout', '-q', '--orphan', 'rewritten')
        (self.repo / 'initial.txt').unlink()
        (self.repo / 'replacement.txt').write_text('replacement\n')
        self.git('add', '-A')
        self.git('commit', '-qm', 'replacement')
        current = self.output('rev-parse', 'HEAD')
        self.git('checkout', '-q', '-B', 'main')
        self.grafts_path().write_text(f'{current} {self.baseline_main}\n')
        # Git itself is fooled by the graft, but our guardian must not be.
        self.git('merge-base', '--is-ancestor', self.baseline_main, current)
        result = evaluate(self.repo, self.baseline, self.pin)
        self.assertEqual(result['status'], 'NO_GO')
        self.assertIn('git_grafts_detected', result['blockers'])

    def test_broken_grafts_symlink_is_denied(self):
        self.grafts_path().symlink_to('nonexistent-grafts')
        result = evaluate(self.repo, self.baseline, self.pin)
        self.assertIn('git_grafts_detected', result['blockers'])

    def test_linked_worktree_uses_common_git_dir_for_grafts(self):
        self.git('checkout', '--detach', '-q')
        linked = Path(self.temp.name) / 'linked'
        self.git('worktree', 'add', '-q', str(linked), 'main')
        self.assertEqual(
            evaluate(linked, self.baseline, self.pin)['status'],
            'LOCAL_REFS_PRESERVED_UNATTESTED',
        )
        self.grafts_path().write_text('')
        result = evaluate(linked, self.baseline, self.pin)
        self.assertIn('git_grafts_detected', result['blockers'])

    def test_unverifiable_grafts_path_fails_closed(self):
        with patch('scripts.ocaf_branch_guardian._grafts_state', return_value=None):
            result = evaluate(self.repo, self.baseline, self.pin)
        self.assertIn('git_grafts_path_unverifiable', result['blockers'])

    def test_grafts_created_during_evaluation_fails_closed(self):
        with patch('scripts.ocaf_branch_guardian._grafts_state', side_effect=[False, True]):
            result = evaluate(self.repo, self.baseline, self.pin)
        self.assertIn('git_grafts_detected', result['blockers'])

    def test_intact_repository_without_grafts_still_passes(self):
        result = evaluate(self.repo, self.baseline, self.pin)
        self.assertEqual(result['status'], 'LOCAL_REFS_PRESERVED_UNATTESTED')
