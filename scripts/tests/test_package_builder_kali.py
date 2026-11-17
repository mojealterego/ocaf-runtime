"""Kali packaging tests; /bin/true is a fixture, not OCAF binary proof."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import struct
import zlib
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "package_builder_kali.py"
spec = importlib.util.spec_from_file_location("package_builder_kali", SCRIPT)
assert spec and spec.loader
pkg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pkg)

def chunk(name, data):
    return struct.pack(">I", len(data)) + name + data + struct.pack(">I", zlib.crc32(name + data) & 0xffffffff)


PNG = (bytes.fromhex("89504e470d0a1a0a")
       + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(b"\x00\x00\x00\x00\xff"))
       + chunk(b"IEND", b""))
COMMIT = "0" * 40


@unittest.skipUnless(shutil.which("dpkg-deb") and Path("/bin/true").is_file(),
                     "dpkg-deb and Linux x86-64 fixture required")
class KaliPackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.binary = root / "binary"
        self.binary.write_bytes(Path("/bin/true").read_bytes())
        self.icon = root / "icon.png"
        self.icon.write_bytes(PNG)
        self.one = root / "a.deb"

    def build(self, output):
        return pkg.package_kali(self.binary, self.icon, output,
                                version="0.1.0", commit=COMMIT)

    def test_builds_reproducible_kali_deb_and_validates_payload(self):
        a = self.build(self.one)
        b = self.build(self.one.parent / "b.deb")
        self.assertEqual(a["package_sha256"], b["package_sha256"])
        self.assertEqual(a["platform"], "kali-linux-amd64")
        self.assertEqual(a["signing"], "UNSIGNED")
        p = subprocess.check_output(["dpkg-deb", "--field", str(self.one), "Architecture"], text=True).strip()
        self.assertEqual(p, "amd64")
        listing = subprocess.check_output(["dpkg-deb", "--contents", str(self.one)], text=True)
        self.assertIn("./usr/bin/ocaf-agents-builder", listing)
        self.assertIn("org.ocaf.agentsbuilder.desktop", listing)

    def test_rejects_wrong_arch_and_invalid_source_commit(self):
        invalid = bytearray(self.binary.read_bytes())
        invalid[18:20] = b"\x28\x00"
        self.binary.write_bytes(invalid)
        with self.assertRaisesRegex(ValueError, "x86-64"):
            self.build(self.one)
        self.binary.write_bytes(Path("/bin/true").read_bytes())
        with self.assertRaisesRegex(ValueError, "source commit"):
            pkg.package_kali(self.binary, self.icon, self.one, version="0.1.0", commit="main")

    def test_refuses_symlink_and_overwrite(self):
        link = self.one.parent / "symlink"
        link.symlink_to(self.binary)
        with self.assertRaisesRegex(ValueError, "regular input"):
            pkg.package_kali(link, self.icon, self.one, version="0.1.0", commit=COMMIT)
        self.build(self.one)
        before = self.one.read_bytes()
        with self.assertRaises(FileExistsError):
            self.build(self.one)
        self.assertEqual(before, self.one.read_bytes())

    def test_png_corruption_and_trailing_data_rejected(self):
        source = self.icon.read_bytes()
        self.icon.write_bytes(source[:-1] + bytes([source[-1] ^ 1]))
        with self.assertRaisesRegex(ValueError, "CRC"):
            self.build(self.one)
        self.icon.write_bytes(source + b"X")
        with self.assertRaisesRegex(ValueError, "IEND or trailing"):
            self.build(self.one)
        self.assertFalse(self.one.exists())

    def test_invalid_png_fails_without_publish(self):
        self.icon.write_bytes(b"not png" * 200)
        with self.assertRaisesRegex(ValueError, "PNG"):
            self.build(self.one)
        self.assertFalse(self.one.exists())


if __name__ == "__main__":
    unittest.main()
