"""Negative tests for the KaiOS package verifier."""
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from scripts.verify_kaios_package import verify

VALID = {
    "manifest.webapp": json.dumps({"launch_path": "/index.html", "type": "web", "permissions": {}}).encode(),
    "index.html": b'<script src="app.js"></script>',
    "app.js": b"/* source */",
    "style.css": b"body {}",
}


class KaiOSPackageVerifierTests(unittest.TestCase):
    def check(self, members):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "package.zip"
            with zipfile.ZipFile(path, "w") as archive:
                for name, data in members:
                    archive.writestr(name, data)
            return verify(path)

    def test_valid_package(self):
        self.assertEqual(len(self.check(list(VALID.items()))), 4)

    def test_path_traversal_rejected(self):
        with self.assertRaises(ValueError):
            self.check(list(VALID.items()) + [("../evil", b"x")])

    def test_duplicate_rejected(self):
        with self.assertRaises(ValueError):
            self.check(list(VALID.items()) + [("app.js", b"evil")])

    def test_privileged_permissions_rejected(self):
        data = dict(VALID)
        data["manifest.webapp"] = json.dumps({
            "launch_path": "/index.html", "type": "web", "permissions": {"contacts": {}}
        }).encode()
        with self.assertRaises(ValueError):
            self.check(list(data.items()))

    def test_wrong_entrypoint_rejected(self):
        data = dict(VALID)
        data["manifest.webapp"] = json.dumps({
            "launch_path": "/evil.html", "type": "web", "permissions": {}
        }).encode()
        with self.assertRaises(ValueError):
            self.check(list(data.items()))

    def test_oversized_member_rejected(self):
        data = dict(VALID)
        data["app.js"] = b"x" * (64 * 1024 + 1)
        with self.assertRaises(ValueError):
            self.check(list(data.items()))


if __name__ == "__main__":
    unittest.main()
