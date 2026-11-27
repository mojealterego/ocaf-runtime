"""Real loopback tests for bounded local HTTP connection admission."""
import http.client
import importlib.util
from pathlib import Path
import socket
import tempfile
import threading
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'ocaf_connection_runner', ROOT / 'apps/agents-builder-web/local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class HeldBackend:
    def __init__(self):
        self.entered = threading.Event()
        self.release = threading.Event()

    def status(self):
        self.entered.set()
        if not self.release.wait(5):
            raise RuntimeError('test barrier timed out')
        return {'state': 'stopped', 'model': None}

    def stop(self):
        self.release.set()


class SingleConnectionServer(runner.OCAFServer):
    max_connections = 1

    def __init__(self, *args):
        self.finished = threading.Event()
        super().__init__(*args)

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            self.finished.set()


class ConnectionBudgetTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'index.html').write_bytes(b'public editor')
        self.backend = HeldBackend()
        self.server = SingleConnectionServer(('127.0.0.1', 0), self.root,
                                             self.backend, 'test-token')
        self.port = self.server.server_address[1]
        self.worker = threading.Thread(target=self.server.serve_forever,
                                       kwargs={'poll_interval': 0.01}, daemon=True)
        self.worker.start()
        self.addCleanup(self.close)

    def close(self):
        self.backend.release.set()
        self.server.shutdown()
        self.server.server_close()
        self.worker.join(3)

    def request(self, path):
        conn = http.client.HTTPConnection('127.0.0.1', self.port, timeout=3)
        try:
            conn.request('GET', path, headers={'Authorization': 'Bearer test-token'})
            response = conn.getresponse()
            return response.status, response.read()
        finally:
            conn.close()

    def test_overload_rejected_and_slot_recovers(self):
        outcomes = []
        errors = []

        def first():
            try:
                outcomes.append(self.request('/v1/local-runner/status'))
            except Exception as exc:
                errors.append(exc)

        client = threading.Thread(target=first, daemon=True)
        client.start()
        try:
            self.assertTrue(self.backend.entered.wait(2))
            # On saturation the server rejects before reading any request.
            with socket.create_connection(('127.0.0.1', self.port), timeout=2) as conn:
                response = b''
                while b'\r\n\r\n' not in response:
                    block = conn.recv(4096)
                    if not block:
                        break
                    response += block
                self.assertTrue(response.startswith(b'HTTP/1.1 503 '), response)
                self.assertIn(b'Connection: close\r\n', response)
                self.assertIn(b'Content-Length: 0\r\n', response)
        finally:
            self.backend.release.set()
            client.join(3)
        self.assertFalse(client.is_alive())
        self.assertEqual(errors, [])
        self.assertEqual(outcomes[0][0], 200)
        self.assertTrue(self.server.finished.wait(2))
        self.assertEqual(self.request('/'), (200, b'public editor'))

    def test_worker_start_failure_does_not_leak_admission_slot(self):
        left, right = socket.socketpair()
        try:
            with patch.object(runner.ThreadingHTTPServer, 'process_request',
                              side_effect=RuntimeError('thread start failed')):
                with self.assertRaisesRegex(RuntimeError, 'thread start failed'):
                    self.server.process_request(left, ('127.0.0.1', 0))
        finally:
            left.close()
            right.close()
        self.assertEqual(self.request('/'), (200, b'public editor'))

    def test_malformed_http_releases_slot(self):
        with socket.create_connection(('127.0.0.1', self.port), timeout=2) as conn:
            conn.sendall(b'not HTTP\r\n\r\n')
            conn.shutdown(socket.SHUT_WR)
            while conn.recv(4096):
                pass
        self.assertTrue(self.server.finished.wait(2))
        self.assertEqual(self.request('/'), (200, b'public editor'))

    def test_invalid_connection_budget_rejected(self):
        class InvalidServer(runner.OCAFServer):
            max_connections = 0

        with self.assertRaises(ValueError):
            instance = InvalidServer(('127.0.0.1', 0), self.root,
                                     self.backend, 'test-token')
            instance.server_close()


if __name__ == '__main__':
    unittest.main()
