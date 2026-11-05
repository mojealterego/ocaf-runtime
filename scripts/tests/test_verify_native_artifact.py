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
                archive.writestr("classes.dex", b"dex\n035\x00" + bytes(2040))
            module.verify("android", p)
            with zipfile.ZipFile(p, "w") as archive:
                archive.writestr("AndroidManifest.xml", bytes(2048))
            with self.assertRaises(ValueError):
                module.verify("android", p)

    def test_apk_rejects_fake_or_empty_dex_bytecode(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "app.apk"
            for dex in (b"", bytes(2048), b"dex\n999\x00" + bytes(2040)):
                with zipfile.ZipFile(p, "w") as archive:
                    archive.writestr("AndroidManifest.xml", bytes(2048))
                    archive.writestr("classes.dex", dex)
                with self.subTest(dex=dex[:8]), self.assertRaises(ValueError):
                    module.verify("android", p)

    def test_apk_rejects_duplicate_security_critical_entries(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "duplicate.apk"
            for duplicate in ("AndroidManifest.xml", "classes.dex"):
                with self.subTest(duplicate=duplicate):
                    with zipfile.ZipFile(p, "w") as archive:
                        archive.writestr("AndroidManifest.xml", bytes(2048))
                        archive.writestr("classes.dex", b"dex\n035\x00" + bytes(2040))
                        import warnings
                        with warnings.catch_warnings():
                            warnings.simplefilter("ignore", UserWarning)
                            archive.writestr(duplicate, bytes(2048))
                    with self.assertRaisesRegex(ValueError, "duplicate"):
                        module.verify("android", p)

    def test_linux_elf_signature_and_header(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "app"
            payload = bytearray(4096)
            payload[:4] = bytes.fromhex("7f454c46")
            payload[4] = 2  # ELFCLASS64
            payload[5] = 1  # little endian
            payload[16:18] = bytes.fromhex("0300")  # ET_DYN (PIE)
            p.write_bytes(payload)
            module.verify("linux", p)
            for offset, bad in [(0, b"NOPE"), (4, bytes(1)), (5, bytes(1)), (16, bytes(2))]:
                changed = bytearray(payload)
                changed[offset:offset + len(bad)] = bad
                p.write_bytes(changed)
                with self.subTest(offset=offset), self.assertRaises(ValueError):
                    module.verify("linux", p)

    def test_missing_artifact_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                module.verify("windows", Path(d) / "missing.exe")


if __name__ == "__main__":
    unittest.main()
