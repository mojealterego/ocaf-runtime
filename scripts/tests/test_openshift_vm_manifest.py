"""Validate the OpenShift admission candidate has strict, non-running bounds."""
import importlib.util
import unittest
from pathlib import Path

root = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ocaf038_vm_manifest", root / "scripts/openshift_vm_manifest.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class TestVMManifest(unittest.TestCase):
    def test_immutable_target_and_stopped_state(self):
        v = mod.build_manifest()
        self.assertEqual(v["metadata"]["namespace"], "mojealterego-dev")
        self.assertEqual(v["metadata"]["name"], "ocaf038-preflight")
        self.assertEqual(v["spec"]["runStrategy"], "Halted")
        self.assertEqual(v["kind"], "VirtualMachine")

    def test_resource_limits_and_official_boot_source(self):
        v = mod.build_manifest()
        cpu = v["spec"]["template"]["spec"]["domain"]["resources"]
        self.assertEqual(cpu["requests"], {"cpu": "1", "memory": "2Gi"})
        self.assertEqual(cpu["limits"], {"cpu": "2", "memory": "3Gi"})
        img = v["spec"]["dataVolumeTemplates"][0]["spec"]
        self.assertEqual(img["sourceRef"], {"kind": "DataSource", "name": "fedora",
                                             "namespace": "openshift-virtualization-os-images"})
        self.assertEqual(img["storage"]["resources"]["requests"]["storage"], "30Gi")

    def test_no_credentials_and_no_public_routes(self):
        v = mod.build_manifest()
        import json
        text = json.dumps(v)
        for value in ("password:", "ssh_authorized_keys", "Secret", "Service", "Route"):
            self.assertNotIn(value, text)
        self.assertIn("ssh_pwauth: false", text)

if __name__ == "__main__":
    unittest.main()
