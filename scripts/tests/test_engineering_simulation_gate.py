"""Real subprocess regression tests for OCAF operator-approved test gate."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts.engineering_simulation_gate import GateError, evaluate


class EngineeringSimulationGateTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.repo = Path(temp.name).resolve()
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.email', 'test@localhost')
        self.git('config', 'user.name', 'OCAF Fixture')
        (self.repo / 'scripts/tests').mkdir(parents=True)
        (self.repo / 'scripts/ocaf_branch_guardian.py').write_text('MARKER = 1\n')
        for suite in ['test_ocaf_branch_guardian.py', 'test_ocaf_branch_guardian_grafts.py']:
            (self.repo / 'scripts/tests' / suite).write_text(
                'import unittest\nclass TestOne(unittest.TestCase):\n'
                '    def test_positive(self):\n        self.assertEqual(2+2,4)\n'
            )
        self.commit()

    def git(self, *args):
        return subprocess.run(['git', '-C', str(self.repo), *args],
                              capture_output=True, text=True, check=True, timeout=10).stdout.strip()

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-qm', 'fixture')
        self.sha = self.git('rev-parse', 'HEAD')

    def test_pinned_clean_main_executes_hybrid_reference_suite(self):
        import shutil
        source = Path(__file__).resolve().parents[1]
        shutil.copy2(source / 'ocaf_hybrid_reference.py',
                     self.repo / 'scripts/ocaf_hybrid_reference.py')
        shutil.copy2(Path(__file__).resolve().parent / 'test_ocaf_hybrid_reference.py',
                     self.repo / 'scripts/tests/test_ocaf_hybrid_reference.py')
        self.commit()
        receipt = evaluate(self.repo, self.sha, 'SOURCE-1.65')
        self.assertEqual(receipt['scope'], 'bounded_hybrid_lexical_vector_reference_model')
        self.assertEqual(receipt['test_count'], 10)
        self.assertEqual(len(receipt['suite_results']), 1)
        self.assertFalse(receipt['production_approved'])
        self.assertFalse(receipt['full_stage_accepted'])

    def test_hybrid_reference_source_tampering_never_passes(self):
        import shutil
        source = Path(__file__).resolve().parents[1]
        shutil.copy2(source / 'ocaf_hybrid_reference.py',
                     self.repo / 'scripts/ocaf_hybrid_reference.py')
        shutil.copy2(Path(__file__).resolve().parent / 'test_ocaf_hybrid_reference.py',
                     self.repo / 'scripts/tests/test_ocaf_hybrid_reference.py')
        self.commit()
        (self.repo / 'scripts/ocaf_hybrid_reference.py').write_text('source was changed\n')
        with self.assertRaisesRegex(GateError, 'uncommitted'):
            evaluate(self.repo, self.sha, 'SOURCE-1.65')

    def test_both_benchmark_suites_execute_under_pinned_git_main(self):
        import shutil
        source = Path(__file__).resolve().parents[1]
        for name in ('ocaf_benchmark_harness.py', 'ocaf_hybrid_reference.py',
                     'ocaf_hybrid_benchmark.py'):
            shutil.copy2(source / name, self.repo / 'scripts' / name)
        for name in ('test_ocaf_benchmark_harness.py', 'test_ocaf_hybrid_benchmark.py'):
            shutil.copy2(Path(__file__).resolve().parent / name,
                         self.repo / 'scripts/tests' / name)
        self.commit()
        receipt = evaluate(self.repo, self.sha, 'SOURCE-2.05')
        self.assertEqual(receipt['scope'], 'bounded_python_sha256_and_hybrid_reference_benchmarks')
        self.assertEqual(receipt['test_count'], 18)
        self.assertEqual(len(receipt['suite_results']), 2)
        self.assertFalse(receipt['production_approved'])
        self.assertFalse(receipt['full_stage_accepted'])

    def test_pinned_clean_main_executes_both_real_suites(self):
        report = evaluate(self.repo, self.sha, 'SOURCE-2.06')
        self.assertEqual(report['status'], 'SCOPED_ENGINEERING_SIMULATION_PASS')
        self.assertEqual(report['test_count'], 2)
        self.assertEqual(len(report['suite_results']), 2)
        self.assertFalse(report['external_audit_required'])
        self.assertFalse(report['production_approved'])
        self.assertFalse(report['full_stage_accepted'])
        self.assertFalse(report['real_external_host_verified'])

    def test_wrong_commit_rejected(self):
        with self.assertRaisesRegex(GateError, 'mismatch'):
            evaluate(self.repo, 'a'*40, 'SOURCE-2.06')

    def test_invalid_commit_rejected(self):
        with self.assertRaisesRegex(GateError, '40 lowercase'):
            evaluate(self.repo, 'bad', 'SOURCE-2.06')

    def test_unregistered_stage_fails_closed(self):
        with self.assertRaisesRegex(GateError, 'no registered'):
            evaluate(self.repo, self.sha, 'SOURCE-1.55')

    def test_untracked_source_rejected(self):
        (self.repo / 'rogue').write_text('untracked\n')
        with self.assertRaisesRegex(GateError, 'uncommitted'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_dirty_tracked_source_rejected(self):
        (self.repo / 'scripts/ocaf_branch_guardian.py').write_text('MARKER = 999\n')
        with self.assertRaisesRegex(GateError, 'uncommitted'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_detached_head_rejected(self):
        self.git('checkout', '--detach', '-q')
        with self.assertRaisesRegex(GateError, 'git verification failed'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_failed_test_never_fakes_pass(self):
        p = self.repo / 'scripts/tests/test_ocaf_branch_guardian.py'
        p.write_text('import unittest\nclass Fail(unittest.TestCase):\n'
                     '    def test_failure(self):\n        self.assertEqual(2,3)\n')
        self.commit()
        with self.assertRaisesRegex(GateError, 'did not pass'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_zero_tests_is_refused(self):
        (self.repo / 'scripts/tests/test_ocaf_branch_guardian.py').write_text('import unittest\n')
        self.commit()
        with self.assertRaisesRegex(GateError, 'did not pass'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_symlinked_suite_refused(self):
        test = self.repo / 'scripts/tests/test_ocaf_branch_guardian.py'
        other = self.repo / 'test.txt'
        other.write_text('import unittest\n')
        test.unlink()
        test.symlink_to(other)
        self.commit()
        with self.assertRaisesRegex(GateError, 'unsafe source'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_bad_timeout_rejected(self):
        for timeout in [0, 121, True]:
            with self.subTest(timeout=timeout), self.assertRaisesRegex(GateError, 'timeout'):
                evaluate(self.repo, self.sha, 'SOURCE-2.06', timeout)

    def test_test_runner_mutation_blocks_receipt(self):
        p = self.repo / 'scripts/tests/test_ocaf_branch_guardian.py'
        p.write_text('import unittest\nfrom pathlib import Path\n'
                     'class Mutate(unittest.TestCase):\n'
                     '    def test_rewrite(self):\n'
                     '        Path("scripts/ocaf_branch_guardian.py").write_text("changed\\n")\n'
                     '        self.assertTrue(True)\n')
        self.commit()
        with self.assertRaisesRegex(GateError, 'checkout changed'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_assume_unchanged_forged_source_denied(self):
        self.git('update-index', '--assume-unchanged', 'scripts/ocaf_branch_guardian.py')
        (self.repo / 'scripts/ocaf_branch_guardian.py').write_text('MARKER = 999\n')
        self.assertEqual(self.git('status', '--porcelain', '--untracked-files=all'), '')
        with self.assertRaisesRegex(GateError, 'blob mismatch'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_skip_worktree_forged_test_denied(self):
        self.git('update-index', '--skip-worktree', 'scripts/tests/test_ocaf_branch_guardian.py')
        (self.repo / 'scripts/tests/test_ocaf_branch_guardian.py').write_text(
            'import unittest\nclass Trojan(unittest.TestCase):\n'
            '    def test_fake(self):\n        self.assertTrue(True)\n'
        )
        self.assertEqual(self.git('status', '--porcelain', '--untracked-files=all'), '')
        with self.assertRaisesRegex(GateError, 'blob mismatch'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')

    def test_branch_not_main_rejected(self):
        self.git('checkout', '-q', '-b', 'other')
        with self.assertRaisesRegex(GateError, 'mismatch'):
            evaluate(self.repo, self.sha, 'SOURCE-2.06')


if __name__ == '__main__':
    unittest.main()
