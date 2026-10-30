"""Stage 0.56: regression tests for the read-only OmniDev candidate patch gate."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omnidev_patch_gate import GateError, inspect_patch


def git(repo: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", "-C", str(repo), *args], stderr=subprocess.PIPE)


class PatchGateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name) / "repo"
        self.repo.mkdir()
        git(self.repo, "init", "-q")
        git(self.repo, "config", "user.email", "test@example.invalid")
        git(self.repo, "config", "user.name", "OCAF test")
        (self.repo / "src").mkdir()
        (self.repo / "src" / "worker.rs").write_text("pub fn value() -> u32 { 1 }\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-qm", "base")
        self.base = git(self.repo, "rev-parse", "HEAD").decode().strip()
        self.patch = Path(self.tmp.name) / "candidate.patch"

    def make_patch(self, relpath="src/worker.rs", content="pub fn value() -> u32 { 2 }\n"):
        p = self.repo / relpath
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content)
        self.patch.write_bytes(git(self.repo, "diff", "--", relpath))
        git(self.repo, "checkout", "--", relpath)

    def test_valid_patch_is_checked_without_mutating_repo(self):
        self.make_patch()
        before = git(self.repo, "status", "--porcelain")
        result = inspect_patch(self.repo, self.patch, self.base)
        self.assertEqual(result["paths"], ["src/worker.rs"])
        self.assertEqual(result["base_sha"], self.base)
        self.assertEqual(result["changed_lines"], 2)
        self.assertEqual(git(self.repo, "status", "--porcelain"), before)

    def test_rejects_stale_base(self):
        self.make_patch()
        with self.assertRaisesRegex(GateError, "base"):
            inspect_patch(self.repo, self.patch, "0" * 40)

    def test_rejects_invalid_patch_and_non_applicable_patch(self):
        self.patch.write_bytes(b"not a git patch\n")
        with self.assertRaises(GateError):
            inspect_patch(self.repo, self.patch, self.base)
        self.make_patch()
        (self.repo / "src/worker.rs").write_text("changed locally\n")
        git(self.repo, "add", "src/worker.rs")
        with self.assertRaises(GateError):
            inspect_patch(self.repo, self.patch, self.base)

    def test_rejects_protected_paths(self):
        (self.repo / "Cargo.lock").write_text("old\n")
        git(self.repo, "add", "Cargo.lock")
        git(self.repo, "commit", "-qm", "lock")
        self.base = git(self.repo, "rev-parse", "HEAD").decode().strip()
        (self.repo / "Cargo.lock").write_text("new\n")
        self.patch.write_bytes(git(self.repo, "diff", "--", "Cargo.lock"))
        git(self.repo, "checkout", "--", "Cargo.lock")
        with self.assertRaisesRegex(GateError, "path"):
            inspect_patch(self.repo, self.patch, self.base)

    def test_rejects_path_traversal_and_binary(self):
        self.patch.write_bytes(b"diff --git a/../../outside b/../../outside\n--- a/../../outside\n+++ b/../../outside\n@@ -1 +1 @@\n-a\n+b\n")
        with self.assertRaises(GateError):
            inspect_patch(self.repo, self.patch, self.base)
        self.patch.write_bytes(b"diff --git a/src/worker.rs b/src/worker.rs\nGIT binary patch\nliteral 1\nAcmZQz\n")
        with self.assertRaises(GateError):
            inspect_patch(self.repo, self.patch, self.base)

    def test_rejects_symlink_ancestor(self):
        (self.repo / "src" / "link").symlink_to(self.repo / "src", target_is_directory=True)
        self.patch.write_text("diff --git a/src/link/file.rs b/src/link/file.rs\nnew file mode 100644\n--- /dev/null\n+++ b/src/link/file.rs\n@@ -0,0 +1 @@\n+hello\n")
        with self.assertRaises(GateError):
            inspect_patch(self.repo, self.patch, self.base)

    def test_new_regular_source_file_is_allowed(self):
        path = self.repo / "tests" / "contract.rs"
        path.parent.mkdir(parents=True)
        self.patch.write_text(
            "diff --git a/tests/contract.rs b/tests/contract.rs\n"
            "new file mode 100644\n--- /dev/null\n+++ b/tests/contract.rs\n"
            "@@ -0,0 +1 @@\n+#[test] fn contract() {}\n"
        )
        self.assertEqual(inspect_patch(self.repo, self.patch, self.base)["paths"], ["tests/contract.rs"])

    def test_rejects_untracked_path_collision(self):
        path = self.repo / "tests" / "contract.rs"
        path.parent.mkdir(parents=True)
        path.write_text("local untracked content\n")
        self.patch.write_text(
            "diff --git a/tests/contract.rs b/tests/contract.rs\n"
            "new file mode 100644\n--- /dev/null\n+++ b/tests/contract.rs\n"
            "@@ -0,0 +1 @@\n+test\n"
        )
        with self.assertRaises(GateError):
            inspect_patch(self.repo, self.patch, self.base)

    def test_rejects_executable_mode_change(self):
        self.patch.write_text(
            "diff --git a/src/worker.rs b/src/worker.rs\n"
            "old mode 100644\nnew mode 100755\n"
        )
        with self.assertRaisesRegex(GateError, "mode"):
            inspect_patch(self.repo, self.patch, self.base)

    def test_rejects_oversized_patch(self):
        self.patch.write_bytes(b"x" * (512 * 1024 + 1))
        with self.assertRaisesRegex(GateError, "size"):
            inspect_patch(self.repo, self.patch, self.base)


if __name__ == "__main__":
    unittest.main()
