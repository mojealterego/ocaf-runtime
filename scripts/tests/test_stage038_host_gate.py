"""Stage 0.38 host evidence parser regression tests (no systemd mutations)."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "stage038_host_gate.py"
SPEC = importlib.util.spec_from_file_location("stage038_host_gate", SCRIPT)
assert SPEC and SPEC.loader
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)

PROPS = {
    "Id": "ocaf-coordinator-wasi.service",
    "LoadState": "loaded", "ActiveState": "active", "User": "ocaf",
    "NoNewPrivileges": "yes", "KillMode": "control-group",
    "MemoryMax": "1610612736", "TasksMax": "128",
    "CPUQuotaPerSecUSec": "2s", "PrivateTmp": "yes",
    "ProtectSystem": "strict",
    "ControlGroup": "/system.slice/ocaf-coordinator-wasi.service",
    "FragmentPath": "/etc/systemd/system/ocaf-coordinator-wasi.service",
}

class Stage038HostGateTests(unittest.TestCase):
    def test_parse_complete_safe_systemd_properties(self):
        record = gate.parse_unit_properties("\n".join(f"{k}={v}" for k, v in PROPS.items()))
        self.assertEqual(record, PROPS)

    def test_reject_duplicate_or_incomplete_properties(self):
        contents = "\n".join(f"{k}={v}" for k, v in PROPS.items())
        with self.assertRaises(ValueError):
            gate.parse_unit_properties(contents + "\nUser=root")
        with self.assertRaises(ValueError):
            gate.parse_unit_properties(contents.replace("MemoryMax=1610612736\n", ""))
        with self.assertRaises(ValueError):
            gate.parse_unit_properties(contents + "\nUnexpected=yes")

    def test_bounded_memory_tasks_and_cpu(self):
        self.assertTrue(gate.finite_positive_bound("128", 256))
        for value in ("infinity", "-1", "0", "100000000000000"):
            self.assertFalse(gate.finite_positive_bound(value, 256))
        self.assertTrue(gate.cpu_quota_bounded("2s"))
        for value in ("infinity", "4s", "nan", "0s", "-1s", "bogus"):
            self.assertFalse(gate.cpu_quota_bounded(value))

    def test_unit_name_is_limited_to_ocaf_service(self):
        self.assertTrue(gate.UNIT_NAME.fullmatch("ocaf-coordinator-wasi.service"))
        for value in ("ssh.service", "../../etc/passwd", "ocaf-wasi.service;reboot",
                      "ocaf-foo.socket", "ocaf-foo.service\nbar"):
            self.assertIsNone(gate.UNIT_NAME.fullmatch(value))

    def test_report_is_not_production_approval(self):
        with patch.dict(os.environ, {"OCAF_PRIVATE_SIGNING_KEY": "DO_NOT_PRINT_TOKEN"}):
            report = gate.build_report()
        self.assertFalse(report["production_approved"])
        self.assertFalse(report["stage_038_complete"])
        self.assertIn(report["status"], ("NO_GO", "HOST_PREREQUISITES_VERIFIED_ONLY"))
        self.assertNotIn("DO_NOT_PRINT_TOKEN", json.dumps(report))

    def test_report_is_create_only_and_private(self):
        with tempfile.TemporaryDirectory() as tmp:
            report = Path(tmp) / "evidence.json"
            with patch("sys.argv", ["stage038_host_gate.py", "--report", str(report)]):
                result = gate.main()
                self.assertIn(result, (0, 1))
                evidence = json.loads(report.read_text())
                self.assertEqual(evidence["schema_version"], 1)
                self.assertEqual(report.stat().st_mode & 0o777, 0o600)
                before = report.read_text()
                with self.assertRaises(SystemExit) as exc:
                    gate.main()
                self.assertEqual(exc.exception.code, 2)
                self.assertEqual(report.read_text(), before)

    def test_refuses_unsafe_unit_before_invoking_systemctl(self):
        with patch("subprocess.run") as call:
            with self.assertRaises(ValueError):
                gate.build_report("../../etc/passwd")
        call.assert_not_called()

    def test_cgroup_outside_root_rejected(self):
        with self.assertRaises(ValueError):
            gate.read_cgroup_stats(Path("/etc"))

    def test_actual_kernel_cgroup_limits_must_be_finite(self):
        good = {"memory.max": "1610612736", "pids.max": "128",
                "cpu.max": "200000 100000"}
        self.assertTrue(gate.kernel_limits_enforced(good))
        for name, value in (("memory.max", "max"),
                            ("memory.max", "3221225472"),
                            ("pids.max", "max"),
                            ("pids.max", "512"),
                            ("cpu.max", "max 100000"),
                            ("cpu.max", "300000 100000"),
                            ("cpu.max", "nonsense"),
                            ("cpu.max", "0 100000")):
            changed = {**good, name: value}
            self.assertFalse(gate.kernel_limits_enforced(changed))
        self.assertFalse(gate.kernel_limits_enforced({}))

if __name__ == "__main__":
    unittest.main()
