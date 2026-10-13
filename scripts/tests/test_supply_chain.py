"""Regression tests for Cargo.lock policy and untrusted sources."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / "supply_chain.py"
spec = importlib.util.spec_from_file_location("supply_chain", MODULE)
sc = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = sc
spec.loader.exec_module(sc)

HEADER = """version = 4

[[package]]
name = "ocaf_runtime"
version = "0.26.0"
dependencies = ["test-crate"]

[[package]]
name = "test-crate"
version = "1.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "{checksum}"
"""


class SupplyChainTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / "Cargo.toml").write_text(
            '[package]\nname = "ocaf_runtime"\nversion = "0.26.0"\nedition = "2021"\n',
            encoding="utf-8",
        )
        self.lock = self.root / "Cargo.lock"
        self.lock.write_text(HEADER.format(checksum="a" * 64), encoding="utf-8")

    def test_accepts_exact_pinned_registry_snapshot(self):
        result = sc.inspect(self.root)
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["registry_packages"], 1)
        self.assertEqual(len(result["cargo_lock_sha256"]), 64)

    def _create_reviewed_patch(self):
        crate = self.root / "vendor/zenoh-transport"
        crate.mkdir(parents=True)
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "zenoh-transport"\nversion = "1.10.1"\n'
            '[dependencies.lz4_flex]\nversion = "0.11.6"\n',
            encoding="utf-8",
        )
        with (self.root / "Cargo.toml").open("a", encoding="utf-8") as fh:
            fh.write('\n[patch.crates-io]\nzenoh-transport = { path = "vendor/zenoh-transport" }\n')
        self.lock.write_text(self.lock.read_text(encoding="utf-8")
            + '\n[[package]]\nname = "zenoh-transport"\nversion = "1.10.1"\n'
            + '\n[[package]]\nname = "lz4_flex"\nversion = "0.11.6"\n'
            + 'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            + 'checksum = "' + "b"*64 + '"\n', encoding="utf-8")

    def test_accepts_explicit_single_vendored_security_patch(self):
        self._create_reviewed_patch()
        report = sc.inspect(self.root)
        self.assertTrue(report["vendored_zenoh_transport"])

    def test_rejects_unsafe_codec_backslide(self):
        self._create_reviewed_patch()
        manifest = self.root / "vendor/zenoh-transport/Cargo.toml"
        manifest.write_text(manifest.read_text().replace("0.11.6", "0.10.0"))
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_rejects_patched_manifest_without_local_package(self):
        self._create_reviewed_patch()
        self.lock.write_text(self.lock.read_text().replace(
            '[[package]]\nname = "zenoh-transport"\nversion = "1.10.1"\n', ""
        ))
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_rejects_arbitrary_extra_local_package(self):
        self._create_reviewed_patch()
        self.lock.write_text(self.lock.read_text()
            + '\n[[package]]\nname = "injected"\nversion = "1.0.0"\n')
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_rejects_missing_vendored_source_manifest(self):
        self._create_reviewed_patch()
        (self.root / "vendor/zenoh-transport/Cargo.toml").unlink()
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_refuses_missing_lockfile(self):
        self.lock.unlink()
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_refuses_git_dependency_even_with_checksum(self):
        self.lock.write_text(
            self.lock.read_text(encoding="utf-8").replace(
                "registry+https://github.com/rust-lang/crates.io-index",
                "git+https://github.com/evil/dep#abcd",
            ), encoding="utf-8")
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_refuses_wrong_registry(self):
        self.lock.write_text(
            self.lock.read_text(encoding="utf-8").replace(
                "https://github.com/rust-lang/crates.io-index",
                "https://evil.example/index",
            ), encoding="utf-8")
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_refuses_bad_checksum(self):
        self.lock.write_text(HEADER.format(checksum="a" * 63), encoding="utf-8")
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_refuses_duplicate_locked_identity(self):
        raw = self.lock.read_text(encoding="utf-8")
        start = raw.index("[[package]]", raw.index("[[package]]") + 1)
        self.lock.write_text(raw + raw[start:], encoding="utf-8")
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_refuses_mismatched_root_version(self):
        self.lock.write_text(
            self.lock.read_text(encoding="utf-8").replace(
                'version = "0.26.0"', 'version = "0.25.0"'
            ), encoding="utf-8")
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_refuses_symlinked_lockfile(self):
        target = self.root / "other.lock"
        self.lock.rename(target)
        self.lock.symlink_to(target)
        with self.assertRaises(sc.PolicyError):
            sc.inspect(self.root)

    def test_cli_fails_closed_without_leaking_source_paths(self):
        self.lock.unlink()
        proc = subprocess.run(
            [sys.executable, str(MODULE), "--repo", str(self.root)],
            check=False, capture_output=True, text=True
        )
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(json.loads(proc.stdout)["status"], "FAIL")
        self.assertNotIn(str(self.root), proc.stdout)


if __name__ == "__main__":
    unittest.main()
