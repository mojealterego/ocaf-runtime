"""Offline, no-network tests for sanitised OpenShift Sandbox audit."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("stage038_redhat_audit", ROOT / "scripts/openshift_sandbox_audit.py")
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)

class TestOpenShiftSandboxAudit(unittest.TestCase):
    def test_namespace_fixed_to_operator_link(self):
        self.assertEqual(module.NAMESPACE, "mojealterego-dev")

    def test_untrusted_stderr_never_returned(self):
        class Result:
            stdout = ""
            stderr = "Bearer sensitive_do_not_log"
            returncode = 1
        with patch.object(module.subprocess, "run", return_value=Result()) as mock:
            success, text = module.command(["-n", module.NAMESPACE, "get", "pods", "-o", "json"])
            self.assertFalse(success)
            self.assertEqual(text, "")
            self.assertIn("-n", mock.call_args.args[0])

    def test_virtual_machine_count_without_names(self):
        fixture = {"items": [
            {"metadata": {"name": "private-vm-name"}, "status": {"printableStatus": "Running"}},
            {"metadata": {"name": "secret-other-vm"}, "status": {"printableStatus": "Stopped"}},
        ]}
        with patch.object(module, "command", return_value=(True, json.dumps(fixture))):
            result = module.inventory("virtualmachines.kubevirt.io")
        self.assertEqual(result, {"accessible": True, "count": 2, "running": 1})
        self.assertNotIn("private-vm-name", json.dumps(result))

    def test_rejects_unexpected_json_and_authorization(self):
        with patch.object(module, "command", return_value=(True, json.dumps({"items": "bad"}))):
            self.assertFalse(module.inventory("pods")["accessible"])
        with patch.object(module, "command", return_value=(True, "superuser")):
            self.assertEqual(module.authorization("create", "virtualmachines.kubevirt.io"), "unavailable")

if __name__ == "__main__":
    unittest.main()
