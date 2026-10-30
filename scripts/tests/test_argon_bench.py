import sys
import unittest

from scripts.argon_bench import BenchmarkError, benchmark, nearest_rank, verify_receipt


class ArgonBenchTests(unittest.TestCase):
    def test_percentiles_are_nearest_rank_not_interpolated(self):
        samples = [1.0, 2.0, 4.0, 3.0]
        self.assertEqual(nearest_rank(samples, 0.5), 2.0)
        self.assertEqual(nearest_rank(samples, 0.95), 4.0)
        self.assertEqual(nearest_rank(samples, 0.99), 4.0)

    def test_actual_subprocess_measurements_and_verify(self):
        values = benchmark([sys.executable, "-c", "pass"], warmups=1, samples=4, timeout_s=2)
        self.assertEqual(len(values), 4)
        self.assertTrue(all(isinstance(x, float) and x >= 0 for x in values))
        receipt = {
            "schema_version": 1,
            "commit_sha": "a" * 40,
            "clean_tree": True,
            "samples_ms": values,
            "p50_ms": nearest_rank(values, 0.50),
            "p95_ms": nearest_rank(values, 0.95),
            "p99_ms": nearest_rank(values, 0.99),
        }
        self.assertTrue(verify_receipt(receipt, expected_sha="a" * 40, max_p99_ms=10000))
        self.assertFalse(verify_receipt(receipt, expected_sha="b" * 40, max_p99_ms=10000))
        receipt["p99_ms"] = -1
        self.assertFalse(verify_receipt(receipt, expected_sha="a" * 40, max_p99_ms=10000))

    def test_nonzero_failure_never_produces_success(self):
        with self.assertRaises(BenchmarkError):
            benchmark([sys.executable, "-c", "raise RuntimeError('failed')"],
                      warmups=0, samples=1, timeout_s=2)

    def test_timeout_is_not_counted_as_measurement(self):
        with self.assertRaises(BenchmarkError):
            benchmark([sys.executable, "-c", "import time; time.sleep(1)"],
                      warmups=0, samples=1, timeout_s=0.01)

    def test_rejects_invalid_resource_limits(self):
        with self.assertRaises(BenchmarkError):
            benchmark([sys.executable, "-c", "pass"], warmups=0, samples=0, timeout_s=1)
        with self.assertRaises(BenchmarkError):
            benchmark([], warmups=0, samples=1, timeout_s=1)


if __name__ == "__main__":
    unittest.main()
