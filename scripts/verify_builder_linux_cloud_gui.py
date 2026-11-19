#!/usr/bin/env python3
"""Linux native GUI -> restricted Tauri mTLS -> real Rust Cloud/SQLite.
Run only from builder_cloud_full_stack with ephemeral operator fixture identity.
"""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
from verify_builder_linux_local_gui import Driver, free_port, require, isolated_downloads, export_project

def run(binary, driver):
    binary = Path(binary).resolve(strict=True)
    driver = Path(driver).resolve(strict=True)
    token = os.environ.pop("OCAF_TEST_GUI_TOKEN")
    foreign = os.environ.pop("OCAF_TEST_GUI_FOREIGN_TOKEN")
    if not all((binary.is_file(), driver.is_file(), token, foreign)):
        raise RuntimeError("missing test inputs")
    port, native = free_port(), free_port()
    while port == native:
        native = free_port()
    with tempfile.TemporaryDirectory(prefix="ocaf-cloud-gui-") as tmp:
        root = Path(tmp)
        env = dict(os.environ)
        for name, subdir in [("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config"),
                             ("XDG_CACHE_HOME", "cache")]:
            p = root / subdir
            p.mkdir(mode=0o700)
            env[name] = str(p)
        downloads = isolated_downloads(root, env)
        with (root / "webdriver.log").open("wb") as log:
            proc = subprocess.Popen([str(driver), "--port", str(port),
                                     "--native-port", str(native), "--native-host", "127.0.0.1"],
                                    env=env, stdout=log, stderr=log, start_new_session=True)
            c = Driver(port)
            try:
                c.wait_for(lambda: proc.poll() is None and c.request("GET", "/status") is not None, "driver ready")
                c.start(binary)
                c.wait_for(lambda: c.read("project-title") == "demo", "editor loaded")
                c.fill("project-id", "native-local")
                c.click("new-project")
                c.wait_for(lambda: c.read("project-title") == "native-local", "local workspace")
                c.fill("node-id", "offline_transform")
                c.click("add-transform")
                c.wait_for(lambda: c.read("revision-label") == "r2", "local revision")
                c.click("workspace-cloud")
                def connect(tenant, credential):
                    c.fill("cloud-tenant", tenant)
                    c.fill("cloud-token", credential)
                    c.click("cloud-connect")
                    c.wait_for(lambda: "sesja w pamięci" in c.read("cloud-status"), "session configured")
                    require(c.read("cloud-token", "value") == "", "token field not cleared")
                connect("another", foreign)
                c.fill("project-id", "hello")
                c.click("cloud-open")
                c.wait_for(lambda: "403" in c.read("message"), "foreign token rejected")
                require(c.read("project-title") == "Brak projektu", "foreign tenant loaded")
                c.click("cloud-disconnect")
                connect("local", token)
                c.fill("project-id", "native-cloud")
                c.click("cloud-new")
                c.wait_for(lambda: c.read("project-title") == "native-cloud" and "szkic" in c.read("revision-label"), "authenticated new")
                c.fill("node-id", "cloud_transform")
                c.click("add-transform")
                c.wait_for(lambda: "cloud_transform" in c.read("graph"), "Cloud NoCode")
                c.click("preview")
                c.wait_for(lambda: "CLOUD VALIDATION" in c.read("preview-output") and
                           "cloud_transform" in c.read("preview-output"), "authenticated Rust compiler")
                c.click("cloud-save")
                c.wait_for(lambda: "potwierdził zapis CAS r1" in c.read("message"), "first durable CAS")
                original, original_data = export_project(c, downloads, "native-cloud", 1)
                c.upload(original)
                c.wait_for(lambda: "szkic" in c.read("revision-label"), "Cloud NoCode import")
                c.click("preview")
                c.wait_for(lambda: "CLOUD VALIDATION" in c.read("preview-output"), "NoCode imported compile")
                c.click("mode-code")
                graph = json.loads(c.read("dsl", "value"))
                require(graph["tenant_id"] == "local" and graph["project_id"] == "native-cloud", "wrong graph identity")
                graph["max_cost_minor"] = 222
                c.fill("dsl", json.dumps(graph, separators=(",", ":")))
                c.click("apply-code")
                c.wait_for(lambda: "r2" in c.read("revision-label"), "Cloud Code draft")
                exported, data = export_project(c, downloads, "native-cloud", 2)
                bad = root / "foreign.json"
                data["tenant_id"] = "another"
                bad.write_text(json.dumps(data))
                c.upload(bad)
                c.wait_for(lambda: "tenant" in c.read("message").lower(), "foreign import denied")
                c.upload(exported)
                c.wait_for(lambda: "Import" in c.read("message"), "Cloud Code import")
                c.click("preview")
                c.wait_for(lambda: "CLOUD VALIDATION" in c.read("preview-output"), "Code imported compile")
                c.click("cloud-save")
                c.wait_for(lambda: "potwierdził zapis CAS r2" in c.read("message"), "second durable CAS")
                c.click("mode-nocode")
                c.wait_for(lambda: c.read("cost-budget", "value") == "222", "Cloud roundtrip")
                c.click("workspace-local")
                c.wait_for(lambda: c.read("project-title") == "native-local" and
                           c.read("revision-label") == "r2" and "offline_transform" in c.read("graph"), "local isolation")
                c.close()
                c.start(binary)
                c.wait_for(lambda: c.read("project-title") == "demo", "native restart")
                c.click("workspace-cloud")
                connect("local", token)
                c.fill("project-id", "native-cloud")
                c.click("cloud-open")
                c.wait_for(lambda: c.read("project-title") == "native-cloud" and
                           c.read("revision-label") == "r2" and c.read("cost-budget", "value") == "222", "Cloud restart recovery")
                c.click("mode-code")
                restored = json.loads(c.read("dsl", "value"))
                require(restored["max_cost_minor"] == 222 and restored["nodes"] == graph["nodes"] and
                        restored["edges"] == graph["edges"], "Cloud data lost after restart")
                c.fill("project-id", "native-cloud-code")
                c.click("cloud-new")
                c.wait_for(lambda: c.read("project-title") == "native-cloud-code" and
                           "szkic" in c.read("revision-label"), "create Cloud in Code mode")
                code = json.loads(c.read("dsl", "value"))
                code["max_cost_minor"] = 333
                c.fill("dsl", json.dumps(code))
                c.click("apply-code")
                c.click("preview")
                c.wait_for(lambda: "CLOUD VALIDATION" in c.read("preview-output"), "new Cloud Code compile")
                code_export, _ = export_project(c, downloads, "native-cloud-code", 1)
                c.upload(code_export)
                c.wait_for(lambda: "Import" in c.read("message"), "new Cloud Code import")
                c.click("cloud-save")
                c.wait_for(lambda: "potwierdził zapis CAS r1" in c.read("message"), "new Cloud Code durable CAS")
                print(json.dumps({"test": "AB-LINUX-NATIVE-CLOUD-GUI", "result": "PASS",
                                 "binary_sha256": hashlib.file_digest(binary.open("rb"), "sha256").hexdigest(),
                                 "mtls_oauth": True, "foreign_tenant_denied": True,
                                 "code_nocode_cas": True, "local_isolation": True, "restart_recovery": True,
                                 "code_nocode_export_import": True, "foreign_import_denied": True,
                                 "signed_release": False}, sort_keys=True), flush=True)
            finally:
                try:
                    c.close()
                finally:
                    if proc.poll() is None:
                        os.killpg(proc.pid, signal.SIGTERM)
                        try:
                            proc.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            os.killpg(proc.pid, signal.SIGKILL)
                            proc.wait(timeout=5)
if __name__ == "__main__":
    run(os.environ["OCAF_LINUX_GUI_BINARY"], os.environ["OCAF_LINUX_GUI_DRIVER"])
