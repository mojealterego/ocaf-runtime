"""Untrusted CI shard receipts: aggregate only validated real-engine evidence."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[1] / "stage038_evidence_aggregate.py"
SPEC = importlib.util.spec_from_file_location("stage038_evidence_aggregate", SOURCE)
assert SPEC is not None and SPEC.loader is not None
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)

COMMIT = "a" * 40
ENGINE = "b" * 64

def shard(index: int, cycles: int = 3) -> dict:
    return {
        "schema_version": 1,
        "acceptance": "Linux WASIp1 subprocess soak; not a security certification",
        "git_commit": COMMIT, "architecture": "x86_64",
        "engine_sha256": ENGINE, "kernel": "6.8.0",
        "started_unix_secs": 100, "completed_unix_secs": 115,
        "cycles_requested": cycles, "cycles_completed": cycles,
        "passed": True, "new_private_workspace_residue": [],
        "limits": {"max_rss_kib": 1572864, "max_fds": 512, "max_processes": 64},
        "cycles": [{"cycle": x, "returncode": 0, "timed_out": False,
                    "elapsed_sec": 2.5, "peak_observed_rss_kib": 1024,
                    "peak_observed_open_fds_per_process": 7,
                    "peak_observed_process_count": 2,
                    "samples": 50} for x in range(1, cycles + 1)]
    }

def make(root: Path, shards: int = 2) -> None:
    for i in range(1, shards + 1):
        (root / f"shard-{i}.json").write_text(json.dumps(shard(i)))

class ExtendedSoakEvidenceTests(unittest.TestCase):
    def test_accepts_exact_coherent_shards_without_certifying_release(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            make(root)
            result = module.aggregate(root, expected_commit=COMMIT,
                                      expected_shards=2, cycles_per_shard=3)
            self.assertEqual(result["cycles_total"], 6)
            self.assertEqual(result["engine_sha256"], ENGINE)
            self.assertFalse(result["stage_038_complete"])
            self.assertFalse(result["production_approved"])
            self.assertEqual(result["status"], "CI_EXTENDED_SOAK_PASS_NOT_STAGING_GO")

    def test_missing_or_extra_shard_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            make(root, 1)
            with self.assertRaises(ValueError):
                module.aggregate(root, COMMIT, 2, 3)
            make(root, 2)
            (root / "shard-3.json").write_text("{}")
            with self.assertRaises(ValueError):
                module.aggregate(root, COMMIT, 2, 3)

    def test_tampered_commit_digest_or_failed_result_is_rejected(self):
        for field, value in [
            ("git_commit", "other"), ("engine_sha256", "0"*64),
            ("architecture", "aarch64"), ("passed", False),
            ("new_private_workspace_residue", ["ocaf-wasi-leaked"]),
            ("cycles_completed", 2), ("cycles_requested", 999),
        ]:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as d:
                root = Path(d)
                make(root)
                obj = json.loads((root/"shard-2.json").read_text())
                obj[field] = value
                (root/"shard-2.json").write_text(json.dumps(obj))
                with self.assertRaises(ValueError):
                    module.aggregate(root, COMMIT, 2, 3)

    def test_cross_shard_engine_mismatch_refused(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            make(root)
            obj = shard(2)
            obj["engine_sha256"] = "c" * 64
            (root/"shard-2.json").write_text(json.dumps(obj))
            with self.assertRaises(ValueError):
                module.aggregate(root, COMMIT, 2, 3)

    def test_bad_cycle_data_refused(self):
        for change in [
            {"cycle": 1, "returncode": 1},
            {"cycle": 1, "timed_out": True},
            {"cycle": 1, "ceiling_violation": "rss"},
            {"cycle": 7},
            {"cycle": 1, "samples": 0},
        ]:
            with tempfile.TemporaryDirectory() as d:
                root = Path(d)
                make(root)
                obj = shard(2)
                obj["cycles"][0].update(change)
                (root/"shard-2.json").write_text(json.dumps(obj))
                with self.assertRaises(ValueError):
                    module.aggregate(root, COMMIT, 2, 3)

    def test_output_is_private_and_create_only(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            make(root)
            report = root / "summary.json"
            module.save_report(report, module.aggregate(root, COMMIT, 2, 3))
            self.assertEqual(report.stat().st_mode & 0o777, 0o600)
            data = report.read_bytes()
            with self.assertRaises(FileExistsError):
                module.save_report(report, {"tampered": True})
            self.assertEqual(report.read_bytes(), data)

    def test_bad_shard_limits_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            make(root)
            obj=shard(2)
            obj["limits"]={"max_rss_kib":0,"max_fds":0,"max_processes":0}
            (root/"shard-2.json").write_text(json.dumps(obj))
            with self.assertRaises(ValueError):
                module.aggregate(root, COMMIT, 2, 3)

if __name__ == "__main__":
    unittest.main()
