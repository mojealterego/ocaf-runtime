#!/usr/bin/env python3
"""Native Tauri Linux GUI acceptance for the local Code/NoCode round trip.

Run inside Xvfb, with tauri-driver 2.1.0 and the system WebKitWebDriver.
No external service or synthetic backend; the driver launches the release ELF.
Cloud checks are negative checks only, not authenticated Cloud acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

ELEMENT = "element-6066-11e4-a52e-4f735466cecf"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class Driver:
    def __init__(self, port):
        self.base = f"http://127.0.0.1:{port}"
        self.sid = None
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def request(self, method, path, payload=None):
        data = None if payload is None else json.dumps(payload).encode()
        req = urllib.request.Request(self.base + path, data=data, method=method,
                                     headers={"Content-Type": "application/json"})
        try:
            with self.opener.open(req, timeout=20) as response:
                body = json.load(response)
        except urllib.error.HTTPError as exc:
            raise RuntimeError(f"WebDriver rejected {method} {path}: {exc.code}") from exc
        require("value" in body, "invalid WebDriver response")
        value = body["value"]
        require(not (isinstance(value, dict) and "error" in value),
                "WebDriver command failed")
        return value

    def start(self, binary):
        result = self.request("POST", "/session", {"capabilities": {
            "alwaysMatch": {"tauri:options": {"application": str(binary)}}}})
        self.sid = result["sessionId"]
        require(isinstance(self.sid, str) and self.sid, "missing session ID")

    def call(self, method, path, payload=None):
        require(self.sid is not None, "no WebDriver session")
        return self.request(method, "/session/" + self.sid + path, payload)

    def close(self):
        if self.sid is not None:
            try:
                self.call("DELETE", "")
            finally:
                self.sid = None

    def element(self, ident):
        return self.call("POST", "/element", {"using": "css selector",
                                               "value": "#" + ident})[ELEMENT]

    def click(self, ident):
        self.call("POST", "/element/" + self.element(ident) + "/click", {})

    def fill(self, ident, value):
        path = "/element/" + self.element(ident)
        self.call("POST", path + "/clear", {})
        self.call("POST", path + "/value", {"text": value, "value": list(value)})

    def read(self, ident, prop="textContent"):
        require(prop in {"textContent", "value", "hidden", "disabled"}, "unsupported property")
        return self.call("GET", f"/element/{self.element(ident)}/property/{prop}")

    def wait_for(self, predicate, label, seconds=12):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            try:
                if predicate():
                    return
            except (RuntimeError, OSError):
                pass
            time.sleep(0.15)
        raise RuntimeError(f"GUI assertion timed out: {label}")


def run(binary, driver):
    binary = binary.resolve(strict=True)
    driver = driver.resolve(strict=True)
    require(binary.is_file() and os.access(binary, os.X_OK), "invalid executable")
    require(driver.is_file() and os.access(driver, os.X_OK), "invalid driver")
    port, native = free_port(), free_port()
    while port == native:
        native = free_port()
    with tempfile.TemporaryDirectory(prefix="ocaf-gui-") as directory:
        root = Path(directory)
        env = dict(os.environ)
        for name, subdir in [("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config"),
                             ("XDG_CACHE_HOME", "cache")]:
            p = root / subdir
            p.mkdir(mode=0o700)
            env[name] = str(p)
        # Unique test-local directories prevent writing the operator's projects.
        with (root / "webdriver.log").open("wb") as log:
            process = subprocess.Popen([str(driver), "--port", str(port),
                "--native-port", str(native), "--native-host", "127.0.0.1"],
                env=env, stdout=log, stderr=log, start_new_session=True)
            client = Driver(port)
            try:
                client.wait_for(lambda: process.poll() is None and
                                client.request("GET", "/status") is not None, "driver ready")
                client.start(binary)
                client.wait_for(lambda: client.read("project-title") == "demo", "editor loaded")
                client.fill("project-id", "native-e2e")
                client.click("new-project")
                client.wait_for(lambda: client.read("project-title") == "native-e2e" and
                                client.read("revision-label") == "r1", "new local project")
                client.fill("node-id", "native_transform")
                client.click("add-transform")
                client.wait_for(lambda: client.read("revision-label") == "r2" and
                                "native_transform" in client.read("graph"), "NoCode insertion")
                client.click("mode-code")
                client.wait_for(lambda: client.read("code-view", "hidden") is False, "Code view")
                graph = json.loads(client.read("dsl", "value"))
                require(graph["project_id"] == "native-e2e" and graph["revision"] == 2,
                        "Code/NoCode project identity mismatch")
                require(any(node["id"] == "native_transform" for node in graph["nodes"]),
                        "NoCode node missing in Code DSL")
                graph["max_cost_minor"] = 200
                client.fill("dsl", json.dumps(graph, separators=(",", ":")))
                client.click("apply-code")
                client.wait_for(lambda: client.read("revision-label") == "r3", "Code application")
                client.click("mode-nocode")
                client.wait_for(lambda: client.read("nocode-view", "hidden") is False
                                and client.read("cost-budget", "value") == "200", "round-trip")
                client.click("preview")
                client.wait_for(lambda: "DRY RUN" in client.read("preview-output") and
                                "native_transform" in client.read("preview-output") and
                                "native-e2e r3" in client.read("preview-output"), "local preview")
                client.click("workspace-cloud")
                client.wait_for(lambda: client.read("cloud-controls", "hidden") is False and
                                client.read("cloud-open", "disabled") is True and
                                client.read("cloud-save", "disabled") is True,
                                "no unauthenticated Cloud write")
                client.click("workspace-local")
                client.wait_for(lambda: client.read("project-title") == "native-e2e" and
                                client.read("revision-label") == "r3", "local project retained")
                client.close()
                # A fresh native process must read the on-disk local revision.
                client.start(binary)
                client.wait_for(lambda: client.read("project-title") == "demo", "restarted editor")
                client.fill("project-id", "native-e2e")
                client.click("open-project")
                client.wait_for(lambda: client.read("project-title") == "native-e2e" and
                                client.read("revision-label") == "r3" and
                                client.read("cost-budget", "value") == "200", "restart recovery")
                client.click("mode-code")
                client.wait_for(lambda: client.read("code-view", "hidden") is False, "reopened Code view")
                reopened = json.loads(client.read("dsl", "value"))
                require(reopened["project_id"] == graph["project_id"] and
                        reopened["revision"] == 3 and reopened["nodes"] == graph["nodes"] and
                        reopened["edges"] == graph["edges"] and reopened["max_cost_minor"] == 200,
                        "project differs after native restart")
                return {"test": "AB-LINUX-LOCAL-CODE-NOCODE-E2E", "result": "PASS",
                        "binary_sha256": hashlib.file_digest(binary.open("rb"),"sha256").hexdigest(),
                        "local_code_nocode_roundtrip": True, "persistent_restart": True,
                        "unauthenticated_cloud_disabled": True,
                        "authenticated_cloud": "UNVERIFIED", "platform": "linux-x86_64"}
            finally:
                try:
                    client.close()
                finally:
                    if process.poll() is None:
                        os.killpg(process.pid, signal.SIGTERM)
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            os.killpg(process.pid, signal.SIGKILL)
                            process.wait(timeout=5)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--driver", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(run(args.binary, args.driver), sort_keys=True))
