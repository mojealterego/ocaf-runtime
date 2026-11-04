#!/usr/bin/env python3
"""OCAF Agents Builder localhost-only GGUF runner via installed llama.cpp.

This service is NOT a cloud multi-tenant gateway. It serves the existing UI on
127.0.0.1 and brokers an operator-approved local llama-server subprocess.
No remote URLs, shell commands, tool execution, credential persistence or model
downloads are accepted by the control API. Python 3.10+; stdlib only.
"""
from __future__ import annotations

import argparse
import hashlib
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
import stat
import struct
import subprocess
import sys
import tempfile
import threading
from typing import Any
from urllib.parse import urlsplit

MODEL_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_. -]{0,126}\.gguf$", re.I)
STATIC = frozenset({
    "index.html", "style.css", "app.mjs", "editor.mjs", "cloud.mjs",
    "i18n.mjs", "credentials.mjs", "models.mjs", "integrations-ui.mjs",
    "runner-client.mjs", "manifest.webmanifest", "sw.js",
})
MIME = {
    ".html": "text/html; charset=utf-8",
    ".mjs": "text/javascript; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".webmanifest": "application/manifest+json",
}
MAX_REQUEST = 32768
MAX_RESPONSE = 512 * 1024
MAX_PROMPT = 12000


class RunnerError(Exception):
    def __init__(self, status: int, message: str):
        super().__init__(message)
        self.status = status


def valid_settings(settings: Any, *, context_limit: int = 8192,
                   thread_limit: int = 16, batch_limit: int = 2048) -> dict:
    """Fail-closed counterpart to the strict model settings v1 schema."""
    bounds = {
        "context_size": (256, min(131072, context_limit), int),
        "threads": (1, min(128, thread_limit), int),
        "batch_size": (1, min(8192, batch_limit), int),
        "gpu_layers": (0, 2048, int),
        "temperature": (0, 2, (int, float)),
        "top_p": (0, 1, (int, float)),
        "top_k": (1, 512, int),
        "min_p": (0, 1, (int, float)),
        "repeat_penalty": (0.5, 3, (int, float)),
        "presence_penalty": (-2, 2, (int, float)),
        "frequency_penalty": (-2, 2, (int, float)),
        "seed": (-1, 2147483647, int),
        "max_output_tokens": (1, 32768, int),
        "flash_attention": (False, True, bool),
        "use_mmap": (False, True, bool),
    }
    if type(settings) is not dict or set(settings) != set(bounds):
        raise RunnerError(422, "settings: invalid or incomplete profile")
    for key, (lo, hi, accepted) in bounds.items():
        value = settings[key]
        if type(value) is bool and accepted is not bool:
            raise RunnerError(422, "settings: boolean is not a number")
        if not isinstance(value, accepted):
            raise RunnerError(422, "settings: invalid " + key)
        if isinstance(value, float):
            if not __import__("math").isfinite(value):
                raise RunnerError(422, "settings: non-finite value")
        if not lo <= value <= hi:
            raise RunnerError(422, "settings: out of range " + key)
    if settings["max_output_tokens"] > settings["context_size"]:
        raise RunnerError(422, "settings: output exceeds context")
    return dict(settings)


def check_gguf(root: Path, filename: str, max_bytes: int) -> dict:
    """Inspect a regular GGUF file in the operator-controlled models directory.

    The model directory and file cannot be symlinks. No untrusted file paths
    are accepted from HTTP clients, only a basename already in the catalog.
    """
    if not isinstance(filename, str) or not MODEL_NAME.fullmatch(filename):
        raise RunnerError(422, "model: invalid GGUF filename")
    path = root / filename
    try:
        if path.is_symlink() or path.resolve(strict=True).parent != root:
            raise RunnerError(422, "model: symlink or escaped model directory")
        with path.open("rb") as handle:
            metadata = os.fstat(handle.fileno())
            if not stat.S_ISREG(metadata.st_mode) or not 24 <= metadata.st_size <= max_bytes:
                raise RunnerError(422, "model: invalid file size or type")
            header = handle.read(24)
    except (OSError, RuntimeError) as exc:
        raise RunnerError(422, "model: file inaccessible") from exc
    if header[:4] != b"GGUF":
        raise RunnerError(422, "model: GGUF magic mismatch")
    version, tensors, metadata_items = struct.unpack_from("<IQQ", header, 4)
    if version not in (2, 3) or tensors > 10_000_000 or metadata_items > 65_536:
        raise RunnerError(422, "model: unsupported GGUF header")
    return {"name": filename, "size_bytes": metadata.st_size,
            "version": version, "tensor_count": tensors}


