"""Fail-closed release bundle validation tests; runs without GitHub credentials."""
from __future__ import annotations

import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

from scripts.create_release_manifest import ManifestError, build_manifest
from scripts.verify_release_bundle import BundleError, verify_bundle

BINARIES = ("ocaf-coordinator", "ocaf-cluster-health", "ocaf-rpc-node", "ocaf-effect-sink")
ARCHIVE = "ocaf_runtime-linux-x86_64.tar.gz"
SBOM = "ocaf_runtime.cdx.json"
MANIFEST = "RELEASE-MANIFEST.json"
SHA = "a" * 40
REPOSITORY = "mojealterego/ocaf-runtime"


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
        self.write_manifest()
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
        for name in (ARCHIVE, SBOM, MANIFEST):
            digest = hashlib.sha256((self.root / name).read_bytes()).hexdigest()
            lines.append(f"{digest}  {name}\n")
        (self.root / "SHA256SUMS").write_text("".join(lines), encoding="ascii")

    def write_manifest(self, *, sha: str = SHA, repository: str = REPOSITORY) -> None:
        manifest = build_manifest(self.root, sha, repository)
        (self.root / MANIFEST).write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )

    def verify(self):
        return verify_bundle(
            self.root, expected_sha=SHA, expected_repository=REPOSITORY
        )

    def test_accepts_source_bound_release_bundle(self) -> None:
        evidence = self.verify()
        self.assertEqual(evidence["status"], "PASS")
        self.assertEqual(set(evidence["sha256"]), {ARCHIVE, SBOM, MANIFEST})
        self.assertEqual(evidence["binaries"], sorted(BINARIES))
        self.assertEqual(evidence["source"]["git_sha"], SHA)

    def test_rejects_tampered_archive(self) -> None:
        with (self.root / ARCHIVE).open("ab") as f:
            f.write(b"tampering")
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_tampered_sbom(self) -> None:
        (self.root / SBOM).write_text("{}", encoding="utf-8")
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_extra_checksum_entry(self) -> None:
        with (self.root / "SHA256SUMS").open("a", encoding="ascii") as f:
            f.write("0" * 64 + "  another-file\n")
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_path_traversal_in_tar(self) -> None:
        self.make_archive(bad_name="../ocaf-coordinator")
        self.write_manifest()
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_symlink_in_tar(self) -> None:
        self.make_archive(symbolic_link=True)
        self.write_manifest()
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_non_executable_binary(self) -> None:
        self.make_archive(mode=0o644)
        self.write_manifest()
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_empty_components(self) -> None:
        (self.root / SBOM).write_text(json.dumps({
            "bomFormat": "CycloneDX", "specVersion": "1.5",
            "metadata": {"component": {"name": "ocaf_runtime"}},
            "components": [],
        }), encoding="utf-8")
        self.write_manifest()
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_symlinked_checksum_file(self) -> None:
        checksums = self.root / "SHA256SUMS"
        real = self.root / "actual-checksums"
        checksums.rename(real)
        checksums.symlink_to(real)
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_missing_component_name(self) -> None:
        (self.root / SBOM).write_text(json.dumps({
            "bomFormat": "CycloneDX", "specVersion": "1.5",
            "metadata": {}, "components": [{"name": "test"}],
        }), encoding="utf-8")
        self.write_manifest()
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_manifest_bound_to_different_commit(self) -> None:
        self.write_manifest(sha="b" * 40)
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_manifest_subject_hash_substitution(self) -> None:
        manifest = json.loads((self.root / MANIFEST).read_text(encoding="utf-8"))
        manifest["subjects"][ARCHIVE]["sha256"] = "0" * 64
        (self.root / MANIFEST).write_text(json.dumps(manifest), encoding="utf-8")
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_duplicate_manifest_json_field(self) -> None:
        (self.root / MANIFEST).write_text(
            '{"schema_version":1,"schema_version":1,"source":{},"subjects":{}}',
            encoding="utf-8",
        )
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_rejects_symlinked_release_manifest(self) -> None:
        manifest = self.root / MANIFEST
        real = self.root / "actual-release-manifest.json"
        manifest.rename(real)
        manifest.symlink_to(real)
        self.write_checksums()
        with self.assertRaises(BundleError):
            self.verify()

    def test_manifest_builder_rejects_invalid_source_identity(self) -> None:
        with self.assertRaises(ManifestError):
            build_manifest(self.root, "not-a-commit", REPOSITORY)
        with self.assertRaises(ManifestError):
            build_manifest(self.root, SHA, "not-a-repository")

    def test_manifest_builder_rejects_symlinked_subject(self) -> None:
        archive = self.root / ARCHIVE
        real = self.root / "actual-release.tar.gz"
        archive.rename(real)
        archive.symlink_to(real)
        with self.assertRaises(ManifestError):
            build_manifest(self.root, SHA, REPOSITORY)


if __name__ == "__main__":
    unittest.main()
