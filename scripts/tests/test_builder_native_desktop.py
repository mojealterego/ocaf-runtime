"""Source contracts for actual Windows/macOS Tauri v2 entrypoint; no build claims."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
DESKTOP = ROOT / "apps" / "agents-builder-desktop"
WEB = ROOT / "apps" / "agents-builder-web"


class DesktopNativeSourceTests(unittest.TestCase):
    def test_tauri_v2_uses_shared_code_nocode_web_frontend(self):
        config = json.loads((DESKTOP / "src-tauri" / "tauri.conf.json").read_text())
        self.assertEqual(config["$schema"], "https://schema.tauri.app/config/2")
        self.assertEqual(config["build"]["frontendDist"], "../../agents-builder-web")
        self.assertTrue((WEB / "app.mjs").is_file())
        self.assertTrue((WEB / "index.html").is_file())
        self.assertEqual(config["app"]["windows"][0]["url"], "index.html")

    def test_native_rust_entrypoint_and_restricted_capabilities(self):
        cargo = (DESKTOP / "src-tauri" / "Cargo.toml").read_text()
        main = (DESKTOP / "src-tauri" / "src" / "main.rs").read_text()
        self.assertIn('tauri = { version = "2"', cargo)
        self.assertIn("tauri::generate_context!()", main)
        capabilities = json.loads((DESKTOP / "src-tauri" / "capabilities" / "default.json").read_text())
        self.assertEqual(capabilities["permissions"], ["core:default"])
        self.assertEqual(capabilities["windows"], ["main"])


if __name__ == "__main__":
    unittest.main()
