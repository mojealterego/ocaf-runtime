"""Fail-closed release bundle validation tests; runs without GitHub credentials."""
from __future__ import annotations

import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

from scripts.verify_release_bundle import BundleError, verify_bundle

BINARIES = ("ocaf-coordinator", "ocaf-cluster-health", "ocaf-rpc-node", "ocaf-effect-sink")
ARCHIVE = "ocaf_runtime-linux-x86_64.tar.gz"
SBOM = "ocaf_runtime.cdx.json"


class ReleaseBundleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.make_archive()
        (self.root / SBOM).write_text(json.dumps({
            "bomFormat": "CycloneDX", "specVersion": "1.5",
            "metadata": {"component": {"name": "ocaf_runtime"}},
            "components": [{"type": "library", "name": "serde", "version": "1.0"}],
        }), encoding="utf-8")
        self.write_checksums()

    def make_archive(self, *, bad_name: str | None = None,
                     mode: int = 0o755, symbolic_link: bool = False) -> None:
        with tarfile.open(self.root / ARCHIVE, "w:gz") as tar:
            for name in BINARIES:
                item = tarfile.TarInfo(bad_name if bad_name and name == BINARIES[0] else name)
                if symbolic_link and name == BINARIES[0]:
                    item.type = tarfile.SYMTYPE
                    item.linkname = "../etc/passwd"
                else:
                    item.size = 4
                    item.mode = mode
                tar.addfile(item, io.BytesIO(b"OCAF") if item.isfile() else None)

    def write_checksums(self) -> None:
        lines = []
        for name in (ARCHIVE, SBOM):
            digest = hashlib.sha256((self.root / name).read_bytes()).hexdigest()
            lines.append(f"{digest}  {name}\n")
        (self.root / "SHA256SUMS").write_text("".join(lines), encoding="ascii")

    def test_accepts_verified_two_file_bundle(self) -> None:
        evidence = verify_bundle(self.root)
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(set(evidence["sha256"]), {ARCHIVE, SBOM})
        self.assertEqual(evidence["binaries"], sorted(BINARIES))

    def test_rejects_tampered_archive(self) -> None:
        with (self.root / ARCHIVE).open("ab") as f:
            f.write(b"tampering")
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_tampered_sbom(self) -> None:
        (self.root / SBOM).write_text("{}", encoding="utf-8")
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_extra_checksum_entry(self) -> None:
        with (self.root / "SHA256SUMS").open("a", encoding="ascii") as f:
            f.write("0" * 64 + "  another-file\n")
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_path_traversal_in_tar(self) -> None:
        self.make_archive(bad_name="../ocaf-coordinator")
        self.write_checksums()
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_symlink_in_tar(self) -> None:
        self.make_archive(symbolic_link=True)
        self.write_checksums()
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_non_executable_binary(self) -> None:
        self.make_archive(mode=0o644)
        self.write_checksums()
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_empty_components(self) -> None:
        (self.root / SBOM).write_text(json.dumps({
            "bomFormat": "CycloneDX", "specVersion": "1.5",
            "metadata": {"component": {"name": "ocaf_runtime"}},
            "components": [],
        }), encoding="utf-8")
        self.write_checksums()
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_symlinked_checksum_file(self) -> None:
        checksums = self.root / "SHA256SUMS"
        real = self.root / "actual-checksums"
        checksums.rename(real)
        checksums.symlink_to(real)
        with self.assertRaises(BundleError):
            verify_bundle(self.root)

    def test_rejects_missing_component_name(self) -> None:
        (self.root / SBOM).write_text(json.dumps({
            "bomFormat": "CycloneDX", "specVersion": "1.5",
            "metadata": {}, "components": [{"name": "test"}],
        }), encoding="utf-8")
        self.write_checksums()
        with self.assertRaises(BundleError):
            verify_bundle(self.root)


if __name__ == "__main__":
    unittest.main()
