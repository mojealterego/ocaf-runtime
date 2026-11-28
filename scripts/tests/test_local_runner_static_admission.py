"""Real loopback HTTP and filesystem tests for static editor admission."""
import http.client
import importlib.util
import os
from pathlib import Path
import tempfile
import threading
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'ocaf_static_runner', ROOT / 'apps/agents-builder-web/local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class Backend:
    def stop(self):
        pass


class StaticAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.web = self.root / 'web'
        self.web.mkdir()
        self.index = self.web / 'index.html'
        self.index.write_bytes(b'public editor')
        self.server = runner.OCAFServer(('127.0.0.1', 0), self.web, Backend(), 'test-token')
        self.port = self.server.server_address[1]
        self.worker = threading.Thread(target=self.server.serve_forever,
                                       kwargs={'poll_interval': 0.01}, daemon=True)
        self.worker.start()
        self.addCleanup(self.close)

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.worker.join(3)

    def request(self, path, timeout=2):
        conn = http.client.HTTPConnection('127.0.0.1', self.port, timeout=timeout)
        try:
            conn.request('GET', path, headers={'Host': f'127.0.0.1:{self.port}'})
            res = conn.getresponse()
            return res.status, res.read()
        finally:
            conn.close()

    def test_valid_assets_and_unknown_path(self):
        self.assertEqual(self.request('/'), (200, b'public editor'))
        self.assertEqual(self.request('/index.html'), (200, b'public editor'))
        self.assertEqual(self.request('/private.txt')[0], 404)

    def test_absolute_and_ambiguous_targets_denied(self):
        for path in ('http://foreign.invalid/index.html', '/index.html?', '/index.html#',
                     '/index.html?x=1', '/%69ndex.html', '//index.html'):
            with self.subTest(path=path):
                self.assertEqual(self.request(path)[0], 400)

    def test_symlink_to_private_file_denied(self):
        secret = self.root / 'private.txt'
        secret.write_bytes(b'private fixture')
        self.index.unlink()
        self.index.symlink_to(secret)
        code, body = self.request('/')
        self.assertEqual(code, 404)
        self.assertNotIn(b'private fixture', body)

    def test_oversized_asset_denied(self):
        self.index.write_bytes(b'x' * (1024 * 1024 + 1))
        self.assertEqual(self.request('/')[0], 404)

    @unittest.skipUnless(hasattr(os, 'mkfifo'), 'requires POSIX FIFO')
    def test_fifo_rejected_without_blocking(self):
        self.index.unlink()
        os.mkfifo(self.index)
        try:
            self.assertEqual(self.request('/', timeout=1)[0], 404)
        finally:
            # Release a reader on the pre-fix implementation, if present.
            try:
                fd = os.open(self.index, os.O_WRONLY | os.O_NONBLOCK)
            except OSError:
                pass
            else:
                os.close(fd)

    def test_replacement_after_lstat_denied(self):
        replacement = self.web / 'replacement'
        replacement.write_bytes(b'replaced asset')
        original = Path.lstat
        swapped = False

        def swap(path, *args, **kwargs):
            nonlocal swapped
            result = original(path, *args, **kwargs)
            if path == self.index and not swapped:
                swapped = True
                os.replace(replacement, self.index)
            return result

        with patch.object(Path, 'lstat', swap):
            self.assertEqual(self.request('/')[0], 404)
        self.assertTrue(swapped)


if __name__ == '__main__':
    unittest.main()
