import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'stage038_assert_denial.py'


class TwoHostDenialEvidenceTests(unittest.TestCase):
    def check(self, exit_code: int, stdout: str, stderr: str) -> subprocess.CompletedProcess:
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            out = root / 'client.stdout'
            err = root / 'client.stderr'
            out.write_text(stdout, encoding='utf-8')
            err.write_text(stderr, encoding='utf-8')
            return subprocess.run([
                sys.executable, str(SCRIPT), '--exit-code', str(exit_code),
                '--stdout-file', str(out), '--stderr-file', str(err),
            ], capture_output=True, text=True, check=False)

    def test_accepts_only_application_layer_rejection(self):
        self.assertEqual(self.check(1, '', 'Error: remote RPC rejected\n').returncode, 0)

    def test_rejects_successful_cross_principal_call(self):
        self.assertNotEqual(self.check(0, '', 'Error: remote RPC rejected\n').returncode, 0)

    def test_rejects_nonzero_network_or_transport_error(self):
        for error in ('Error: Connection refused\n', 'Error: deadline has elapsed\n',
                      'Error: invalid peer certificate\n', '', 'Error: remote RPC rejected\nError: network down\n'):
            with self.subTest(error=error):
                self.assertNotEqual(self.check(1, '', error).returncode, 0)

    def test_rejects_stdout_on_denial(self):
        self.assertNotEqual(self.check(1, '{"output":{"ok":true}}\n', 'Error: remote RPC rejected\n').returncode, 0)

    def test_rejects_invalid_exit_code(self):
        self.assertNotEqual(self.check(-1, '', 'Error: remote RPC rejected\n').returncode, 0)


if __name__ == '__main__':
    unittest.main()
