"""Executable tests of native artifact format verification (not a GUI test)."""
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("native_verifier", ROOT / "scripts/verify_native_artifact.py")
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class NativeArtifactVerification(unittest.TestCase):
    def test_windows_pe_signature(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "app.exe"
            payload = bytearray(4096)
            payload[:2] = b"MZ"
            struct.pack_into("<I", payload, 0x3c, 256)
            payload[256:260] = bytes.fromhex("50450000")
            p.write_bytes(payload)
            module.verify("windows", p)
            payload[256:260] = b"FAIL"
            p.write_bytes(payload)
            with self.assertRaises(ValueError):
                module.verify("windows", p)

    def test_macho_signature(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "app"
            p.write_bytes(bytes.fromhex("cffaedfe") + bytes(4092))
            module.verify("macos", p)
            module.verify("ios", p)
            p.write_bytes(b"NOPE" + bytes(4092))
            with self.assertRaises(ValueError):
                module.verify("ios", p)

    def test_apk_requires_manifest_and_dex(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "app.apk"
            with zipfile.ZipFile(p, "w") as archive:
                archive.writestr("AndroidManifest.xml", bytes(2048))
                archive.writestr("classes.dex", bytes(2048))
            module.verify("android", p)
            with zipfile.ZipFile(p, "w") as archive:
                archive.writestr("AndroidManifest.xml", bytes(2048))
            with self.assertRaises(ValueError):
                module.verify("android", p)

    def test_missing_artifact_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                module.verify("windows", Path(d) / "missing.exe")


if __name__ == "__main__":
    unittest.main()
