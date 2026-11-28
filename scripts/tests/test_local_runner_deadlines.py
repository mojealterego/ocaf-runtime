"""Loopback and deterministic-clock regressions for absolute request read budgets."""
import http.client
import importlib.util
from pathlib import Path
import socket
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'ocaf_deadline_runner', ROOT / 'apps/agents-builder-web/local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class Backend:
    def __init__(self):
        self.calls = []

    def chat(self, prompt):
        self.calls.append(('chat', prompt))
        return {'text': prompt}

    def import_model(self, name, source, length):
        data = source.read(length)
        self.calls.append(('import', name, data))
        return {'name': name}

    def stop(self):
        pass


class ReadDeadlineTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'index.html').write_bytes(b'editor')
        self.backend = Backend()
        self.server = runner.OCAFServer(('127.0.0.1', 0), self.root,
                                        self.backend, 'test-token')
        self.port = self.server.server_address[1]
        self.worker = threading.Thread(target=self.server.serve_forever,
                                       kwargs={'poll_interval': 0.01}, daemon=True)
        self.worker.start()
        self.addCleanup(self.close)
        for name, value in [('header_deadline', 0.25), ('json_deadline', 0.25),
                            ('upload_deadline', 0.25), ('timeout', 1.5)]:
            p = patch.object(runner.LocalHandler, name, value, create=True)
            p.start()
            self.addCleanup(p.stop)

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.worker.join(3)

    def headers(self, endpoint, content_type, size):
        return (f'POST /v1/local-runner/{endpoint} HTTP/1.1\r\n'
                f'Host: 127.0.0.1:{self.port}\r\n'
                f'Origin: http://127.0.0.1:{self.port}\r\n'
                'Authorization: Bearer test-token\r\n'
                f'Content-Type: {content_type}\r\n'
                f'Content-Length: {size}\r\n'
                'X-OCAF-Model-Name: model.gguf\r\n\r\n').encode()

    def request(self, endpoint, content_type, data):
        conn = http.client.HTTPConnection('127.0.0.1', self.port, timeout=3)
        try:
            conn.sock = socket.create_connection(('127.0.0.1', self.port), timeout=3)
            conn.sock.sendall(self.headers(endpoint, content_type, len(data)) + data)
            response = conn.response_class(conn.sock, method='POST')
            response.begin()
            return response.status, response.read()
        finally:
            conn.close()

    def slow(self, initial, fragments):
        with socket.create_connection(('127.0.0.1', self.port), timeout=0.8) as conn:
            conn.sendall(initial)
            for fragment in fragments:
                time.sleep(0.09)
                try:
                    conn.sendall(fragment)
                except OSError:
                    break
            received = b''
            try:
                while True:
                    chunk = conn.recv(4096)
                    if not chunk:
                        break
                    received += chunk
            except ConnectionResetError:
                pass
            return received

    def test_slow_header_cannot_extend_deadline(self):
        initial = f'GET / HTTP/1.1\r\nHost: 127.0.0.1:{self.port}\r\nX-Slow: '.encode()
        response = self.slow(initial, [b'a'] * 5 + [b'\r\n\r\n'])
        self.assertFalse(response.startswith(b'HTTP/1.1 200'), response)
        self.assertEqual(self.backend.calls, [])

    def test_slow_json_rejected_before_dispatch(self):
        data = b'{"prompt":"hello"}'
        response = self.slow(self.headers('chat', 'application/json', len(data)),
                             [data[i:i+3] for i in range(0, len(data), 3)])
        self.assertIn(b' 408 ', response)
        self.assertEqual(self.backend.calls, [])

    def test_slow_upload_rejected_before_completion(self):
        data = b'GGUF' + b'x' * 20
        response = self.slow(self.headers('import', 'application/octet-stream', len(data)),
                             [data[i:i+4] for i in range(0, len(data), 4)])
        self.assertIn(b' 408 ', response)
        self.assertEqual(self.backend.calls, [])

    def test_valid_coalesced_bodies(self):
        data = b'{"prompt":"hello"}'
        self.assertEqual(self.request('chat', 'application/json', data)[0], 200)
        payload = b'GGUF' + b'x' * 20
        self.assertEqual(self.request('import', 'application/octet-stream', payload)[0], 201)
        self.assertEqual(self.backend.calls, [('chat', 'hello'), ('import', 'model.gguf', payload)])

    def test_reader_preserves_bytes_after_line_and_handles_eof(self):
        left, right = socket.socketpair()
        self.addCleanup(left.close)
        self.addCleanup(right.close)
        right.sendall(b'line\r\nbody')
        right.shutdown(socket.SHUT_WR)
        with left.makefile('rb') as stream:
            reader = runner.DeadlineReader(stream, left, 12)
            reader.reset(2)
            self.assertEqual(reader.readline(64), b'line\r\n')
            reader.reset(2, body=True)
            self.assertEqual(reader.read(2), b'bo')
            self.assertEqual(reader.read(10), b'dy')
            self.assertEqual(reader.read(1), b'')

    def test_deadline_reader_rejects_expired_buffered_data(self):
        # Real socket buffering, deterministic time: a full buffer must not bypass budget.
        left, right = socket.socketpair()
        self.addCleanup(left.close)
        self.addCleanup(right.close)
        right.sendall(b'hello\n')
        with left.makefile('rb') as stream:
            self.assertTrue(stream.peek(1))
            with patch.object(runner.time, 'monotonic', side_effect=[10.0, 11.0]):
                reader = runner.DeadlineReader(stream, left, 12)
                reader.reset(0.5)
                with self.assertRaises(TimeoutError):
                    reader.readline(32)


if __name__ == '__main__':
    unittest.main()
