"""GGUF import admission, recovery, and real filesystem tests."""
import hashlib
import importlib.util
import io
from pathlib import Path
import struct
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'ocaf_import_runner', ROOT / 'apps/agents-builder-web/local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)
PAYLOAD = b'GGUF' + struct.pack('<IQQ', 3, 1, 0) + b'payload'


class ImportBudgetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runner = runner.LocalRunner(self.root, Path(sys.executable), max_model_bytes=1024)

    def assert_clean(self):
        self.assertFalse(list(self.root.glob('.ocaf-gguf-upload-*')))

    def valid_import(self, name='valid.gguf'):
        receipt = self.runner.import_model(name, io.BytesIO(PAYLOAD), len(PAYLOAD))
        self.assertEqual(receipt['sha256'], hashlib.sha256(PAYLOAD).hexdigest())
        self.assertEqual((self.root / name).read_bytes(), PAYLOAD)
        self.assert_clean()

    def test_concurrent_upload_is_rejected_before_read(self):
        entered = threading.Event()
        release = threading.Event()
        errors = []

        class HeldSource(io.BytesIO):
            def read(self, size):
                entered.set()
                if not release.wait(3):
                    raise RuntimeError('test release timeout')
                return super().read(size)

        def first():
            try:
                self.runner.import_model('first.gguf', HeldSource(PAYLOAD), len(PAYLOAD))
            except Exception as exc:
                errors.append(exc)

        worker = threading.Thread(target=first, daemon=True)
        worker.start()
        try:
            self.assertTrue(entered.wait(3))
            other = io.BytesIO(PAYLOAD)
            with self.assertRaises(runner.RunnerError) as error:
                self.runner.import_model('second.gguf', other, len(PAYLOAD))
            self.assertEqual(error.exception.status, 429)
            self.assertEqual(other.tell(), 0)
        finally:
            release.set()
            worker.join(3)
        self.assertFalse(worker.is_alive())
        self.assertEqual(errors, [])
        self.assertFalse((self.root / 'second.gguf').exists())
        self.valid_import('second.gguf')

    def test_storage_admission_failure_is_controlled_and_retryable(self):
        with patch.object(runner.tempfile, 'mkstemp', side_effect=OSError('disk full')):
            with self.assertRaises(runner.RunnerError) as error:
                self.runner.import_model('failed.gguf', io.BytesIO(PAYLOAD), len(PAYLOAD))
        self.assertEqual(error.exception.status, 507)
        self.assertFalse((self.root / 'failed.gguf').exists())
        self.valid_import()

    def test_source_overread_rejected_before_publication(self):
        class Overread(io.BytesIO):
            calls = 0

            def read(self, size):
                self.calls += 1
                return super().read(abs(size) + 1)

        source = Overread(PAYLOAD + b'x')
        with self.assertRaises(runner.RunnerError) as error:
            self.runner.import_model('oversized.gguf', source, len(PAYLOAD))
        self.assertEqual(source.calls, 1)
        self.assertEqual(error.exception.status, 422)
        self.assertFalse((self.root / 'oversized.gguf').exists())
        self.valid_import()

    def test_truncation_releases_slot_and_removes_temp_file(self):
        with self.assertRaises(runner.RunnerError):
            self.runner.import_model('short.gguf', io.BytesIO(PAYLOAD[:4]), len(PAYLOAD))
        self.assertFalse((self.root / 'short.gguf').exists())
        self.assert_clean()
        self.valid_import()

    def test_existing_model_unchanged(self):
        self.valid_import()
        with self.assertRaises(runner.RunnerError) as error:
            self.runner.import_model('valid.gguf', io.BytesIO(PAYLOAD), len(PAYLOAD))
        self.assertEqual(error.exception.status, 409)
        self.assertEqual((self.root / 'valid.gguf').read_bytes(), PAYLOAD)
        self.assert_clean()


if __name__ == '__main__':
    unittest.main()
