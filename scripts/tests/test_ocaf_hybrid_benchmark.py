"""Deterministic engineering tests for bounded SOURCE-2.05 hybrid workload."""
from __future__ import annotations
import json
import math
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from scripts.ocaf_hybrid_benchmark import BenchmarkError, build_workload, run_benchmark


class HybridBenchmarkTests(unittest.TestCase):
    def test_real_reference_workload_has_expected_top1(self):
        index, queries, fingerprint, payload_bytes = build_workload(32, 8, 5, 8)
        self.assertEqual(len(queries), 8)
        self.assertRegex(fingerprint, r'^[0-9a-f]{64}$')
        self.assertGreater(payload_bytes, 0)
        for query, embedding, relevant_id in queries:
            hits = index.search('benchmark_tenant', query, embedding, 5)
            self.assertTrue(hits)
            self.assertEqual(hits[0].source_id, relevant_id)

    def test_different_seed_changes_queries_but_corpus_digest_is_stable(self):
        a = build_workload(64, 8, 7, 16)
        b = build_workload(64, 8, 7, 16)
        c = build_workload(64, 8, 23, 16)
        self.assertEqual(a[1], b[1])
        self.assertEqual(a[2], b[2])
        self.assertEqual(a[2], c[2])
        self.assertNotEqual(a[1], c[1])

    def test_measured_report_is_scoped_not_rust_or_hardware(self):
        report = run_benchmark(documents=16, dimension=4, seed=3,
                               queries=4, samples=7, warmups=2)
        self.assertEqual(report['status'], 'MEASURED_LOCAL_PYTHON_REFERENCE_ONLY')
        self.assertEqual(report['measured_iterations'], 7)
        self.assertEqual(report['verified_top1_samples'], 7)
        self.assertFalse(report['production_approved'])
        self.assertFalse(report['native_rust_verified'])
        self.assertFalse(report['environment_attested'])
        self.assertTrue(report['cache_condition_unverified'])
        self.assertEqual(report['candidate_documents'], 16)
        self.assertLessEqual(report['p50_us'], report['p95_us'])
        self.assertLessEqual(report['p95_us'], report['p99_us'])
        self.assertLessEqual(report['p99_us'], report['max_us'])

    def test_timing_is_injected_and_nearest_rank_is_reused(self):
        ticks = iter([0, 1000, 1000, 3000, 3000, 9000])
        report = run_benchmark(documents=4, dimension=2, seed=1,
                               queries=2, samples=3, warmups=0,
                               clock=lambda: next(ticks))
        self.assertEqual(report['p50_us'], 2.0)
        self.assertEqual(report['p95_us'], 6.0)
        self.assertEqual(report['max_us'], 6.0)
        self.assertEqual(report['measured_iterations'], 3)

    def test_bounds_reject_invalid_requests_before_workload(self):
        invalid = [
            {'documents': 0}, {'documents': 257}, {'dimension': 0},
            {'dimension': 33}, {'samples': 0}, {'samples': 2001},
            {'warmups': 201}, {'seed': -1}, {'seed': 2**32},
            {'queries': 0}, {'queries': 33}, {'queries': 16, 'documents': 8},
            {'profile': 'invalid'}, {'profile': 'cold', 'warmups': 1},
            {'samples': True}, {'documents': 1.0},
        ]
        for overrides in invalid:
            with self.subTest(overrides=overrides), self.assertRaises(BenchmarkError):
                run_benchmark(**overrides)

    def test_cold_profile_does_not_assert_real_cache_flush(self):
        result = run_benchmark(documents=8, dimension=2, seed=0,
                               queries=2, samples=2, warmups=0, profile='cold')
        self.assertEqual(result['cache_condition'], 'cold')
        self.assertTrue(result['cache_condition_unverified'])
        self.assertFalse(result['environment_attested'])

    def test_invalid_monotonic_clock_never_claims_pass(self):
        ticks = iter([1200, 1000])
        with self.assertRaises(BenchmarkError):
            run_benchmark(documents=2, dimension=2, queries=1,
                          samples=1, warmups=0, clock=lambda: next(ticks))

    def test_full_256_document_corpus_is_supported(self):
        index, queries, fingerprint, payload_bytes = build_workload(256, 8, 13, 16)
        self.assertEqual(index.revision, 256)
        self.assertEqual(len(queries), 16)
        for query, embedding, relevant_id in queries:
            self.assertEqual(index.search('benchmark_tenant', query, embedding, 1)[0].source_id,
                             relevant_id)

    def test_cli_proves_git_provenance_without_approval_claims(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            for args in [
                ('init', '-q', '-b', 'main'),
                ('config', 'user.email', 'fixture@local'),
                ('config', 'user.name', 'Fixture'),
            ]:
                subprocess.run(['git', '-C', str(directory), *args], check=True,
                               capture_output=True, timeout=8)
            (directory / 'file.txt').write_text('fixture', encoding='utf-8')
            for args in [('add', 'file.txt'), ('commit', '-qm', 'fixture')]:
                subprocess.run(['git', '-C', str(directory), *args], check=True,
                               capture_output=True, timeout=8)
            script = Path(__file__).resolve().parents[1] / 'ocaf_hybrid_benchmark.py'
            p = subprocess.run([sys.executable, str(script), '--repo', str(directory),
                                '--documents', '8', '--dimension', '4', '--queries', '4',
                                '--samples', '4', '--warmups', '1'], capture_output=True,
                               text=True, timeout=15)
            self.assertEqual(p.returncode, 0, p.stderr)
            output = json.loads(p.stdout)
            self.assertEqual(len(output['environment']['git_commit_sha']), 40)
            self.assertFalse(output['environment']['environment_attested'])
            self.assertFalse(output['benchmark']['native_rust_verified'])
            self.assertFalse(output['benchmark']['production_approved'])


if __name__ == '__main__':
    unittest.main()
