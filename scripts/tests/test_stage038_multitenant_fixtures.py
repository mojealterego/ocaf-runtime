import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "stage038_multitenant_fixtures.py"


class TwoTenantFixturesTests(unittest.TestCase):
    def generate(self, root: Path, enabled: bool = True):
        env = dict(os.environ)
        env["GITHUB_ACTIONS"] = "true" if enabled else "false"
        return subprocess.run([sys.executable, str(SCRIPT), "--directory", str(root)],
                              env=env, capture_output=True, text=True, timeout=100, check=False)

    def test_signed_manifests_bind_different_tenants_and_exclusive_principals(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "fixtures"
            result = self.generate(root)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotEqual((root / "tenant-a" / "ca.pem").read_bytes(),
                                (root / "tenant-b" / "ca.pem").read_bytes())
            for tenant, agent, tool in (("tenant-a", "agent-a", "ocaf_ci_tenant_a"),
                                         ("tenant-b", "agent-b", "ocaf_ci_tenant_b")):
                base = root / tenant
                manifest = json.loads((base / "manifest.json").read_text())
                peers = json.loads((base / "peers.json").read_text())
                self.assertEqual(manifest["tenant_id"], tenant)
                self.assertEqual(manifest["agent_id"], agent)
                self.assertEqual(manifest["tool"]["name"], tool)
                self.assertEqual(manifest["module_sha256_hex"],
                                 hashlib.sha256((base / "module.wasm").read_bytes()).hexdigest())
                self.assertEqual(peers[agent]["allowed_tools"], [tool])
                other = "agent-b" if agent == "agent-a" else "agent-a"
                self.assertEqual(peers[other]["allowed_tools"], ["demo_echo"])
                verified = subprocess.run([
                    "openssl", "pkeyutl", "-verify", "-rawin", "-pubin",
                    "-inkey", str(base / "publisher.pem"),
                    "-in", str(base / "signing-input.bin"),
                    "-sigfile", str(base / "manifest.sig")
                ], capture_output=True, check=False, timeout=10)
                self.assertEqual(verified.returncode, 0)

    def test_refuses_non_ci_invocation(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = self.generate(Path(tmp) / "fixtures", enabled=False)
            self.assertNotEqual(result.returncode, 0)

    def test_refuses_existing_private_material(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "fixtures"
            root.mkdir()
            result = self.generate(root)
            self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
