"""Security and integration contract for the localhost GGUF llama.cpp gateway."""
from __future__ import annotations

import importlib.util
import http.client
import json
from pathlib import Path
import stat
import struct
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "apps/agents-builder-web/local_runner.py"
SPEC = importlib.util.spec_from_file_location("ocaf_local_runner", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
gateway = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gateway)


def settings():
    return {
        "context_size": 4096, "threads": 2, "batch_size": 128,
        "gpu_layers": 0, "temperature": 0.7, "top_p": 0.95,
        "top_k": 40, "min_p": 0.05, "repeat_penalty": 1.1,
        "presence_penalty": 0, "frequency_penalty": 0, "seed": -1,
        "max_output_tokens": 256, "flash_attention": False, "use_mmap": True,
    }


FAKE_LLAMA_SERVER = r'''#!/usr/bin/env python3
import http.server, json, os, sys
port = int(sys.argv[sys.argv.index("--port") + 1])
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self,*args): pass
    def send(self,status,body):
        raw=json.dumps(body).encode()
        self.send_response(status);self.send_header("Content-Type","application/json")
        self.send_header("Content-Length",str(len(raw)));self.end_headers();self.wfile.write(raw)
    def do_GET(self):
        if self.path=="/health":self.send(200,{"status":"ok"})
        else:self.send(404,{"error":"not found"})
    def do_POST(self):
        if self.path!="/v1/chat/completions" or self.headers.get("Authorization")!="Bearer "+os.environ.get("LLAMA_API_KEY"):
            self.send(403,{"error":"denied"});return
        body=json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.send(200,{"choices":[{"message":{"content":"REAL_HTTP_MOCK: "+body["messages"][0]["content"]}}]})
http.server.ThreadingHTTPServer(("127.0.0.1",port),Handler).serve_forever()
'''


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ocaf-local-runner-")
        self.addCleanup(self.temp.cleanup)
        base = Path(self.temp.name)
        self.models = base / "models"
        self.models.mkdir()
        self.model = self.models / "test.Q4_K_M.gguf"
        self.model.write_bytes(b"GGUF" + struct.pack("<IQQ", 3, 1, 0) + b"unused")
        self.binary = base / "llama-server"
        self.binary.write_text(FAKE_LLAMA_SERVER)
        self.binary.chmod(self.binary.stat().st_mode | stat.S_IXUSR)
        self.runner = gateway.LocalRunner(self.models, self.binary)
        self.addCleanup(self.runner.stop)

    def test_strict_schema_resource_limits_and_gguf_validation(self):
        s = settings()
        self.assertEqual(gateway.valid_settings(s)["context_size"], 4096)
        for bad in (
            {**s, "context_size": 65536},
            {**s, "threads": 500},
            {**s, "temperature": float("nan")},
            {**s, "gpu_layers": True},
            {**s, "additional": 1},
        ):
            with self.assertRaises(gateway.RunnerError):
                gateway.valid_settings(bad)
        self.assertEqual(self.runner.available_models()[0]["version"], 3)
        with self.assertRaises(gateway.RunnerError):
            gateway.check_gguf(self.models, "../test.Q4_K_M.gguf", 100000)
        (self.models / "symlink.gguf").symlink_to(self.model)
        self.assertEqual([m["name"] for m in self.runner.available_models()],
                         ["test.Q4_K_M.gguf"])
        self.model.write_bytes(b"FAKE" + b"\x00" * 24)
        with self.assertRaises(gateway.RunnerError):
            self.runner.start("test.Q4_K_M.gguf", s)

    def test_real_subprocess_http_health_chat_and_shutdown(self):
        self.assertEqual(self.runner.status()["state"], "stopped")
        self.runner.start("test.Q4_K_M.gguf", settings())
        self.assertEqual(self.runner.process.args[self.runner.process.args.index("--host") + 1],
                         "127.0.0.1")
        self.assertNotIn("test-token", " ".join(self.runner.process.args))
        for _ in range(60):
            if self.runner.status()["state"] == "ready":
                break
            time.sleep(0.05)
        self.assertEqual(self.runner.status()["state"], "ready")
        with self.assertRaises(gateway.RunnerError):
            self.runner.start("test.Q4_K_M.gguf", settings())
        self.assertEqual(self.runner.chat("Cześć!")["text"], "REAL_HTTP_MOCK: Cześć!")
        self.runner.stop()
        self.assertEqual(self.runner.status()["state"], "stopped")

    def test_browser_boundary_bearer_origin_host_and_no_method_fallback(self):
        token = "this-is-a-test-session-token"
        server = gateway.OCAFServer(("127.0.0.1", 0),
                                    ROOT / "apps/agents-builder-web",
                                    self.runner, token)
        import threading
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        self.addCleanup(lambda: (server.shutdown(), server.server_close(), worker.join(timeout=3)))
        port = server.server_address[1]
        origin = f"http://127.0.0.1:{port}"
        def call(path, method="GET", data=None, headers=None):
            conn = http.client.HTTPConnection("127.0.0.1", port, timeout=3)
            request_headers = {"Authorization": "Bearer " + token}
            request_headers.update(headers or {})
            raw = json.dumps(data).encode() if data is not None else None
            if raw is not None:
                request_headers["Content-Type"] = "application/json"
            conn.request(method, path, raw, request_headers)
            response = conn.getresponse()
            result = response.status, response.read()
            conn.close()
            return result
        self.assertEqual(call("/v1/local-runner/status")[0], 200)
        self.assertEqual(call("/v1/local-runner/models")[0], 200)
        self.assertEqual(call("/v1/local-runner/status", headers={
            "Authorization": "Bearer wrong"})[0], 401)
        self.assertEqual(call("/v1/local-runner/status", headers={
            "Host": "evil.example"})[0], 403)
        self.assertEqual(call("/v1/local-runner/start", "POST",
                              {"model": "test.Q4_K_M.gguf", "settings": settings()})[0], 403)
        self.assertEqual(call("/v1/local-runner/start", "POST",
                              {"model": "test.Q4_K_M.gguf", "settings": settings()},
                              {"Origin": "http://evil.example"})[0], 403)
        self.assertEqual(call("/v1/local-runner/start", "POST",
                              {"model": "test.Q4_K_M.gguf", "settings": settings()},
                              {"Origin": origin})[0], 200)
        self.assertEqual(call("/../some-private-file")[0], 404)
        self.assertEqual(call("/v1/local-runner/stop", "POST", {},
                              {"Origin": origin})[0], 200)
        self.assertEqual(call("/")[0], 200)
        self.assertEqual(call("/v1/local-runner/stop", "OPTIONS")[0], 405)

    def test_streamed_binary_model_import_is_atomic_and_never_overwrites(self):
        import io
        import hashlib
        payload = b"GGUF" + struct.pack("<IQQ", 3, 1, 0) + b"data"
        result = self.runner.import_model("downloaded.gguf", io.BytesIO(payload), len(payload))
        self.assertEqual(result["sha256"], hashlib.sha256(payload).hexdigest())
        self.assertEqual((self.models / "downloaded.gguf").read_bytes(), payload)
        with self.assertRaisesRegex(gateway.RunnerError, "already exists"):
            self.runner.import_model("downloaded.gguf", io.BytesIO(payload), len(payload))
        with self.assertRaises(gateway.RunnerError):
            self.runner.import_model("../escaped.gguf", io.BytesIO(payload), len(payload))
        with self.assertRaisesRegex(gateway.RunnerError, "truncated upload"):
            self.runner.import_model("truncated.gguf", io.BytesIO(payload[:4]), len(payload))
        self.assertFalse((self.models / "truncated.gguf").exists())
        with self.assertRaisesRegex(gateway.RunnerError, "invalid GGUF upload"):
            self.runner.import_model("invalid.gguf", io.BytesIO(b"FAKE" + payload[4:]),
                                     len(payload))
        self.assertFalse((self.models / "invalid.gguf").exists())
        self.assertFalse(list(self.models.glob(".ocaf-gguf-upload-*")))

    def test_binary_upload_http_requires_token_and_same_origin(self):
        import http.client
        import threading
        token = "binary-upload-test-session-token"
        server = gateway.OCAFServer(("127.0.0.1", 0),
                                    ROOT / "apps/agents-builder-web",
                                    self.runner, token)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        self.addCleanup(lambda: (server.shutdown(), server.server_close(), worker.join(timeout=3)))
        port = server.server_address[1]
        origin = f"http://127.0.0.1:{port}"
        payload = b"GGUF" + struct.pack("<IQQ", 3, 1, 0) + b"payload"
        def upload(auth, req_origin, filename):
            conn = http.client.HTTPConnection("127.0.0.1", port, timeout=4)
            conn.request("POST", "/v1/local-runner/import", payload, {
                "Authorization": auth,
                "Origin": req_origin,
                "Content-Type": "application/octet-stream",
                "X-OCAF-Model-Name": filename,
            })
            result = conn.getresponse()
            status, content = result.status, result.read()
            conn.close()
            return status, content
        self.assertEqual(upload("Bearer wrong", origin, "safe.gguf")[0], 401)
        self.assertEqual(upload("Bearer " + token, "http://evil.example", "safe.gguf")[0], 403)
        result = upload("Bearer " + token, origin, "safe.gguf")
        self.assertEqual(result[0], 201)
        self.assertEqual((self.models / "safe.gguf").read_bytes(), payload)
        self.assertEqual(upload("Bearer " + token, origin, "safe.gguf")[0], 409)



if __name__ == "__main__":
    unittest.main()
