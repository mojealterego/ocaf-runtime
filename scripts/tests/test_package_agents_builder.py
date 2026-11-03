"""Security and reproducibility tests for source bundle release tooling."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / "package_agents_builder.py"
SPEC = importlib.util.spec_from_file_location("package_agents_builder", SCRIPT)
assert SPEC and SPEC.loader
bundle = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bundle)


class PackageBuilderTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ocaf-package-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        def git(*args):
            subprocess.run(
                ["git", "-C", str(self.root), *args], check=True,
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
            )
        git("init", "-q")
        git("config", "user.email", "build-test@example.invalid")
        git("config", "user.name", "OCAF test")
        self.write("apps/agents-builder-web/index.html", b"<html>ok</html>")
        self.write("src/builder_core.rs", b"// compiler")
        self.write("notes.txt", b"tracked")
        git("add", ".")
        git("commit", "-qm", "test source")

    def write(self, relative, contents):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(contents)

    def test_selection_covers_ui_compiler_tests_ci_and_docs(self):
        self.assertTrue(bundle.select_builder("apps/agents-builder-web/tests/cloud.test.mjs"))
        self.assertTrue(bundle.select_builder("src/builder_core.rs"))
        self.assertTrue(bundle.select_builder("src/model_router.rs"))
        self.assertTrue(bundle.select_builder("tests/builder_cloud.rs"))
        self.assertTrue(bundle.select_builder(".github/workflows/agents-builder-source-bundle.yml"))
        self.assertFalse(bundle.select_builder(".env"))
        self.assertFalse(bundle.select_builder("src/unrelated.rs"))

    def test_reproducible_snapshots_and_sha256_manifest(self):
        paths = bundle.tracked_files(self.root)
        sha = bundle.current_sha(self.root)
        a = Path(self.temp.name) / "first.zip"
        b = Path(self.temp.name) / "second.zip"
        left = bundle.write_bundle(self.root, a, sha, paths, "unit-test")
        right = bundle.write_bundle(self.root, b, sha, paths, "unit-test")
        self.assertEqual(left["sha256"], right["sha256"])
        self.assertEqual(hashlib.sha256(a.read_bytes()).hexdigest(), left["sha256"])
        with zipfile.ZipFile(a) as archive:
            self.assertTrue(any(p.endswith("/RELEASE_MANIFEST.json") for p in archive.namelist()))
            self.assertEqual(len(archive.namelist()), 4)

    def test_rejects_modified_symlink_and_path_escape(self):
        with self.assertRaises(ValueError):
            bundle.archive_bytes(self.root, "src/../notes.txt")
            # Direct archive_bytes call must also reject path traversal.
        # A real symlink must be rejected even when it points inside the root.
        linked = self.root / "linked.txt"
        linked.symlink_to("notes.txt")
        with self.assertRaises(ValueError):
            bundle.archive_bytes(self.root, "linked.txt")
        self.assertEqual(bundle.tracked_files(self.root),
                         ["apps/agents-builder-web/index.html", "notes.txt", "src/builder_core.rs"])

    def test_bundle_is_refused_when_file_mutated_between_checkout_and_package(self):
        paths = bundle.tracked_files(self.root)
        self.write("notes.txt", b"UNCOMMITTED")
        # Dirty tracked content must not be packaged under a clean commit SHA.
        with self.assertRaisesRegex(ValueError, "dirty"):
            bundle.require_clean_tree(self.root)
        self.assertEqual(paths[1], "notes.txt")


if __name__ == "__main__":
    unittest.main()
