"""Real OpenSSL PKI regression tests for the OCAF operational mTLS gate."""
import importlib.util
import json
from pathlib import Path
import os
import subprocess
import sys
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / "mtls_lifecycle.py"
spec = importlib.util.spec_from_file_location("mtls_lifecycle", MODULE)
assert spec is not None and spec.loader is not None
mtls = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = mtls
spec.loader.exec_module(mtls)


class MtlsLifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.ca = self.root / "ca.pem"
        self.ca_key = self.root / "ca.key"
        self.generate_ca()
        self.server, self.server_key = self.generate_cert("server", "serverAuth", "DNS:node.example.test")
        self.client, self.client_key = self.generate_cert("client", "clientAuth", "DNS:agent.example.test")

    def openssl(self, *args):
        outcome = subprocess.run(["openssl", *map(str, args)], capture_output=True, timeout=15, check=False)
        self.assertEqual(outcome.returncode, 0, outcome.stderr.decode(errors="replace"))

    def generate_ca(self):
        self.openssl("req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                     "-nodes", "-keyout", self.ca_key, "-out", self.ca,
                     "-days", "2", "-subj", "/CN=OCAF Test Root",
                     "-addext", "basicConstraints=critical,CA:TRUE",
                     "-addext", "keyUsage=critical,keyCertSign,cRLSign")
        os.chmod(self.ca_key, 0o600)

    def generate_cert(self, name, eku, san):
        key = self.root / (name + ".key")
        csr = self.root / (name + ".csr")
        cert = self.root / (name + ".pem")
        extensions = self.root / (name + ".cnf")
        self.openssl("req", "-new", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                     "-nodes", "-keyout", key, "-out", csr, "-subj", f"/CN={name}")
        extensions.write_text("basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\n"
                              f"extendedKeyUsage={eku}\nsubjectAltName={san}\n", encoding="ascii")
        self.openssl("x509", "-req", "-in", csr, "-CA", self.ca, "-CAkey", self.ca_key,
                     "-CAcreateserial", "-out", cert, "-days", "2", "-extfile", extensions)
        os.chmod(key, 0o600)
        return cert, key

    def inspect(self, role="server", certificate=None, key=None, ca=None, expected="node.example.test",
                validity=86400, previous=None, expected_ip=None):
        request = mtls.CheckRequest(role=role, cert=certificate or self.server,
                                   key=key or self.server_key, ca=ca or self.ca,
                                   expected_dns=expected if expected_ip is None else None,
                                   expected_ip=expected_ip, min_valid_seconds=validity,
                                   previous_cert=previous)
        return mtls.check(request)

    def test_accepts_real_p256_server_certificate_and_safe_receipt(self):
        result = self.inspect()
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["role"], "server")
        self.assertEqual(result["identity"], "node.example.test")
        self.assertEqual(len(result["certificate_sha256"]), 64)
        self.assertNotIn(str(self.server_key), json.dumps(result))

    def test_accepts_valid_client_identity_and_eku(self):
        result = self.inspect(role="client", certificate=self.client, key=self.client_key,
                              expected="agent.example.test")
        self.assertEqual(result["role"], "client")

    def test_rejects_wrong_client_eku(self):
        with self.assertRaises(mtls.MtlsError):
            self.inspect(role="client")

    def test_rejects_rogue_ca(self):
        alien_ca = self.root / "alien.pem"
        alien_key = self.root / "alien.key"
        self.openssl("req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                     "-nodes", "-keyout", alien_key, "-out", alien_ca, "-days", "2",
                     "-subj", "/CN=Other CA", "-addext", "basicConstraints=critical,CA:TRUE")
        with self.assertRaises(mtls.MtlsError):
            self.inspect(ca=alien_ca)

    def test_rejects_key_certificate_mismatch(self):
        with self.assertRaises(mtls.MtlsError):
            self.inspect(key=self.client_key)

    def test_rejects_identity_mismatch(self):
        with self.assertRaises(mtls.MtlsError):
            self.inspect(expected="other.example.test")

    def test_rejects_name_only_cn_without_san(self):
        cert, key = self.generate_cert("no-san", "serverAuth", "DNS:unrelated.example.test")
        with self.assertRaises(mtls.MtlsError):
            self.inspect(certificate=cert, key=key, expected="no-san")

    def test_rejects_certificate_with_insufficient_future_lifetime(self):
        with self.assertRaises(mtls.MtlsError):
            self.inspect(validity=3 * 86400)

    def test_rejects_symlink_private_key(self):
        link = self.root / "symlink.key"
        link.symlink_to(self.server_key)
        with self.assertRaises(mtls.MtlsError):
            self.inspect(key=link)

    def test_rejects_world_readable_key(self):
        os.chmod(self.server_key, 0o644)
        with self.assertRaises(mtls.MtlsError):
            self.inspect()

    def test_rejects_invalid_expected_dns_and_ip(self):
        with self.assertRaises(mtls.MtlsError):
            self.inspect(expected="*.example.test")
        with self.assertRaises(mtls.MtlsError):
            self.inspect(expected_ip="999.999.9.9")

    def test_rotation_requires_distinct_cert_fingerprint(self):
        with self.assertRaises(mtls.MtlsError):
            self.inspect(previous=self.server)
        rotated, key = self.generate_cert("rotated", "serverAuth", "DNS:node.example.test")
        outcome = self.inspect(certificate=rotated, key=key, previous=self.server)
        self.assertTrue(outcome["rotation_candidate"])
        self.assertNotEqual(outcome["certificate_sha256"], outcome["previous_certificate_sha256"])

    def test_ca_and_certificate_paths_must_be_real_files(self):
        link = self.root / "ca-link.pem"
        link.symlink_to(self.ca)
        with self.assertRaises(mtls.MtlsError):
            self.inspect(ca=link)
        with self.assertRaises(mtls.MtlsError):
            self.inspect(certificate=self.root)


if __name__ == "__main__":
    unittest.main()
