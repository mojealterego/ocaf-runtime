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
                    return Result(0, "9876\n")
                return Result(0, "5678\n")
            with (patch.object(module.subprocess, "Popen", return_value=Process()),
                  patch.object(module.subprocess, "run", side_effect=command),
                  patch.object(module.time, "sleep")):
                with self.assertRaisesRegex(RuntimeError, "No visible"):
                    module.run(binary, Path(tmp) / "screen.png", seconds=0.01)

    def test_invalid_first_capture_retries_until_valid_png(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "binary"
            binary.write_bytes(b"binary")
            screenshot = Path(tmp) / "screen.png"
            class Process:
                pid = 1234
                def poll(self):
                    return None
                def terminate(self):
                    pass
                def communicate(self, timeout):
                    return (b"", b"")
            class Result:
                def __init__(self, code=0, output=""):
                    self.returncode = code
                    self.stdout = output
            captures = []
            def command(argv, **kwargs):
                if argv[0] == "xdotool" and "search" in argv:
                    return Result(0, "9876\n")
                if argv[0] == "xdotool":
                    return Result(0, "1234\n")
                if argv[0] == "import":
                    captures.append(1)
                    if len(captures) == 1:
                        screenshot.write_bytes(b"bad")
                    else:
                        import struct
                        screenshot.write_bytes(bytes.fromhex("89504e470d0a1a0a") + struct.pack(">I", 13) + b"IHDR" + struct.pack(">II", 640, 480) + b"\x08\x02\x00\x00\x00" + b"x" * 1100)
                    return Result()
                raise AssertionError(argv)
            with (patch.object(module.subprocess, "Popen", return_value=Process()),
                  patch.object(module.subprocess, "run", side_effect=command),
                  patch.object(module.time, "sleep")):
                module.run(binary, screenshot, seconds=5)
            self.assertEqual(len(captures), 2)

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


    def test_capture_command_failure_is_retried(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "binary"
            binary.write_bytes(b"binary")
            screenshot = Path(tmp) / "screen.png"
            class Process:
                pid = 1234
                def poll(self):
                    return None
                def terminate(self):
                    pass
                def communicate(self, timeout):
                    return (b"", b"")
            class Result:
                def __init__(self, code=0, output=""):
                    self.returncode = code
                    self.stdout = output
            captures = []
            def command(argv, **kwargs):
                if argv[0] == "xdotool" and "search" in argv:
                    return Result(0, "9876\\n")
                if argv[0] == "xdotool":
                    return Result(0, "1234\\n")
                if argv[0] == "import":
                    captures.append(kwargs.get("timeout"))
                    if len(captures) == 1:
                        raise module.subprocess.CalledProcessError(1, argv)
                    import struct
                    screenshot.write_bytes(bytes.fromhex("89504e470d0a1a0a") + struct.pack(">I", 13) + b"IHDR" + struct.pack(">II", 640, 480) + b"\\x08\\x02\\x00\\x00\\x00" + b"x" * 1100)
                    return Result()
                raise AssertionError(argv)
            with (patch.object(module.subprocess, "Popen", return_value=Process()),
                  patch.object(module.subprocess, "run", side_effect=command),
                  patch.object(module.time, "sleep")):
                module.run(binary, screenshot, seconds=5)
            self.assertEqual(len(captures), 2)
            self.assertTrue(all(0 < timeout <= 5 for timeout in captures))


if __name__ == "__main__":
    unittest.main()
