"""Executable regression tests for Linux GUI smoke failure paths."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import struct
import zlib


def _chunk(kind, payload):
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff)


def _valid_fixture():
    # Real, complete PNG with enough entropy to exceed the smoke size threshold.
    rows = b"".join(b"\\x00" + bytes((x * 17 + y * 31) % 256 for x in range(640 * 3)) for y in range(480))
    return (bytes.fromhex("89504e470d0a1a0a")
            + _chunk(b"IHDR", struct.pack(">IIBBBBB", 640, 480, 8, 2, 0, 0, 0))
            + _chunk(b"IDAT", zlib.compress(rows))
            + _chunk(b"IEND", b""))


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("linux_gui_smoke", ROOT / "scripts/smoke_linux_gui.py")
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class LinuxGuiSmokeTests(unittest.TestCase):
    def test_png_integrity_rejects_corruption(self):
        valid = _valid_fixture()
        self.assertTrue(module._valid_png(valid))
        crc_corrupt = bytearray(valid)
        crc_corrupt[29] ^= 1
        self.assertFalse(module._valid_png(bytes(crc_corrupt)))
        self.assertFalse(module._valid_png(valid[:-12]))  # no IEND
        self.assertFalse(module._valid_png(valid + b"trailing"))
        self.assertFalse(module._valid_png(valid[:-2]))
        self.assertFalse(module._valid_png(valid[:8]))
        self.assertFalse(module._valid_png(
            valid[:33] + _chunk(b"IEND", b"")))  # no IDAT

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
                        screenshot.write_bytes(_valid_fixture())
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
                    return Result(0, "9876\n")
                if argv[0] == "xdotool":
                    return Result(0, "1234\n")
                if argv[0] == "import":
                    captures.append(kwargs.get("timeout"))
                    if len(captures) == 1:
                        raise module.subprocess.CalledProcessError(1, argv)
                    import struct
                    screenshot.write_bytes(_valid_fixture())
                    return Result()
                raise AssertionError(argv)
            with (patch.object(module.subprocess, "Popen", return_value=Process()),
                  patch.object(module.subprocess, "run", side_effect=command),
                  patch.object(module.time, "sleep")):
                module.run(binary, screenshot, seconds=5)
            self.assertEqual(len(captures), 2)
            self.assertTrue(all(0 < timeout <= 5 for timeout in captures))



    def test_window_discovery_respects_global_deadline(self):
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

            def command(argv, **kwargs):
                if argv[0] == "xdotool":
                    self.assertGreater(kwargs["timeout"], 0)
                    self.assertLessEqual(kwargs["timeout"], 0.1)
                    raise module.subprocess.TimeoutExpired(argv, kwargs["timeout"])
                raise AssertionError(argv)

            with (patch.object(module.subprocess, "Popen", return_value=Process()),
                  patch.object(module.subprocess, "run", side_effect=command),
                  patch.object(module.time, "sleep")):
                with self.assertRaisesRegex(RuntimeError, "No visible"):
                    module.run(binary, Path(tmp) / "screen.png", seconds=0.1)

if __name__ == "__main__":
    unittest.main()
