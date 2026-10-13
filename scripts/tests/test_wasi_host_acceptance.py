"""Offline unit checks for Linux WASI evidence collector, no cargo or network."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "wasi_host_acceptance.py"
SPEC = importlib.util.spec_from_file_location("wasi_host_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


@unittest.skipUnless(os.name == "posix" and Path("/proc/self/status").exists(), "Linux only")
class HostAcceptanceTests(unittest.TestCase):
    def test_sha256_records_exact_bytes(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "engine"
            p.write_bytes(b"approved")
            actual = module.file_sha256(p)
            self.assertEqual(len(actual), 64)
            p.write_bytes(b"tampered")
            self.assertNotEqual(actual, module.file_sha256(p))

    def test_process_census_includes_self(self):
        pids = module.descendants(os.getpid())
        self.assertIn(os.getpid(), pids)
        rss, descriptors, count = module.sample_tree(os.getpid())
        self.assertGreaterEqual(rss, 0)
        self.assertGreaterEqual(descriptors, 0)
        self.assertGreaterEqual(count, 1)

    def test_wrong_operator_sha_aborts_before_any_guest_execution(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "wasmtime"
            p.write_bytes(b"engine-placeholder")
            report = Path(d) / "should-not-exist.json"
            with patch.dict(os.environ, {
                "OCAF_TEST_WASMTIME": str(p),
                "OCAF_TEST_WASMTIME_SHA256": "0" * 64,
            }):
                with patch("sys.argv", [
                    "wasi_host_acceptance.py", "--cycles", "1", "--report", str(report),
                ]):
                    with self.assertRaises(SystemExit) as raised:
                        module.main()
            self.assertEqual(raised.exception.code, 2)
            self.assertFalse(report.exists())


if __name__ == "__main__":
    unittest.main()
