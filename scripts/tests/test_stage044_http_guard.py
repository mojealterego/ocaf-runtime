"""Redirects in token-bearing acceptance requests must not reach a target."""
import http.server
from pathlib import Path
import ssl
import sys
import threading
import unittest
import urllib.error
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage044_http_guard import guarded_opener


class RedirectTarget(http.server.BaseHTTPRequestHandler):
    captured = []

    def do_POST(self):
        if self.path == "/start":
            self.send_response(302)
            self.send_header("Location", "/capture")
            self.send_header("Content-Length", "0")
            self.end_headers()
        else:
            type(self).captured.append(dict(self.headers))
            self.send_response(200)
            self.send_header("Content-Length", "0")
            self.end_headers()

    def do_GET(self):
        type(self).captured.append(dict(self.headers))
        self.send_response(200)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def log_message(self, format, *args):
        pass


class GuardTests(unittest.TestCase):
    def test_redirect_does_not_forward_oauth_or_vault_token(self):
        RedirectTarget.captured = []
        server = http.server.HTTPServer(("127.0.0.1", 0), RedirectTarget)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            opener = guarded_opener(ssl.create_default_context())
            for name in ("Authorization", "X-Vault-Token"):
                request = urllib.request.Request(
                    f"http://127.0.0.1:{server.server_port}/start",
                    data=b"{}", method="POST", headers={name: "sensitive-token"},
                )
                with self.subTest(name=name), self.assertRaises(urllib.error.HTTPError) as result:
                    opener.open(request, timeout=2)
                self.assertEqual(result.exception.code, 302)
            self.assertEqual(RedirectTarget.captured, [])
        finally:
            server.shutdown()
            server.server_close()
            worker.join(timeout=2)


if __name__ == "__main__":
    unittest.main()
