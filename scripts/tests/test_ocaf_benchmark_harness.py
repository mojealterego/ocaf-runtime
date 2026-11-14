"""Small deterministic tests: no performance or production claims are inferred."""
import unittest
from scripts.ocaf_benchmark_harness import (
    BenchmarkConfig, BenchmarkError, measure, nearest_rank, validate_config
)


class BenchmarkHarnessTests(unittest.TestCase):
    def config(self, **overrides):
        fields = dict(
            name='sha256_bytes', profile='warm', warmups=1, samples=3,
            payload_bytes=1024
        )
        fields.update(overrides)
        return BenchmarkConfig(**fields)

    def test_nearest_rank_percentiles(self):
        self.assertEqual(nearest_rank([1000, 4000, 5000], 50), 4000)
        self.assertEqual(nearest_rank([1000, 4000, 5000], 95), 5000)
        self.assertEqual(nearest_rank([1000, 4000, 5000], 99), 5000)

    def test_repeatable_clock_and_warmup_is_not_measured(self):
        ticks = iter([0, 1000, 2000, 6000, 7000, 12000])
        visited = []
        report = measure(self.config(), lambda n: visited.append(n), clock=lambda: next(ticks))
        self.assertEqual(visited, [0, 0, 1, 2])
        self.assertEqual(report['measured_iterations'], 3)
        self.assertEqual(report['warmup_iterations'], 1)
        self.assertEqual(report['mean_us'], 3.333)
        self.assertEqual(report['p50_us'], 4.0)
        self.assertEqual(report['p95_us'], 5.0)
        self.assertEqual(report['p99_us'], 5.0)
        self.assertEqual(report['max_us'], 5.0)
        self.assertEqual(report['status'], 'MEASURED_LOCAL_PYTHON_ONLY')
        self.assertFalse(report['production_approved'])
        self.assertTrue(report['cache_condition_unverified'])

    def test_zero_samples_rejected(self):
        with self.assertRaises(BenchmarkError):
            validate_config(self.config(samples=0))

    def test_unbounded_iterations_rejected(self):
        with self.assertRaises(BenchmarkError):
            validate_config(self.config(samples=10001))

    def test_cold_profile_never_mislabels_warmup_as_cold(self):
        with self.assertRaises(BenchmarkError):
            validate_config(self.config(profile='cold', warmups=10))

    def test_invalid_name_and_payload_limits_rejected(self):
        for config in [
            self.config(name='../bad'),
            self.config(payload_bytes=0),
            self.config(payload_bytes=1048577),
            self.config(profile='fake'),
        ]:
            with self.subTest(config=config), self.assertRaises(BenchmarkError):
                validate_config(config)

    def test_time_regression_raises_without_fake_metrics(self):
        ticks = iter([2000, 1000])
        with self.assertRaises(BenchmarkError):
            measure(
                self.config(samples=1, warmups=0), lambda _: None,
                clock=lambda: next(ticks)
            )

    def test_percentile_without_samples_fail_closed(self):
        with self.assertRaises(BenchmarkError):
            nearest_rank([], 99)

    def test_report_does_not_claim_rust_or_hardware_approval(self):
        ticks = iter([0, 1000])
        report = measure(
            self.config(warmups=0, samples=1), lambda _: None,
            clock=lambda: next(ticks)
        )
        self.assertIn('not_rust', report['limitations'])
        self.assertFalse(report['production_approved'])


if __name__ == '__main__':
    unittest.main()
