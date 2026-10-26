"""The stage 0.44 acceptance gates must remain active under `python -O`."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parents[1]


class OptimizedAcceptanceTests(unittest.TestCase):
    def run_optimized(self, code, settings):
        with tempfile.TemporaryDirectory() as directory:
            token = Path(directory) / "token"
            token.write_text("test-token")
            token.chmod(0o600)
            env = {**os.environ, "PYTHONPATH": str(SCRIPTS), **settings}
            for name in ("OCAF_E2E_TOKEN_FILE", "OCAF_VAULT_TOKEN_FILE"):
                if name in settings:
                    env[name] = str(token)
            result = subprocess.run(
                [sys.executable, "-O", "-c", code], env=env,
                capture_output=True, text=True, timeout=10, check=False,
            )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn('"PASS"', result.stdout)

    def test_oauth_rejects_bad_issuer_under_optimized_python(self):
        code = """
import os
from unittest.mock import patch
import stage044_network_acceptance as probe
with patch.object(probe, 'fetch', return_value=(500, b'{}')):
    try:
        probe.main()
    except RuntimeError:
        raise SystemExit(0)
raise SystemExit(1)
"""
        self.run_optimized(code, {
            "OCAF_E2E_ISSUER": "https://issuer.example",
            "OCAF_E2E_GATEWAY": "https://gateway.example",
            "OCAF_E2E_TENANT": "tenant", "OCAF_E2E_USER": "user",
            "OCAF_E2E_RESOURCE": "https://omat.example",
            "OCAF_E2E_TOKEN_FILE": "private test file",
        })

    def test_vault_rejects_wrong_plaintext_under_optimized_python(self):
        code = """
import base64
import json
from unittest.mock import patch
import stage044_vault_acceptance as probe
class Response:
    def __init__(self, status, body):
        self.status = status
        self.body = json.dumps(body).encode()
    def __enter__(self): return self
    def __exit__(self, *args): return False
    def read(self, maximum): return self.body
class Opener:
    def open(self, request, timeout):
        operation = request.full_url.split('/v1/transit/')[1].split('/')[0]
        context = base64.b64decode(json.loads(request.data)['context'])
        if b'wrong-tenant' in context:
            return Response(400, {'errors':['context denied']})
        if operation == 'decrypt':
            return Response(200, {'data':{'plaintext':base64.b64encode(b'wrong').decode()}})
        return Response(200, {'data':{'ciphertext':'vault:v1:fake'}})
with patch.object(probe, 'guarded_opener', return_value=Opener()):
    try:
        probe.main()
    except RuntimeError:
        raise SystemExit(0)
raise SystemExit(1)
"""
        self.run_optimized(code, {
            "OCAF_VAULT_ADDR": "https://vault.example", "OCAF_VAULT_KEY": "key",
            "OCAF_E2E_TENANT": "tenant", "OCAF_E2E_USER": "user",
            "OCAF_E2E_PROVIDER": "provider", "OCAF_VAULT_TOKEN_FILE": "ignored",
        })


if __name__ == "__main__":
    unittest.main()
