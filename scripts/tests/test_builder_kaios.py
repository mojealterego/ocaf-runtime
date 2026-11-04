"""Static KaiOS 2.5 packaging and fail-closed capability contracts."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / "apps" / "agents-builder-kaios"


class KaiOSContracts(unittest.TestCase):
    def test_packaged_webapp_manifest_is_unprivileged(self):
        manifest = json.loads((APP / "manifest.webapp").read_text())
        self.assertEqual(manifest["launch_path"], "/index.html")
        self.assertEqual(manifest["permissions"], {})
        self.assertEqual(manifest["type"], "web")

    def test_offline_editor_does_not_claim_cloud_execution(self):
        js = (APP / "app.js").read_text()
        self.assertIn("localStorage.setItem", js)
        self.assertIn("JSON.parse", js)
        self.assertIn("value.nodes.length > 32", js)
        self.assertIn("Chmura niedostępna", js)
        self.assertNotIn("eval(", js)
        self.assertNotIn("fetch(", js)

    def test_single_non_module_script_and_keypad_controls(self):
        html = (APP / "index.html").read_text()
        js = (APP / "app.js").read_text()
        self.assertIn('<script src="app.js"></script>', html)
        self.assertIn("SoftLeft", js)
        self.assertIn("SoftRight", js)
        self.assertTrue((APP / "style.css").is_file())


if __name__ == "__main__":
    unittest.main()
