"""Loopback HTTP admission tests; no real llama model is required."""
import http.client
import importlib.util
from pathlib import Path
import sys
import threading
import unittest
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'ocaf_header_runner', ROOT / 'apps/agents-builder-web/local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class RecordingRunner:
    def __init__(self):
        self.calls = []

    def chat(self, prompt):
        self.calls.append(('chat', prompt))
        return {'text': prompt}

    def import_model(self, name, source, length):
        self.calls.append(('import', name, source.read(length)))
        return {'name': name}

    def stop(self):
        return {'state': 'stopped', 'model': None}


class RecordingServer(runner.OCAFServer):
    def handle_error(self, request, client_address):
        self.errors.append(type(sys.exc_info()[1]).__name__)


class HeaderAdmission(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.backend = RecordingRunner()
        self.token = 'test-local-runner-token'
        self.server = RecordingServer(('127.0.0.1', 0), Path(self.tmp.name),
                                      self.backend, self.token)
        self.server.errors = []
        self.worker = threading.Thread(target=self.server.serve_forever,
                                       kwargs={'poll_interval': 0.01}, daemon=True)
        self.worker.start()
        self.addCleanup(self.close)
        self.port = self.server.server_address[1]

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.worker.join(timeout=3)

    def request(self, *, extra=(), override=None, upload=False):
        body = b'GGUF' if upload else b'{"prompt":"hello"}'
        endpoint = 'import' if upload else 'chat'
        headers = {
            'Host': f'127.0.0.1:{self.port}',
            'Authorization': 'Bearer ' + self.token,
            'Origin': f'http://127.0.0.1:{self.port}',
            'Content-Type': 'application/octet-stream' if upload else 'application/json',
            'Content-Length': str(len(body)),
        }
        if upload:
            headers['X-OCAF-Model-Name'] = 'test.gguf'
        headers.update(override or {})
        connection = http.client.HTTPConnection('127.0.0.1', self.port, timeout=3)
        try:
            connection.putrequest('POST', '/v1/local-runner/' + endpoint, skip_host=True)
            for key, value in [*headers.items(), *extra]:
                connection.putheader(key, value)
            connection.endheaders(body)
            try:
                response = connection.getresponse()
                response.read()
                return response.status
            except http.client.RemoteDisconnected:
                return None
        finally:
            connection.close()

    def assert_denied(self, code, **kwargs):
        self.assertEqual(self.request(**kwargs), code)
        self.assertEqual(self.backend.calls, [])
        self.assertEqual(self.server.errors, [])

    def test_duplicate_host_is_denied(self):
        self.assert_denied(403, extra=[('Host', 'foreign.invalid')])

    def test_duplicate_origin_is_denied(self):
        self.assert_denied(403, extra=[('Origin', 'http://foreign.invalid')])

    def test_duplicate_authorization_is_denied(self):
        self.assert_denied(401, extra=[('Authorization', 'Bearer conflicting')])

    def test_non_ascii_authorization_is_denied_without_handler_exception(self):
        self.assert_denied(401, override={'Authorization': b'Bearer \xff'})

    def test_duplicate_content_type_is_denied(self):
        self.assert_denied(415, extra=[('Content-Type', 'text/html')])

    def test_duplicate_model_name_is_denied(self):
        self.assert_denied(400, upload=True, extra=[('X-OCAF-Model-Name', 'other.gguf')])

    def test_single_valid_headers_reach_backend(self):
        self.assertEqual(self.request(), 200)
        self.assertEqual(self.request(upload=True), 201)
        self.assertEqual(self.backend.calls, [('chat', 'hello'), ('import', 'test.gguf', b'GGUF')])
        self.assertEqual(self.server.errors, [])


if __name__ == '__main__':
    unittest.main()
