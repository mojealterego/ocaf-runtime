"""Process-level desktop smoke test behavior with real subprocesses."""
import importlib.util
from pathlib import Path
import os
import stat
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("desktop_smoke", ROOT / "scripts/smoke_native_desktop.py")
assert SPEC and SPEC.loader
smoke = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(smoke)


class DesktopSmokeTests(unittest.TestCase):
    def test_missing_binary_fails_closed(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(FileNotFoundError):
                smoke.smoke(Path(d) / "missing")

    @unittest.skipIf(sys.platform == "win32", "POSIX executable fixture")
    def test_healthy_process_survives_window(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "healthy"
            p.write_text("#!/bin/sh\nsleep 3\n")
            p.chmod(p.stat().st_mode | stat.S_IXUSR)
            smoke.smoke(p, 0.5)

    @unittest.skipIf(sys.platform == "win32", "POSIX executable fixture")
    def test_crash_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "crash"
            p.write_text("#!/bin/sh\nexit 7\n")
            p.chmod(p.stat().st_mode | stat.S_IXUSR)
            with self.assertRaises(RuntimeError):
                smoke.smoke(p, 0.5)


if __name__ == "__main__":
    unittest.main()
