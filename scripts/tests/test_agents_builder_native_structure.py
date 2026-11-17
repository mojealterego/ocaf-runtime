"""Static contracts for OCAF native builder sources, NOT target build/E2E proof."""
import json
from pathlib import Path
import unittest
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[2]


def source(path: str) -> str:
    file = ROOT / path
    if not file.is_file():
        raise AssertionError(f"Missing native source: {path}")
    return file.read_text(encoding="utf-8")


class NativeBuilderSourceContracts(unittest.TestCase):
    def test_tauri_hosts_shared_frontend_without_privileged_plugins(self):
        cfg = json.loads(source("apps/desktop/src-tauri/tauri.conf.json"))
        assets = (ROOT / "apps/desktop/src-tauri" / cfg["build"]["frontendDist"]).resolve()
        self.assertEqual(assets, (ROOT / "apps/agents-builder-web").resolve())
        self.assertTrue((assets / "index.html").is_file())
        permissions = json.loads(source("apps/desktop/src-tauri/capabilities/default.json"))
        self.assertEqual(permissions["permissions"], ["core:default"])
        host = source("apps/desktop/src-tauri/src/main.rs")
        self.assertIn("tauri::Builder::default()", host)
        self.assertNotIn("std::process::Command", host)

    def test_android_bundles_common_ui_and_enforces_origin_and_saf(self):
        gradle = source("apps/android/app/build.gradle.kts")
        self.assertIn('from("../../agents-builder-web")', gradle)
        self.assertIn('dependsOn(copySharedBuilderUi)', gradle)
        self.assertIn('dependsOn(generateKotlinFfi, buildAndroidFfi)', gradle)
        manifest = ElementTree.fromstring(source("apps/android/app/src/main/AndroidManifest.xml"))
        ns = "{http://schemas.android.com/apk/res/android}"
        self.assertFalse(any(e.attrib.get(ns + "name") == "android.permission.INTERNET"
                             for e in manifest.findall("uses-permission")))
        native = source("apps/android/app/src/main/java/com/ocaf/builder/MainActivity.kt")
        for term in ("WebViewAssetLoader", "setOf(\"https://appassets.androidplatform.net\")",
                     "ActivityResultContracts.GetContent", "ActivityResultContracts.CreateDocument",
                     "validateLocalJsonDsl(dataString)", "MIXED_CONTENT_NEVER_ALLOW"):
            self.assertIn(term, native)
        self.assertNotIn("addJavascriptInterface(", native)

    def test_ios_has_local_only_navigation_native_export_and_ffi(self):
        spec = source("apps/ios/project.yml")
        self.assertIn("buildPhase: resources", spec)
        self.assertIn("OTHER_SWIFT_FLAGS:", spec)
        self.assertIn("OTHER_LDFLAGS:", spec)
        self.assertIn("libocaf_builder_mobile_ffi.a", spec)
        app = source("apps/ios/Sources/OCAFBuilderApp.swift")
        for term in ("WKWebView", "url.isFileURL", "WKScriptMessageHandler",
                     "validateLocalJsonDsl(source: content)", "UIActivityViewController"):
            self.assertIn(term, app)
        build = source("apps/ios/build.sh")
        self.assertIn("rustup target list --installed", build)
        self.assertIn("xcodebuild", build)

    def test_android_and_ios_native_export_is_rust_validated_before_file_write(self):
        kotlin = source("apps/android/app/src/main/java/com/ocaf/builder/MainActivity.kt")
        export = kotlin.split('"export" -> {', 1)[1].split('"validate" -> {', 1)[0]
        self.assertIn("validateLocalJsonDsl(dataString)", export)
        self.assertIn("return@addWebMessageListener", export)
        self.assertLess(export.index("validateLocalJsonDsl(dataString)"),
                        export.index("exportLauncher.launch"))

        swift = source("apps/ios/Sources/OCAFBuilderApp.swift")
        export = swift.split('guard payload["action"] as? String == "export"', 1)[1]
        self.assertIn("validateLocalJsonDsl(source: content)", export)
        self.assertLess(export.index("validateLocalJsonDsl(source: content)"),
                        export.index("data.write(to: temporary)"))

    def test_mobile_ffi_consumes_same_canonical_rust_core(self):
        bridge = source("crates/ocaf-builder-mobile-ffi/src/lib.rs")
        self.assertIn('#[path = "../../../src/builder_core.rs"]', bridge)
        self.assertIn("uniffi::setup_scaffolding!();", bridge)
        self.assertIn('from_dsl(&source, "local")', bridge)
        self.assertIn('"effects_executed": false', bridge)
        self.assertIn("pub fn canonical_local_json_dsl", bridge)

    def test_web_ui_keeps_real_browser_export_and_native_bridges(self):
        app = source("apps/agents-builder-web/app.mjs")
        self.assertIn("URL.createObjectURL(blob)", app)
        self.assertIn("window.ocafHost.postMessage", app)
        self.assertIn("window.webkit.messageHandlers.ocafHost.postMessage", app)
        self.assertIn("ocaf-rust-validation", app)
        self.assertIn("encodeDsl(state.project, TENANT)", app)
        sw = source("apps/agents-builder-web/sw.js")
        self.assertIn("ocaf-builder-web-v8", sw)
        self.assertIn("./preview-gate.mjs", sw)
        self.assertIn("previewGate.take(receipt.request_id", app)
        self.assertIn("request_id:requestId", app)
        self.assertIn("invalidatePreview();", app)
        self.assertIn('.put("request_id", requestId)', source("apps/android/app/src/main/java/com/ocaf/builder/MainActivity.kt"))
        self.assertIn('"request_id": requestID', source("apps/ios/Sources/OCAFBuilderApp.swift"))

    def test_ten_products_remain_partial_and_never_claim_production(self):
        matrix = json.loads(source("docs/roadmap/agents_builder_products_v11.json"))
        self.assertEqual(len(matrix["products"]), 10)
        for entry in matrix["products"]:
            self.assertEqual(entry["status"], "PARTIAL")
            self.assertTrue(entry["code_paths_found"])
            self.assertFalse(entry["implemented_verified"])
            self.assertFalse(entry["production_approved"])
            self.assertEqual(entry["evidence"], [])
            for path in entry["code_paths_found"]:
                self.assertTrue((ROOT / path).is_file(), path)
        cert = json.loads(source("release/CERTIFICATE_RECORD.json"))
        self.assertEqual(cert["overall_gate_status"], "NIEPOTWIERDZONE")
        self.assertFalse(cert["production_approved"])


if __name__ == "__main__":
    unittest.main()
