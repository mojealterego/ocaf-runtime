"""Static contracts for Linux GUI smoke evidence and process ownership."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "smoke_linux_gui.py"
WORKFLOW = ROOT / ".github" / "workflows" / "agents-builder-native-linux.yml"


class LinuxGuiSmokeContracts(unittest.TestCase):
    def test_window_is_owned_by_spawned_process(self):
        source = SCRIPT.read_text()
        self.assertIn('"getwindowpid"', source)
        self.assertIn("str(process.pid)", source)
        self.assertIn('if window is None:', source)

    def test_screenshot_requires_png_magic_and_nontrivial_size(self):
        source = SCRIPT.read_text()
        self.assertIn('bytes.fromhex("89504e470d0a1a0a")', source)
        self.assertIn("st_size < 1024", source)

    def test_workflow_gates_artifact_on_gui_smoke(self):
        workflow = WORKFLOW.read_text()
        self.assertLess(workflow.index("Verify mapped Linux GUI"), workflow.index("Upload Linux executable"))
        self.assertIn("linux-gui.png", workflow)
        self.assertIn("smoke_linux_gui.py", workflow)


if __name__ == "__main__":
    unittest.main()
