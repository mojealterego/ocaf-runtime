"""Local TLS end-to-end tests for signed 0.39 egress policy; no guest claim."""
import base64
import http.server
import ipaddress
import json
from pathlib import Path
import shutil
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import unittest

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_egress_broker as egress
from stage039_signed_manifest import canonical_payload


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        body = b"actual-local-tls-upstream"
        if self.path == "/redirect":
            self.send_response(302)
            self.send_header("Location", "https://169.254.169.254/latest/meta-data/")
            body = b""
        else:
            self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass


class EgressBrokerTLS(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if shutil.which("openssl") is None:
            raise unittest.SkipTest("openssl unavailable for local TLS fixture")
        cls.tmp = tempfile.TemporaryDirectory()
        path = Path(cls.tmp.name)
        cls.cert = path / "cert.pem"
        key = path / "key.pem"
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048",
                        "-keyout", str(key), "-out", str(cls.cert), "-days", "1",
                        "-nodes", "-subj", "/CN=localhost",
                        "-addext", "subjectAltName=DNS:localhost"],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cls.cert, key)
        cls.server.socket = context.wrap_socket(cls.server.socket, server_side=True)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.port = cls.server.server_address[1]

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=2)
        cls.tmp.cleanup()

    def setUp(self):
        self.key = Ed25519PrivateKey.generate()
        self.public = self.key.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        self.calls = []

        def resolver(host, port, **kwargs):
            self.calls.append((host, port))
            return socket.getaddrinfo("127.0.0.1", port, **kwargs)

        tls = ssl.create_default_context(cafile=str(self.cert))
        self.broker = egress.EgressBroker(
            trusted_keys={"lab": self.public}, ssl_context=tls,
            allowed_ports=frozenset({self.port}),
            allowed_networks=(ipaddress.ip_network("127.0.0.1/32"),),
            resolver=resolver)

    def permit(self, *, url=None, tenant="tenant-a", nonce="a" * 32, cap=128,
               key=None):
        payload = {"schema_version": 1, "tenant_id": tenant, "nonce": nonce,
                   "url": url or f"https://localhost:{self.port}/ok",
                   "expires_unix": int(time.time()) + 60, "max_bytes": cap}
        signer = key or self.key
        public = signer.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        return json.dumps({"key_id": "lab", "public_key": base64.b64encode(public).decode(),
                           "signature": base64.b64encode(signer.sign(canonical_payload(payload))).decode(),
                           "payload": payload}).encode()

    def test_signed_tenant_fetch_pins_one_dns_answer_to_real_tls_peer(self):
        self.assertEqual(self.broker.fetch(signed_permit=self.permit(),
                                           authenticated_tenant="tenant-a"),
                         b"actual-local-tls-upstream")
        self.assertEqual(self.calls, [("localhost", self.port)])
        with self.assertRaisesRegex(egress.EgressDenied, "replayed"):
            self.broker.fetch(signed_permit=self.permit(), authenticated_tenant="tenant-a")

    def test_durable_replay_survives_broker_restart(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "permits.sqlite"
            store = egress.DurablePermitReplay(path)
            self.broker.replay_store = store
            first = self.permit(nonce="7" * 32)
            self.assertEqual(self.broker.fetch(signed_permit=first,
                                               authenticated_tenant="tenant-a"),
                             b"actual-local-tls-upstream")
            store.close()
            restarted_store = egress.DurablePermitReplay.reopen(path)
            try:
                restarted = egress.EgressBroker(
                    trusted_keys=self.broker.trusted_keys,
                    ssl_context=self.broker.ssl_context,
                    allowed_ports=self.broker.allowed_ports,
                    allowed_networks=self.broker.allowed_networks,
                    resolver=self.broker.resolver,
                    replay_store=restarted_store)
                before = len(self.calls)
                with self.assertRaisesRegex(egress.EgressDenied, "replayed"):
                    restarted.fetch(signed_permit=first, authenticated_tenant="tenant-a")
                self.assertEqual(len(self.calls), before)
                self.assertEqual(restarted.fetch(signed_permit=self.permit(nonce="8" * 32),
                                                 authenticated_tenant="tenant-a"),
                                 b"actual-local-tls-upstream")
            finally:
                restarted_store.close()

    def test_durable_replay_rejects_mutable_file(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "permits.sqlite"
            store = egress.DurablePermitReplay(path)
            store.close()
            path.chmod(0o666)
            with self.assertRaisesRegex(egress.EgressDenied, "permissions"):
                egress.DurablePermitReplay.reopen(path)

    def test_durable_replay_is_shared_with_new_process(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "permits.sqlite"
            store = egress.DurablePermitReplay(path)
            store.claim("tenant-a", "9" * 32, int(time.time()) + 60,
                        int(time.time()))
            store.close()
            script = (
                "import sys,time; from pathlib import Path; "
                "from stage039_egress_broker import DurablePermitReplay,EgressDenied; "
                "db=DurablePermitReplay.reopen(Path(sys.argv[1])); "
                "\ntry: db.claim('tenant-a','9'*32,int(time.time())+60,int(time.time()))"
                "\nexcept EgressDenied as exc: assert 'replayed' in str(exc)"
                "\nelse: raise AssertionError('replay admitted after process restart')"
                "\nfinally: db.close()"
            )
            subprocess.run([sys.executable, "-c", script, str(path)],
                           check=True, env={"PYTHONPATH": str(Path(egress.__file__).parent)})

    def test_rejects_redirect_metadata_mixed_dns_and_rebinding(self):
        redirect = self.permit(url=f"https://localhost:{self.port}/redirect")
        with self.assertRaisesRegex(egress.EgressDenied, "redirects"):
            self.broker.fetch(signed_permit=redirect, authenticated_tenant="tenant-a")
        self.assertEqual(len(self.calls), 1)
        self.broker.resolver = lambda host, port, **kw: [
            *socket.getaddrinfo("127.0.0.1", port, **kw),
            *socket.getaddrinfo("169.254.169.254", port, **kw)]
        with self.assertRaisesRegex(egress.EgressDenied, "private, metadata"):
            self.broker.fetch(signed_permit=self.permit(nonce="b" * 32),
                              authenticated_tenant="tenant-a")
        self.broker.resolver = lambda host, port, **kw: socket.getaddrinfo(
            "127.0.0.2", port, **kw)
        with self.assertRaisesRegex(egress.EgressDenied, "private, metadata"):
            self.broker.fetch(signed_permit=self.permit(nonce="c" * 32),
                              authenticated_tenant="tenant-a")

    def test_denies_wrong_signer_tenant_userinfo_ip_and_resolver_outage(self):
        for label, permit, tenant in (
            ("wrong-signer", self.permit(key=Ed25519PrivateKey.generate()), "tenant-a"),
            ("wrong-tenant", self.permit(nonce="d" * 32), "tenant-b"),
            ("userinfo", self.permit(url=f"https://user@localhost:{self.port}/ok", nonce="e" * 32), "tenant-a"),
            ("IP-literal", self.permit(url=f"https://127.0.0.1:{self.port}/ok", nonce="f" * 32), "tenant-a"),
        ):
            with self.subTest(label=label), self.assertRaises(egress.EgressDenied):
                self.broker.fetch(signed_permit=permit, authenticated_tenant=tenant)
        self.assertEqual(self.calls, [])
        self.broker.resolver = lambda *_a, **_kw: (_ for _ in ()).throw(OSError("DNS down"))
        with self.assertRaisesRegex(egress.EgressDenied, "DNS unavailable"):
            self.broker.fetch(signed_permit=self.permit(nonce="1" * 32),
                              authenticated_tenant="tenant-a")

    def test_resolver_cannot_change_signed_port_or_bypass_default_private_ip_deny(self):
        self.broker.resolver = lambda host, port, **kw: socket.getaddrinfo(
            "127.0.0.1", port + 1, **kw)
        with self.assertRaisesRegex(egress.EgressDenied, "changed signed destination port"):
            self.broker.fetch(signed_permit=self.permit(nonce="4" * 32),
                              authenticated_tenant="tenant-a")
        self.broker.allowed_networks = ()
        self.broker.resolver = lambda host, port, **kw: socket.getaddrinfo(
            "127.0.0.1", port, **kw)
        with self.assertRaisesRegex(egress.EgressDenied, "private, metadata"):
            self.broker.fetch(signed_permit=self.permit(nonce="5" * 32),
                              authenticated_tenant="tenant-a")

    def test_rejects_tls_hostname_mismatch_and_oversized_response(self):
        # Connection still goes only to the approved literal 127.0.0.1, but
        # the certificate is for localhost, not the signed other hostname.
        with self.assertRaisesRegex(egress.EgressDenied, "TLS egress"):
            self.broker.fetch(signed_permit=self.permit(
                url=f"https://other.example:{self.port}/ok", nonce="2" * 32),
                authenticated_tenant="tenant-a")
        with self.assertRaisesRegex(egress.EgressDenied, "body length"):
            self.broker.fetch(signed_permit=self.permit(nonce="3" * 32, cap=4),
                              authenticated_tenant="tenant-a")


if __name__ == "__main__":
    unittest.main()
