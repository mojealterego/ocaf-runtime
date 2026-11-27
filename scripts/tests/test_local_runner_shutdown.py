"""Process ownership and bounded shutdown regression tests."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import threading
import unittest
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'ocaf_shutdown_runner', ROOT / 'apps/agents-builder-web/local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class ShutdownTests(unittest.TestCase):
    def setUp(self):
        self.runner = runner.LocalRunner.__new__(runner.LocalRunner)
        self.runner.lock = threading.RLock()
        self.proc = Mock(spec=subprocess.Popen)
        self.proc.poll.return_value = None
        self.runner.process = self.proc
        self.runner.model = 'model.gguf'
        self.runner.backend_port = 12345
        self.runner.backend_key = 'test-key'
        self.runner.settings = {'seed': 1}

    def assert_retained(self):
        self.assertIs(self.runner.process, self.proc)
        self.assertEqual(self.runner.model, 'model.gguf')
        self.assertEqual(self.runner.backend_port, 12345)
        self.assertEqual(self.runner.backend_key, 'test-key')
        self.assertEqual(self.runner.settings, {'seed': 1})

    def assert_stopped(self):
        for field in ('process', 'model', 'backend_port', 'backend_key', 'settings'):
            self.assertIsNone(getattr(self.runner, field))

    def test_terminate_failure_retains_handle_for_retry(self):
        self.proc.terminate.side_effect = PermissionError('denied')
        with self.assertRaises(runner.RunnerError) as error:
            self.runner.stop()
        self.assertEqual(error.exception.status, 503)
        self.assert_retained()
        self.proc.terminate.side_effect = None
        self.proc.poll.return_value = 0
        self.assertEqual(self.runner.stop(), {'state': 'stopped', 'model': None})
        self.assert_stopped()

    def test_kill_failure_retains_handle(self):
        self.proc.wait.side_effect = subprocess.TimeoutExpired('llama', 4)
        self.proc.kill.side_effect = PermissionError('denied')
        with self.assertRaises(runner.RunnerError) as error:
            self.runner.stop()
        self.assertEqual(error.exception.status, 503)
        self.assert_retained()
        self.proc.kill.assert_called_once()

    def test_second_timeout_retains_handle(self):
        self.proc.wait.side_effect = subprocess.TimeoutExpired('llama', 4)
        with self.assertRaises(runner.RunnerError) as error:
            self.runner.stop()
        self.assertEqual(error.exception.status, 503)
        self.assert_retained()
        self.assertEqual(self.proc.wait.call_count, 2)

    def test_exit_during_terminate_is_successful(self):
        self.proc.poll.side_effect = [None, 0]
        self.proc.terminate.side_effect = ProcessLookupError('already exited')
        self.assertEqual(self.runner.stop(), {'state': 'stopped', 'model': None})
        self.assert_stopped()

    def test_graceful_then_force_kill_reaps_process(self):
        self.proc.wait.side_effect = [subprocess.TimeoutExpired('llama', 4), -9]
        self.assertEqual(self.runner.stop(), {'state': 'stopped', 'model': None})
        self.proc.terminate.assert_called_once()
        self.proc.kill.assert_called_once()
        self.assertEqual(self.proc.wait.call_count, 2)
        self.assert_stopped()

    def test_real_child_is_reaped_and_stop_is_idempotent(self):
        child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])
        def cleanup():
            if child.poll() is None:
                child.kill()
                child.wait(timeout=3)
        self.addCleanup(cleanup)
        self.runner.process = child
        self.assertEqual(self.runner.stop(), {'state': 'stopped', 'model': None})
        self.assertIsNotNone(child.poll())
        self.assert_stopped()
        self.assertEqual(self.runner.stop(), {'state': 'stopped', 'model': None})


if __name__ == '__main__':
    unittest.main()
