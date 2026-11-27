"""Deterministic process-generation and backend JSON boundary regressions."""
import importlib.util
import json
from pathlib import Path
import threading
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'ocaf_generation_runner', ROOT / 'apps/agents-builder-web/local_runner.py')
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class Process:
    exit_code = None

    def poll(self):
        return self.exit_code


class Response:
    status = 200

    def __init__(self, raw, hook):
        self.raw = raw
        self.hook = hook

    def read(self, limit):
        self.hook()
        return self.raw[:limit]


class GenerationTests(unittest.TestCase):
    def setUp(self):
        self.runner = runner.LocalRunner.__new__(runner.LocalRunner)
        self.runner.lock = threading.RLock()
        self.runner.request_slot = threading.BoundedSemaphore(1)
        self.runner.settings = dict.fromkeys(('max_output_tokens', 'temperature', 'top_p',
            'top_k', 'min_p', 'repeat_penalty', 'presence_penalty', 'frequency_penalty', 'seed'), 1)
        self.runner.process = Process()
        self.runner.model = 'original.gguf'
        self.runner.backend_port = 12345
        self.runner.backend_key = 'original-key'
        self.health_hook = lambda: None
        self.chat_hook = lambda: None
        self.raw = b'{"choices":[{"message":{"content":"ok"}}]}'
        self.calls = []
        owner = self

        class Connection:
            def __init__(self, host, port, timeout):
                self.port = port

            def request(self, method, path, payload=None, headers=None):
                self.method = method
                owner.calls.append((method, self.port, payload))

            def getresponse(self):
                if self.method == 'GET':
                    return Response(b'{}', owner.health_hook)
                return Response(owner.raw, owner.chat_hook)

            def close(self):
                pass

        self.addCleanup(patch.stopall)
        patch.object(runner.http.client, 'HTTPConnection', Connection).start()

    def replace_process(self):
        with self.runner.lock:
            self.runner.process = Process()
            self.runner.model = 'replacement.gguf'
            self.runner.backend_port = 23456
            self.runner.backend_key = 'replacement-key'

    def clear_process(self):
        with self.runner.lock:
            self.runner.process = None
            self.runner.model = None
            self.runner.settings = None
            self.runner.backend_port = None
            self.runner.backend_key = None

    def assert_chat_denied(self, status):
        with self.assertRaises(runner.RunnerError) as error:
            self.runner.chat('hello')
        self.assertEqual(error.exception.status, status)
        self.assertTrue(self.runner.request_slot.acquire(blocking=False))
        self.runner.request_slot.release()

    def test_health_replacement_is_not_ready(self):
        self.health_hook = self.replace_process
        self.assertEqual(self.runner.status(), {'state': 'loading', 'model': 'replacement.gguf'})

    def test_health_stop_is_not_ready(self):
        self.health_hook = self.clear_process
        self.assertEqual(self.runner.status(), {'state': 'stopped', 'model': None})

    def test_health_exit_is_not_ready(self):
        def exit_process():
            self.runner.process.exit_code = 9
        self.health_hook = exit_process
        self.assertEqual(self.runner.status(), {'state': 'failed', 'model': 'original.gguf', 'exit_code': 9})

    def test_replacement_during_health_never_dispatches_chat(self):
        self.health_hook = self.replace_process
        self.assert_chat_denied(409)
        self.assertEqual([call[0] for call in self.calls], ['GET'])

    def test_stop_during_health_never_dispatches_chat(self):
        self.health_hook = self.clear_process
        self.assert_chat_denied(409)
        self.assertEqual([call[0] for call in self.calls], ['GET'])

    def test_replacement_during_inference_discards_stale_output(self):
        self.chat_hook = self.replace_process
        self.assert_chat_denied(409)
        self.assertEqual(json.loads(self.calls[-1][2])['model'], 'original.gguf')

    def test_stop_during_inference_discards_stale_output(self):
        self.chat_hook = self.clear_process
        self.assert_chat_denied(409)

    def test_ambiguous_or_invalid_backend_json_is_rejected(self):
        for raw in (b'{"choices":[{"message":{"content":"old","content":"new"}}]}',
                    b'{"choices":[{"message":{"content":"\\ud800"}}]}',
                    b'{"choices":[{"message":{"content":"ok"}}],"n":NaN}'):
            with self.subTest(raw=raw):
                self.raw = raw
                self.assert_chat_denied(502)

    def test_valid_inference_preserves_model_and_releases_slot(self):
        self.assertEqual(self.runner.chat('hello'), {'text': 'ok', 'model': 'original.gguf'})
        self.assertEqual(self.runner.chat('again'), {'text': 'ok', 'model': 'original.gguf'})


if __name__ == '__main__':
    unittest.main()
