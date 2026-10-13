"""Unit tests for the IBM Cloud host readiness checker (no cloud account)."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "ibm_cloud_wasi_preflight.py"
SPEC = importlib.util.spec_from_file_location("ibm_cloud_wasi_preflight", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class IBMCloudPreflightTests(unittest.TestCase):
    def test_cgroup_v2_requires_all_resource_controllers(self):
        self.assertTrue(module.controllers_ready("cpu cpuset io memory pids"))
        self.assertFalse(module.controllers_ready("cpu cpuset io pids"))
        self.assertFalse(module.controllers_ready("memory pids"))
        self.assertFalse(module.controllers_ready(""))

    def test_meminfo_requires_valid_number(self):
        self.assertEqual(module.memory_kib("MemTotal: 8192 kB\nMemFree: 100 kB"), 8192)
        self.assertIsNone(module.memory_kib("MemTotal: invalid kB"))
        self.assertIsNone(module.memory_kib("MemFree: 1024 kB"))

    def test_report_does_not_contain_iam_secrets(self):
        with patch.dict(os.environ, {
            "IBMCLOUD_API_KEY": "DO_NOT_LEAK_KEY",
            "IBM_CLOUD_API_KEY": "DO_NOT_LEAK_KEY",
        }):
            report = module.build_report()
        self.assertIn("checks", report)
        self.assertIn("external_gates", report)
        self.assertNotIn("DO_NOT_LEAK_KEY", str(report))
        self.assertNotIn("api_key", str(report).lower())

    def test_create_only_report_never_overwrites(self):
        with tempfile.TemporaryDirectory() as work:
            file = Path(work) / "readiness.json"
            with patch("sys.argv", ["ibm_cloud_wasi_preflight.py", "--report", str(file)]):
                module.main()
                first = file.read_text()
                with self.assertRaises(FileExistsError):
                    module.main()
                self.assertEqual(first, file.read_text())
                self.assertEqual(file.stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
