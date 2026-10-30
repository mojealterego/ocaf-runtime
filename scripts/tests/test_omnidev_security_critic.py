"""0.56 — real patch static risk inspection, always requires independent review."""
import contextlib
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omnidev_security_critic import inspect_added_lines, criticize_patch, review_candidate, main


class CriticTests(unittest.TestCase):
    def setUp(self):
        self.head = b"diff --git a/src/worker.rs b/src/worker.rs\nindex 1234567..abcdef0 100644\n--- a/src/worker.rs\n+++ b/src/worker.rs\n"

    def test_safe_candidate_never_gets_automatic_security_approval(self):
        p = self.head + b"@@ -1 +1 @@\n-pub fn value() -> u32 { 1 }\n+pub fn value() -> u32 { 2 }\n"
        r = criticize_patch(p, ["src/worker.rs"])
        self.assertEqual(r["status"], "SECURITY_REVIEW_REQUIRED")
        self.assertFalse(r["findings"])
        self.assertEqual(len(r["patch_sha256"]), 64)

    def test_unsafe_block_prevents_automated_admission(self):
        p = self.head + b"@@ -1 +1 @@\n-pub fn value() {}\n+unsafe { std::ptr::write(ptr, 1); }\n"
        r = criticize_patch(p, ["src/worker.rs"])
        self.assertEqual(r["status"], "DENIED_FOR_AUTOMATION")
        self.assertIn("UNSAFE_RUST", [f["rule"] for f in r["findings"]])
        self.assertEqual(r["findings"][0]["new_line"], 1)

    def test_tampering_with_verification_is_flagged(self):
        p = self.head + b"@@ -5,0 +6 @@\n+let tls = client.danger_accept_invalid_certs(true);\n"
        r = criticize_patch(p, ["src/worker.rs"])
        self.assertIn("TLS_VERIFICATION_BYPASS", [f["rule"] for f in r["findings"]])
        self.assertEqual(r["findings"][0]["new_line"], 6)

    def test_deleted_risky_code_does_not_generate_finding(self):
        p = self.head + b"@@ -1 +1 @@\n-unsafe { old(); }\n+pub fn value() {}\n"
        self.assertFalse(criticize_patch(p, ["src/worker.rs"])["findings"])

    def test_multifile_hunks_attribute_findings(self):
        p = self.head + b"@@ -0,0 +1 @@\n+pub fn safe() {}\n" + (
            b"diff --git a/tests/runner.py b/tests/runner.py\n"
            b"new file mode 100644\n--- /dev/null\n+++ b/tests/runner.py\n"
            b"@@ -0,0 +1 @@\n+os.system(cmd)\n"
        )
        r = criticize_patch(p, ["src/worker.rs", "tests/runner.py"])
        self.assertEqual(r["findings"][0]["path"], "tests/runner.py")
        self.assertIn("SHELL_EXECUTION", [f["rule"] for f in r["findings"]])

    def test_secret_literal_is_detected_without_leaking_value(self):
        p = self.head + b"@@ -0,0 +1 @@\n+PRIVATE_KEY = '-----BEGIN PRIVATE KEY-----'\n"
        r = criticize_patch(p, ["src/worker.rs"])
        self.assertIn("POTENTIAL_SECRET", [f["rule"] for f in r["findings"]])
        self.assertNotIn("PRIVATE KEY-----", str(r["findings"]))

    def test_added_line_starting_with_plus_is_not_silently_ignored(self):
        p = self.head + b"@@ -0,0 +1 @@\n+++unsafe { critical(); }\n"
        r = criticize_patch(p, ["src/worker.rs"])
        self.assertIn("UNSAFE_RUST", [f["rule"] for f in r["findings"]])

    def test_end_to_end_review_over_real_git_worktree_is_read_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp) / "repo"
            repo.mkdir()
            def git(*args):
                return subprocess.check_output(["git", "-C", str(repo), *args], stderr=subprocess.PIPE)
            git("init", "-q")
            git("config", "user.email", "test@example.invalid")
            git("config", "user.name", "OCAF test")
            (repo / "src").mkdir()
            f = repo / "src" / "worker.rs"
            f.write_text("pub fn value() -> u32 { 1 }\n")
            git("add", ".")
            git("commit", "-qm", "base")
            base = git("rev-parse", "HEAD").decode().strip()
            f.write_text("pub fn value() -> u32 { 2 }\n")
            patch = Path(tmp) / "candidate.patch"
            patch.write_bytes(git("diff", "--", "src/worker.rs"))
            git("checkout", "--", "src/worker.rs")
            result = review_candidate(repo, patch, base)
            self.assertEqual(result["status"], "SECURITY_REVIEW_REQUIRED")
            self.assertEqual(result["base_sha"], base)
            self.assertEqual(git("status", "--porcelain"), b"")
            self.assertIn("{ 1 }", f.read_text())

    def test_cli_returns_nonzero_until_independent_review(self):
        args = ["critic.py", "--repo", "/repo", "--patch", "/patch", "--base", "a" * 40]
        with mock.patch("sys.argv", args), mock.patch(
            "omnidev_security_critic.review_candidate",
            return_value={"status": "SECURITY_REVIEW_REQUIRED", "findings": []},
        ), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(main(), 4)
        with mock.patch("sys.argv", args), mock.patch(
            "omnidev_security_critic.review_candidate",
            return_value={"status": "DENIED_FOR_AUTOMATION", "findings": [{"rule": "UNSAFE_RUST"}]},
        ), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(main(), 3)

    def test_critic_refuses_mismatch_file_list(self):
        p = self.head + b"@@ -1 +1 @@\n-a\n+b\n"
        with self.assertRaises(ValueError):
            criticize_patch(p, ["src/other.rs"])


if __name__ == "__main__":
    unittest.main()
