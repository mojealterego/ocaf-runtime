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

    def test_rejects_symlinked_parent_directory_even_if_leaf_is_regular(self):
        # An attacker can replace a tracked parent directory with a symlink
        # after checkout. lstat() of the leaf alone does not prevent escape.
        outside = Path(self.temp.name) / "outside"
        outside.mkdir()
        (outside / "payload.txt").write_bytes(b"PRIVATE-OUTSIDE-REPOSITORY")
        (self.root / "linked-dir").symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink|non-regular|unsafe|secure source open rejected"):
            bundle.archive_bytes(self.root, "linked-dir/payload.txt")

    def test_archive_opens_every_component_without_following_symlinks(self):
        import os
        from unittest.mock import patch

        real_open = os.open
        opened = []

        def checked_open(path, flags, *args, **kwargs):
            opened.append((path, flags))
            return real_open(path, flags, *args, **kwargs)

        with patch.object(bundle.os, "open", side_effect=checked_open):
            self.assertEqual(bundle.archive_bytes(self.root, "src/builder_core.rs"),
                             b"// compiler")
        self.assertGreaterEqual(len(opened), 3)
        for _, flags in opened:
            self.assertTrue(flags & os.O_NOFOLLOW)

    def test_bundle_rejects_modified_tracked_content_even_when_zip_matches_worktree(self):
        # A dirty checkout must not be mislabeled with the clean HEAD SHA.
        paths = bundle.tracked_files(self.root)
        sha = bundle.current_sha(self.root)
        self.write("notes.txt", b"UNCOMMITTED-SECRET")
        destination = Path(self.temp.name) / "dirty.zip"
        with self.assertRaisesRegex(ValueError, "Git HEAD|committed source"):
            bundle.write_bundle(self.root, destination, sha, paths, "unit-test")

    def test_failed_verification_preserves_previous_artifact_and_cleans_temporary_file(self):
        # A dirty source must never replace a previously published valid ZIP.
        paths = bundle.tracked_files(self.root)
        sha = bundle.current_sha(self.root)
        destination = Path(self.temp.name) / "published.zip"
        original = bundle.write_bundle(self.root, destination, sha, paths, "unit-test")
        original_bytes = destination.read_bytes()
        self.write("notes.txt", b"UNCOMMITTED-SECRET")
        with self.assertRaisesRegex(ValueError, "Git HEAD|committed source"):
            bundle.write_bundle(self.root, destination, sha, paths, "unit-test")
        self.assertEqual(destination.read_bytes(), original_bytes)
        self.assertEqual(hashlib.sha256(destination.read_bytes()).hexdigest(), original["sha256"])
        self.assertEqual(sorted(p.name for p in destination.parent.iterdir() if p.name.startswith(".published.zip.")), [])

    def test_failed_verification_does_not_publish_new_archive(self):
        paths = bundle.tracked_files(self.root)
        sha = bundle.current_sha(self.root)
        destination = Path(self.temp.name) / "new.zip"
        self.write("notes.txt", b"UNCOMMITTED-SECRET")
        with self.assertRaisesRegex(ValueError, "Git HEAD|committed source"):
            bundle.write_bundle(self.root, destination, sha, paths, "unit-test")
        self.assertFalse(destination.exists())

    def test_rejects_head_movement_during_bundle_verification(self):
        # A concurrent commit must not be published under the old HEAD.
        from unittest.mock import patch

        paths = bundle.tracked_files(self.root)
        sha = bundle.current_sha(self.root)
        destination = Path(self.temp.name) / "head-race.zip"
        real_verify = bundle.verify_bundle

        def concurrent_commit(*args, **kwargs):
            real_verify(*args, **kwargs)
            self.write("notes.txt", b"new committed revision")
            subprocess.run(["git", "-C", str(self.root), "add", "notes.txt"],
                           check=True, capture_output=True)
            subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "concurrent"],
                           check=True, capture_output=True)

        with patch.object(bundle, "verify_bundle", side_effect=concurrent_commit):
            with self.assertRaisesRegex(ValueError, "HEAD changed"):
                bundle.write_bundle(self.root, destination, sha, paths, "unit-test")
        self.assertFalse(destination.exists())

    def test_bundle_is_refused_when_file_mutated_between_checkout_and_package(self):
        paths = bundle.tracked_files(self.root)
        self.write("notes.txt", b"UNCOMMITTED")
        # Dirty tracked content must not be packaged under a clean commit SHA.
        with self.assertRaisesRegex(ValueError, "dirty"):
            bundle.require_clean_tree(self.root)
        self.assertEqual(paths[1], "notes.txt")



    def test_main_does_not_publish_partial_release_when_second_bundle_fails(self):
        from unittest.mock import patch
        import sys

        out = Path(self.temp.name) / "release-output"
        required = (
            "apps/agents-builder-web/index.html",
            "apps/agents-builder-web/app.mjs",
            "apps/agents-builder-web/models.mjs",
            "apps/agents-builder-web/i18n.mjs",
            "apps/agents-builder-web/credentials.mjs",
            "src/builder_core.rs",
            "src/builder_cloud.rs",
            "src/builder_http.rs",
            "src/builder_store.rs",
            "tests/builder_web_contract.rs",
            "scripts/package_agents_builder.py",
        )
        def fail_second(*args, **kwargs):
            if fail_second.calls:
                raise ValueError("injected second archive failure")
            fail_second.calls += 1
            dst = args[1]
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_bytes(b"first archive, not a complete release")
            return {"name": dst.name, "bytes": dst.stat().st_size,
                    "sha256": hashlib.sha256(dst.read_bytes()).hexdigest(),
                    "file_count": len(args[3])}
        fail_second.calls = 0
        with patch.object(bundle, "tracked_files", return_value=list(required)), \\
             patch.object(bundle, "select_builder", return_value=True), \\
             patch.object(bundle, "write_bundle", side_effect=fail_second), \\
             patch.object(sys, "argv", ["package_agents_builder.py", "--repo", str(self.root),
                                        "--out", str(out)]):
            with self.assertRaisesRegex(ValueError, "injected second archive failure"):
                bundle.main()
        self.assertFalse(any(out.iterdir()) if out.exists() else False,
                         "failed release must not expose any partial artifacts")


if __name__ == "__main__":
    unittest.main()
