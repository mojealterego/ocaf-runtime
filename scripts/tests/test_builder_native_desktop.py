"""Source contracts for actual Windows/macOS Tauri v2 entrypoint; no build claims."""
import json
import subprocess
import sys
import tempfile
import struct
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

    def test_generated_windows_icon_is_valid_png_backed_ico(self):
        subprocess.run([sys.executable, str(ROOT / "scripts/generate_builder_icons.py")],
                       check=True)
        icon_dir = DESKTOP / "src-tauri" / "icons"
        png = (icon_dir / "icon.png").read_bytes()
        ico = (icon_dir / "icon.ico").read_bytes()
        self.assertTrue(png.startswith(bytes.fromhex("89504e470d0a1a0a")))
        self.assertEqual(ico[:6], struct.pack("<HHH", 0, 1, 1))
        self.assertEqual(ico[22:], png)

    def test_linux_ci_uses_repository_pinned_rust_toolchain(self):
        workflow = (ROOT / ".github/workflows/agents-builder-native-linux.yml").read_text()
        toolchain = (ROOT / "rust-toolchain.toml").read_text()
        import tomllib
        pinned = tomllib.loads(toolchain)["toolchain"]["channel"]
        self.assertIn("dtolnay/rust-toolchain@", workflow)
        self.assertIn(f"toolchain: '{pinned}'", workflow)

    def test_linux_ci_rebuilds_when_rust_toolchain_changes(self):
        workflow = (ROOT / ".github/workflows/agents-builder-native-linux.yml").read_text()
        self.assertIn('      - "rust-toolchain.toml"', workflow)

    def test_desktop_ci_uses_repository_pinned_rust_toolchain(self):
        import tomllib
        workflow = (ROOT / ".github/workflows/agents-builder-native-desktop.yml").read_text()
        toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())
        pinned = toolchain["toolchain"]["channel"]
        self.assertIn("dtolnay/rust-toolchain@", workflow)
        self.assertIn(f"toolchain: '{pinned}'", workflow)

    def test_desktop_ci_rebuilds_when_rust_toolchain_changes(self):
        workflow = (ROOT / ".github/workflows/agents-builder-native-desktop.yml").read_text()
        self.assertIn('      - "rust-toolchain.toml"', workflow)

    def test_desktop_ci_rebuilds_when_artifact_verifier_changes(self):
        workflow = (ROOT / ".github/workflows/agents-builder-native-desktop.yml").read_text()
        self.assertIn('      - "scripts/verify_native_artifact.py"', workflow)
        self.assertIn('      - "scripts/tests/test_verify_native_artifact.py"', workflow)

    def test_desktop_ci_rebuilds_when_icon_generator_changes(self):
        workflow = (ROOT / ".github/workflows/agents-builder-native-desktop.yml").read_text()
        self.assertIn('      - "scripts/generate_builder_icons.py"', workflow)

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