class LocalRunner:
    def __init__(self, models_dir: Path, llama_bin: Path, *,
                 max_model_bytes: int = 8 * 1024**3, context_limit: int = 8192,
                 thread_limit: int = 16, batch_limit: int = 2048):
        self.models_dir = models_dir.resolve(strict=True)
        if not self.models_dir.is_dir() or models_dir.is_symlink():
            raise ValueError("models-dir must be a real directory, not a symlink")
        self.llama_bin = llama_bin.resolve(strict=True)
        if not self.llama_bin.is_file() or not os.access(self.llama_bin, os.X_OK):
            raise ValueError("llama-server must be an executable regular file")
        if not (24 <= max_model_bytes <= 100 * 1024**3):
            raise ValueError("invalid model size limit")
        self.max_model_bytes = max_model_bytes
        self.context_limit = context_limit
        self.thread_limit = thread_limit
        self.batch_limit = batch_limit
        self.process: subprocess.Popen | None = None
        self.model: str | None = None
        self.settings: dict | None = None
        self.backend_port: int | None = None
        self.backend_key: str | None = None
        self.lock = threading.RLock()
        self.request_slot = threading.BoundedSemaphore(1)

    def available_models(self) -> list[dict]:
        models = []
        for path in sorted(self.models_dir.iterdir(), key=lambda p: p.name):
            if not MODEL_NAME.fullmatch(path.name):
                continue
            try:
                models.append(check_gguf(self.models_dir, path.name, self.max_model_bytes))
            except RunnerError:
                continue
        return models[:200]

    def import_model(self, filename: str, source, length: int) -> dict:
        """Stream an operator-selected GGUF into models-dir without ever
        overwriting a model. Keep memory use bounded to 1 MiB.
        """
        if not isinstance(filename, str) or not MODEL_NAME.fullmatch(filename):
            raise RunnerError(422, "model: invalid GGUF filename")
        if type(length) is not int or not 24 <= length <= self.max_model_bytes:
            raise RunnerError(413, "model: upload exceeds configured limit")
        destination = self.models_dir / filename
        if destination.exists() or destination.is_symlink():
            raise RunnerError(409, "model: file already exists")
        fd, temporary = tempfile.mkstemp(prefix=".ocaf-gguf-upload-",
                                        suffix=".tmp", dir=self.models_dir)
        hash_state = hashlib.sha256()
        try:
            remaining = length
            with os.fdopen(fd, "wb") as output:
                while remaining:
                    chunk = source.read(min(1024 * 1024, remaining))
                    if not chunk:
                        raise RunnerError(422, "model: truncated upload")
                    remaining -= len(chunk)
                    output.write(chunk)
                    hash_state.update(chunk)
                output.flush()
                os.fsync(output.fileno())
            with open(temporary, "rb") as uploaded:
                header = uploaded.read(24)
            # Validate before publishing an archive, never leave an invalid
            # GGUF visible in the active model directory.
            if header[:4] != b"GGUF" or len(header) != 24:
                raise RunnerError(422, "model: invalid GGUF upload")
            version, tensors, metadata = struct.unpack_from("<IQQ", header, 4)
            if version not in (2, 3) or tensors > 10_000_000 or metadata > 65_536:
                raise RunnerError(422, "model: unsupported GGUF upload")
            try:
                os.link(temporary, destination, follow_symlinks=False)
            except FileExistsError as error:
                raise RunnerError(409, "model: concurrent upload or existing file") from error
            return {"name": filename, "size_bytes": length,
                    "sha256": hash_state.hexdigest(), "version": version}
        except OSError as error:
            raise RunnerError(507, "model: local storage unavailable") from error
        finally:
            os.unlink(temporary)

    def status(self) -> dict:
        with self.lock:
            proc, port = self.process, self.backend_port
            model = self.model
            if not proc:
                return {"state": "stopped", "model": None}
            result = proc.poll()
            if result is not None:
                return {"state": "failed", "model": model, "exit_code": result}
        try:
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=0.7)
            try:
                connection.request("GET", "/health")
                result = connection.getresponse()
                result.read(4096)
                ready = result.status == 200
            finally:
                connection.close()
            return {"state": "ready" if ready else "loading", "model": model}
        except (OSError, http.client.HTTPException):
            return {"state": "loading", "model": model}

    def start(self, filename: str, settings: dict) -> dict:
        validated = valid_settings(settings, context_limit=self.context_limit,
                                   thread_limit=self.thread_limit,
                                   batch_limit=self.batch_limit)
        model = check_gguf(self.models_dir, filename, self.max_model_bytes)
        with self.lock:
            if self.process and self.process.poll() is None:
                raise RunnerError(409, "runner: already running; stop it first")
            if self.process:
                self._stop_locked()
            # The backend is private; the browser never talks to its port.
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
                listener.bind(("127.0.0.1", 0))
                port = listener.getsockname()[1]
            key = secrets.token_urlsafe(36)
            env = {name: os.environ[name] for name in (
                "PATH", "HOME", "LANG", "LC_ALL", "TMPDIR", "PREFIX"
            ) if name in os.environ}
            env["LLAMA_API_KEY"] = key
            args = [
                str(self.llama_bin), "--model", str(self.models_dir / filename),
                "--host", "127.0.0.1", "--port", str(port),
                "--no-webui", "--no-slots",
                "--ctx-size", str(validated["context_size"]),
                "--threads", str(validated["threads"]),
                "--batch-size", str(validated["batch_size"]),
                "--n-gpu-layers", str(validated["gpu_layers"]),
                "--flash-attn", "on" if validated["flash_attention"] else "off",
                "--load-mode", "mmap" if validated["use_mmap"] else "none",
            ]
            try:
                proc = subprocess.Popen(
                    args, env=env, stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                    start_new_session=True, close_fds=True,
                )
            except OSError as exc:
                raise RunnerError(503, "runner: cannot start llama-server") from exc
            self.process = proc
            self.backend_port = port
            self.backend_key = key
            self.model = model["name"]
            self.settings = validated
        return {"state": "loading", "model": filename}

    def _stop_locked(self) -> None:
        proc = self.process
        self.process = None
        self.model = None
        self.backend_port = None
        self.backend_key = None
        self.settings = None
        if proc and proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=4)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=4)

    def stop(self) -> dict:
        with self.lock:
            self._stop_locked()
        return {"state": "stopped", "model": None}

    def chat(self, prompt: str) -> dict:
        if not isinstance(prompt, str) or not 1 <= len(prompt) <= MAX_PROMPT:
            raise RunnerError(422, "chat: prompt length invalid")
        if not self.request_slot.acquire(blocking=False):
            raise RunnerError(429, "chat: one inference at a time")
        try:
            if self.status()["state"] != "ready":
                raise RunnerError(409, "chat: runner not ready")
            with self.lock:
                settings = self.settings.copy()
                port = self.backend_port
                key = self.backend_key
            body = {
                "model": self.model,
                "messages": [{"role": "user", "content": prompt}],
                "stream": False, "max_tokens": settings["max_output_tokens"],
                "temperature": settings["temperature"],
                "top_p": settings["top_p"], "top_k": settings["top_k"],
                "min_p": settings["min_p"],
                "repeat_penalty": settings["repeat_penalty"],
                "presence_penalty": settings["presence_penalty"],
                "frequency_penalty": settings["frequency_penalty"],
            }
            if settings["seed"] >= 0:
                body["seed"] = settings["seed"]
            payload = json.dumps(body, ensure_ascii=False).encode("utf-8")
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=180)
            try:
                connection.request("POST", "/v1/chat/completions", payload, {
                    "Authorization": "Bearer " + key,
                    "Content-Type": "application/json",
                    "Accept": "application/json",
                })
                response = connection.getresponse()
                raw = response.read(MAX_RESPONSE + 1)
                if len(raw) > MAX_RESPONSE:
                    raise RunnerError(502, "chat: backend response too large")
                if response.status != 200:
                    raise RunnerError(502, "chat: backend inference rejected")
            except (OSError, http.client.HTTPException) as exc:
                raise RunnerError(503, "chat: local backend unavailable") from exc
            finally:
                connection.close()
            try:
                result = json.loads(raw)
                content = result["choices"][0]["message"]["content"]
                if not isinstance(content, str) or len(content) > 200_000:
                    raise ValueError("invalid assistant response")
            except (KeyError, IndexError, TypeError, ValueError) as exc:
                raise RunnerError(502, "chat: invalid backend response") from exc
            return {"text": content, "model": self.model}
        finally:
            self.request_slot.release()


class OCAFServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = False

    def __init__(self, addr, web_root: Path, runner: LocalRunner, token: str):
        self.web_root = web_root.resolve(strict=True)
        self.runner = runner
        self.token = token
        super().__init__(addr, LocalHandler)

    def server_close(self):
        try:
            self.runner.stop()
        finally:
            super().server_close()


class LocalHandler(BaseHTTPRequestHandler):
    server: OCAFServer
    protocol_version = "HTTP/1.1"
    timeout = 12

    def log_message(self, fmt, *args):
        # Never log API key, prompt, Authorization headers or filesystem paths.
        pass

    def send_bytes(self, status: int, content: bytes, mime: str):
        self.send_response(status)
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(content)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Referrer-Policy", "no-referrer")
        self.send_header("Cross-Origin-Resource-Policy", "same-origin")
        self.send_header("X-Frame-Options", "DENY")
        self.send_header("Connection", "close")
        self.close_connection = True
        self.end_headers()
        self.wfile.write(content)

    def json_response(self, status: int, payload: dict):
        self.send_bytes(status, json.dumps(payload, ensure_ascii=False).encode("utf-8"),
                        "application/json; charset=utf-8")

    def reject(self, err: RunnerError):
        self.json_response(err.status, {"error": str(err)})

    def allowed_host(self) -> bool:
        port = self.server.server_address[1]
        host = self.headers.get("Host", "")
        return host in (f"127.0.0.1:{port}", f"localhost:{port}")

    def expected_origin(self) -> str:
        return f"http://{self.headers.get('Host')}"

    def authorized(self, write=False) -> None:
        if not self.allowed_host():
            raise RunnerError(403, "request: invalid Host header")
        origin = self.headers.get("Origin")
        if origin is not None and origin != self.expected_origin():
            raise RunnerError(403, "request: cross-origin denied")
        if write and origin != self.expected_origin():
            raise RunnerError(403, "request: Origin required")
        authorization = self.headers.get("Authorization", "")
        expected = "Bearer " + self.server.token
        if not secrets.compare_digest(authorization, expected):
            raise RunnerError(401, "request: invalid local session token")

    def route(self) -> str:
        path = urlsplit(self.path)
        if path.query or path.fragment or not path.path.startswith("/"):
            raise RunnerError(400, "request: invalid path")
        return path.path

    def do_GET(self):
        try:
            if not self.allowed_host():
                raise RunnerError(403, "request: invalid Host header")
            path = self.route()
            if path.startswith("/v1/local-runner/"):
                self.authorized()
                if path == "/v1/local-runner/models":
                    self.json_response(200, {"models": self.server.runner.available_models()})
                elif path == "/v1/local-runner/status":
                    self.json_response(200, self.server.runner.status())
                else:
                    raise RunnerError(404, "not found")
                return
            name = "index.html" if path == "/" else path.removeprefix("/")
            if name not in STATIC:
                raise RunnerError(404, "not found")
            data = (self.server.web_root / name).read_bytes()
            self.send_bytes(200, data, MIME.get(Path(name).suffix, "application/octet-stream"))
        except RunnerError as err:
            self.reject(err)
        except OSError:
            self.reject(RunnerError(404, "not found"))

    def do_POST(self):
        try:
            self.authorized(write=True)
            path = self.route()
            if path not in ("/v1/local-runner/start", "/v1/local-runner/chat",
                            "/v1/local-runner/stop", "/v1/local-runner/import"):
                raise RunnerError(404, "not found")
            mime = self.headers.get("Content-Type", "").split(";")[0].strip()
            if path == "/v1/local-runner/import":
                if mime != "application/octet-stream":
                    raise RunnerError(415, "request: binary GGUF required")
                try:
                    size = int(self.headers.get("Content-Length", "0"))
                except ValueError as error:
                    raise RunnerError(400, "request: invalid upload length") from error
                name = self.headers.get("X-OCAF-Model-Name", "")
                result = self.server.runner.import_model(name, self.rfile, size)
                self.json_response(201, result)
                return
            if mime != "application/json":
                raise RunnerError(415, "request: application/json required")
            try:
                n = int(self.headers.get("Content-Length", "0"))
            except ValueError as exc:
                raise RunnerError(400, "request: invalid length") from exc
            if not 0 < n <= MAX_REQUEST:
                raise RunnerError(413, "request: payload too large")
            try:
                body = json.loads(self.rfile.read(n))
            except (json.JSONDecodeError, UnicodeDecodeError) as exc:
                raise RunnerError(400, "request: invalid JSON") from exc
            if type(body) is not dict:
                raise RunnerError(422, "request: expected object")
            if path == "/v1/local-runner/start":
                if set(body) != {"model", "settings"}:
                    raise RunnerError(422, "runner: unexpected arguments")
                result = self.server.runner.start(body["model"], body["settings"])
            elif path == "/v1/local-runner/chat":
                if set(body) != {"prompt"}:
                    raise RunnerError(422, "chat: unexpected arguments")
                result = self.server.runner.chat(body["prompt"])
            else:
                if body:
                    raise RunnerError(422, "runner: stop does not accept arguments")
                result = self.server.runner.stop()
            self.json_response(200, result)
        except RunnerError as err:
            self.reject(err)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_OPTIONS(self):
        self.reject(RunnerError(405, "request: CORS preflight denied"))


