"""Static contracts for Android Compose and iOS SwiftUI native project sources."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
ANDROID = ROOT / "apps/agents-builder-android"
IOS = ROOT / "apps/agents-builder-ios"


class MobileNativeContracts(unittest.TestCase):
    def test_android_compose_entrypoint_and_offline_editor(self):
        settings = (ANDROID / "settings.gradle.kts").read_text()
        gradle = (ANDROID / "app/build.gradle.kts").read_text()
        main = (ANDROID / "app/src/main/java/org/ocaf/agentsbuilder/MainActivity.kt").read_text()
        self.assertIn('include(":app")', settings)
        self.assertIn("org.jetbrains.kotlin.plugin.compose", gradle)
        self.assertIn("ComponentActivity", main)
        self.assertIn("WebView", main)
        self.assertIn("https://appassets.androidplatform.net/assets/index.html", main)
        self.assertIn("WebViewAssetLoader", main)
        self.assertIn('WebResourceResponse("text/javascript", "UTF-8"', main)
        self.assertIn("android.useAndroidX=true", (ANDROID / "gradle.properties").read_text())
        self.assertIn("setAllowFileAccessFromFileURLs(false)", main)
        self.assertIn("setAllowUniversalAccessFromFileURLs(false)", main)

    def test_ios_swiftui_entrypoint_and_local_webview(self):
        source = (IOS / "Sources/OCAFAgentsBuilder/App.swift").read_text()
        self.assertIn("@main", source)
        self.assertIn("WindowGroup", source)
        self.assertIn("WKWebView", source)
        self.assertIn("loadFileURL", source)
        self.assertIn("allowingReadAccessTo: directory", source)


if __name__ == "__main__":
    unittest.main()
