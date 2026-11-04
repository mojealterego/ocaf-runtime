"""Executable regression tests for Linux GUI smoke failure paths."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("linux_gui_smoke", ROOT / "scripts/smoke_linux_gui.py")
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class LinuxGuiSmokeTests(unittest.TestCase):
    def test_missing_binary_rejected_without_spawning(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(module.subprocess, "Popen") as spawn:
                with self.assertRaises(FileNotFoundError):
                    module.run(Path(tmp) / "missing", Path(tmp) / "screen.png")
                spawn.assert_not_called()

    def test_window_from_another_process_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "binary"
            binary.write_bytes(b"binary")
            class Process:
                pid = 1234
                def poll(self):
                    return None
                def terminate(self):
                    pass
                def communicate(self, timeout):
                    return (b"", b"")
            class Result:
                def __init__(self, code, output):
                    self.returncode = code
                    self.stdout = output
            def command(argv, **kwargs):
                if "search" in argv:
                    return Result(0, "9876\\n")
                return Result(0, "5678\\n")
            with patch.object(module.subprocess, "Popen", return_value=Process()), \\
                 patch.object(module.subprocess, "run", side_effect=command), \\
                 patch.object(module.time, "sleep"):
                with self.assertRaisesRegex(RuntimeError, "No visible"):
                    module.run(binary, Path(tmp) / "screen.png", seconds=0.01)

    def test_dead_process_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "binary"
            binary.write_bytes(b"binary")
            class DeadProcess:
                pid = 1234
                def poll(self):
                    return 1
                def communicate(self, timeout):
                    return (b"", b"")
            with patch.object(module.subprocess, "Popen", return_value=DeadProcess()):
                with self.assertRaisesRegex(RuntimeError, "exited before mapping"):
                    module.run(binary, Path(tmp) / "screen.png", seconds=1)


if __name__ == "__main__":
    unittest.main()