def main() -> int:
    parser = argparse.ArgumentParser(description="OCAF local GGUF inference gateway")
    parser.add_argument("--models-dir", required=True, type=Path)
    parser.add_argument("--llama-bin", type=Path,
                        default=Path(shutil.which("llama-server") or "llama-server"))
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--max-model-gib", type=int, default=8)
    parser.add_argument("--max-context", type=int, default=8192)
    parser.add_argument("--max-threads", type=int, default=16)
    parser.add_argument("--max-batch", type=int, default=2048)
    args = parser.parse_args()
    if not (1024 <= args.port <= 65535 and 1 <= args.max_model_gib <= 100
            and 256 <= args.max_context <= 131072
            and 1 <= args.max_threads <= 128 and 1 <= args.max_batch <= 8192):
        parser.error("invalid resource limits")
    try:
        runner = LocalRunner(
            args.models_dir, args.llama_bin,
            max_model_bytes=args.max_model_gib * 1024**3,
            context_limit=args.max_context, thread_limit=args.max_threads,
            batch_limit=args.max_batch,
        )
        token = secrets.token_urlsafe(36)
        app = OCAFServer(("127.0.0.1", args.port),
                         Path(__file__).resolve().parent, runner, token)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    print(f"OCAF Local GGUF: http://127.0.0.1:{args.port}/", flush=True)
    print("Session token (paste only into the local runner panel): " + token, flush=True)
    print("Models directory: " + str(runner.models_dir), flush=True)
    try:
        app.serve_forever(poll_interval=0.3)
    except KeyboardInterrupt:
        pass
    finally:
        app.shutdown()
        app.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
