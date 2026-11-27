"""Local GGUF file inspection regression tests using real filesystem objects."""
import importlib.util
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / 'apps/agents-builder-web/local_runner.py'
SPEC = importlib.util.spec_from_file_location('ocaf_file_runner', MODULE)
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)
HEADER = b'GGUF' + struct.pack('<IQQ', 3, 1, 0)


class FileAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.model = self.root / 'model.gguf'
        self.model.write_bytes(HEADER)

    def test_valid_header(self):
        self.assertEqual(runner.check_gguf(self.root, self.model.name, 1024),
                         {'name': self.model.name, 'size_bytes': 24,
                          'version': 3, 'tensor_count': 1})

    def test_symlink_and_directory_denied(self):
        link = self.root / 'link.gguf'
        link.symlink_to(self.model)
        directory = self.root / 'directory.gguf'
        directory.mkdir()
        for path in (link, directory):
            with self.subTest(path=path.name):
                with self.assertRaises(runner.RunnerError):
                    runner.check_gguf(self.root, path.name, 1024)

    @unittest.skipUnless(hasattr(os, 'mkfifo'), 'requires POSIX FIFO')
    def test_fifo_cannot_block_inspection(self):
        fifo = self.root / 'fifo.gguf'
        os.mkfifo(fifo)
        script = '''import importlib.util, sys
from pathlib import Path
s=importlib.util.spec_from_file_location('m',sys.argv[1])
m=importlib.util.module_from_spec(s); s.loader.exec_module(m)
try:
    m.check_gguf(Path(sys.argv[2]), 'fifo.gguf', 1024)
except m.RunnerError as exc:
    sys.exit(0 if exc.status == 422 else 2)
sys.exit(1)
'''
        result = subprocess.run([sys.executable, '-c', script, str(MODULE),
                                 str(self.root)], timeout=3, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_replacement_after_lstat_denied(self):
        other = self.root / 'replacement.gguf'
        other.write_bytes(HEADER + b'x')
        original = Path.lstat
        replaced = False

        def swap(path, *args, **kwargs):
            nonlocal replaced
            value = original(path, *args, **kwargs)
            if path == self.model and not replaced:
                replaced = True
                os.replace(other, self.model)
            return value

        with patch.object(Path, 'lstat', swap):
            with self.assertRaises(runner.RunnerError):
                runner.check_gguf(self.root, self.model.name, 1024)
        self.assertTrue(replaced)

    def test_truncated_header_after_stat_denied(self):
        original = os.fstat

        def truncate(fd):
            value = original(fd)
            self.model.write_bytes(b'GGUF')
            return value

        with patch.object(os, 'fstat', truncate):
            with self.assertRaises(runner.RunnerError):
                runner.check_gguf(self.root, self.model.name, 1024)


if __name__ == '__main__':
    unittest.main()
