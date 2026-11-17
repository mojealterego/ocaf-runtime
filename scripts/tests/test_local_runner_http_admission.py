"""HTTP integration tests with a recording runner; no model or native build proof."""
import http.client
import importlib.util
from pathlib import Path
import re
import threading
import unittest
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[2]
WEB = ROOT / 'apps/agents-builder-web'
SPEC = importlib.util.spec_from_file_location('runner', WEB / 'local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class RecordingRunner:
    def __init__(self):
        self.calls = []

    def chat(self, prompt):
        self.calls.append(prompt)
        return {'text': prompt}

    def stop(self):
        return {'state': 'stopped', 'model': None}


class HTTPAdmission(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.backend = RecordingRunner()
        self.token = 'test-local-runner-token'
        self.server = runner.OCAFServer(('127.0.0.1', 0), self.root, self.backend, self.token)
        self.worker = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.worker.start()
        self.addCleanup(self.close)
        self.port = self.server.server_address[1]

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.worker.join(timeout=3)

    def request(self, method, path, body=None, headers=None):
        connection = http.client.HTTPConnection('127.0.0.1', self.port, timeout=3)
        try:
            request_headers = {
                'Authorization': 'Bearer ' + self.token,
                'Origin': f'http://127.0.0.1:{self.port}',
                'Content-Type': 'application/json',
            }
            request_headers.update(headers or {})
            connection.request(method, path, body, request_headers)
            response = connection.getresponse()
            return response.status, response.read()
        finally:
            connection.close()

    def test_every_cached_editor_asset_is_served(self):
        source = (WEB / 'sw.js').read_text(encoding='utf-8')
        assets = re.search(r'const ASSETS = \[(.*?)\];', source).group(1)
        names = [name for name in re.findall(r"'\./([^']*)'", assets) if name]
        self.assertIn('preview-gate.mjs', names)
        self.assertIn('strict-json.mjs', names)
        for name in names:
            with self.subTest(name=name):
                payload = (WEB / name).read_bytes()
                (self.root / name).write_bytes(payload)
                code, data = self.request('GET', '/' + name)
                self.assertEqual(code, 200)
                self.assertEqual(data, payload)
        self.assertEqual(self.request('GET', '/local_runner.py')[0], 404)

    def test_duplicate_keys_do_not_reach_backend(self):
        code, _ = self.request('POST', '/v1/local-runner/chat', b'{"prompt":"first","pr\\u006fmpt":"second"}')
        self.assertEqual(code, 400)
        self.assertEqual(self.backend.calls, [])

    def test_invalid_nonfinite_json_and_excessive_nesting(self):
        for body in (b'{"prompt":NaN}', b'{"prompt":Infinity}', b'{"prompt":1e999}',
                     b'{"prompt":"\\ud800"}', b'{"prompt":"\xff"}',
                     b'{"prompt":' + b'[' * 65 + b'0' + b']' * 65 + b'}'):
            with self.subTest(body=body[:35]):
                self.assertEqual(self.request('POST', '/v1/local-runner/chat', body)[0], 400)
        self.assertEqual(self.backend.calls, [])

    def test_ambiguous_http_framing_rejected(self):
        for headers in (
            [('Content-Length', '14'), ('Content-Length', '14')],
            [('Transfer-Encoding', 'chunked'), ('Content-Length', '14')],
            [('Content-Length', '+14')],
        ):
            with self.subTest(headers=headers):
                conn = http.client.HTTPConnection('127.0.0.1', self.port, timeout=3)
                try:
                    conn.putrequest('POST', '/v1/local-runner/chat')
                    conn.putheader('Authorization', 'Bearer ' + self.token)
                    conn.putheader('Origin', f'http://127.0.0.1:{self.port}')
                    conn.putheader('Content-Type', 'application/json')
                    for name, value in headers:
                        conn.putheader(name, value)
                    conn.endheaders(b'{"prompt":"x"}')
                    response = conn.getresponse()
                    self.assertEqual(response.status, 400)
                    response.read()
                finally:
                    conn.close()
        self.assertEqual(self.backend.calls, [])

    def test_valid_unicode_json_reaches_backend_once(self):
        self.assertEqual(self.request('POST', '/v1/local-runner/chat', '{"prompt":"Zażółć"}'.encode())[0], 200)
        self.assertEqual(self.backend.calls, ['Zażółć'])


if __name__ == '__main__':
    unittest.main()
