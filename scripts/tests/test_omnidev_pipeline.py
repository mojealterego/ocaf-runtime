"""OmniDev 0.56 — real generated Git patch, security, durable checkpoint and sandbox."""
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omnidev_pipeline import DockerExecutor, OmniDevError, Repair, Stage056, generate


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args],
                                   stderr=subprocess.PIPE).decode().strip()


class Stage056PipelineTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        git(self.repo, "init", "-q")
        git(self.repo, "config", "user.email", "test@example.invalid")
        git(self.repo, "config", "user.name", "OCAF Tests")
        (self.repo / "src").mkdir()
        (self.repo / "src" / "worker.py").write_text("def value():\n    return 1\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-qm", "base")
        self.base = git(self.repo, "rev-parse", "HEAD")
        self.key = b"k" * 48
        self.journal = Stage056(self.root / "db.sqlite", self.key)
        self.addCleanup(self.journal.db.close)
        self.repair = Repair("tenantA", "task01", self.base, "src/worker.py",
                             "return 1", "return 2")
        self.command = ("python3", "-c", "print(1)")

    def token(self, info, command=None):
        return self.journal.token("tenantA", "task01", self.base,
                                  info["patch_sha256"], command or self.command,
                                  "python:3.13-slim")

    def test_real_diff_admission_and_clean_host(self):
        patch = generate(self.repo, self.repair)
        self.assertIn(b"+    return 2", patch)
        info = self.journal.submit(self.repo, self.repair)
        self.assertEqual(info["status"], "REVIEW_REQUIRED")
        self.assertEqual(info["security"], "SECURITY_REVIEW_REQUIRED")
        self.assertEqual(self.journal.submit(self.repo, self.repair)["patch_sha256"],
                         info["patch_sha256"])
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")
        self.assertTrue(self.journal.verify())

    def test_operator_approval_and_replay_without_second_execution(self):
        info = self.journal.submit(self.repo, self.repair)
        class Executor:
            image = "python:3.13-slim"
            times = 0
            def run(self, repo, base, patch, command):
                self.times += 1
                assert b"return 2" in patch and command == ("python3", "-c", "print(1)")
                return {"exit_code": 0, "sandbox": "SIMULATED_TEST_DOUBLE"}
        executor = Executor()
        token = self.token(info)
        self.assertEqual(self.journal.execute(
            self.repo, "tenantA", "task01", token, self.command, executor)["status"],
            "TESTED_PASS")
        self.assertEqual(self.journal.execute(
            self.repo, "tenantA", "task01", token, self.command, executor)["status"],
            "TESTED_PASS")
        self.assertEqual(executor.times, 1)
        self.assertTrue(self.journal.verify())
        copy = Stage056(self.root / "db.sqlite", self.key)
        try:
            self.assertEqual(copy.status("tenantA", "task01")["status"], "TESTED_PASS")
        finally:
            copy.db.close()
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")

    def test_tenant_token_and_command_are_bound(self):
        info = self.journal.submit(self.repo, self.repair)
        with self.assertRaises(OmniDevError):
            self.journal.status("tenantB", "task01")
        with self.assertRaises(OmniDevError):
            self.journal.execute(self.repo, "tenantB", "task01",
                                 self.token(info), self.command, DockerExecutor())
        with self.assertRaises(OmniDevError):
            self.journal.execute(self.repo, "tenantA", "task01",
                                 "0" * 64, self.command, DockerExecutor())
        with self.assertRaises(OmniDevError):
            self.journal.execute(self.repo, "tenantA", "task01", self.token(info),
                                 ("python3", "-c", "print(2)"), DockerExecutor())
        self.assertEqual(self.journal.status("tenantA", "task01")["status"],
                         "REVIEW_REQUIRED")

    def test_unsafe_and_ambiguous_generation_denied(self):
        attack = Repair("tenantA", "danger", self.base, "src/worker.py",
                        "return 1", 'return __import__("os").system("id")')
        with self.assertRaises(OmniDevError):
            self.journal.submit(self.repo, attack)
        with self.assertRaises(OmniDevError):
            generate(self.repo, Repair("tenantA", "bad", self.base, "src/worker.py",
                                       "not found", "replacement"))
        with self.assertRaises(OmniDevError):
            generate(self.repo, Repair("tenantA", "bad2", self.base, "src/worker.py",
                                       "return 1", "return ("))

    def test_timeout_recovery_is_fail_closed_and_not_automatically_retried(self):
        info = self.journal.submit(self.repo, self.repair)
        class Executor:
            image = "python:3.13-slim"
            def run(self, *args):
                raise TimeoutError("uncertain effect")
        self.assertEqual(self.journal.execute(
            self.repo, "tenantA", "task01", self.token(info),
            self.command, Executor())["status"], "UNCERTAIN")
        with self.assertRaisesRegex(OmniDevError, "retry"):
            self.journal.execute(self.repo, "tenantA", "task01", self.token(info),
                                 self.command, Executor())

    def test_corrupt_candidate_bytes_cannot_use_previous_approval(self):
        info = self.journal.submit(self.repo, self.repair)
        self.journal.db.execute(
            "UPDATE jobs SET patch=? WHERE tenant='tenantA' AND task='task01'",
            (b"malicious altered candidate",))
        with self.assertRaisesRegex(OmniDevError, "fingerprint"):
            self.journal.execute(self.repo, "tenantA", "task01",
                                 self.token(info), self.command, DockerExecutor())

    def test_recovery_needs_evidence_bound_operator_signature_and_remains_terminal(self):
        info = self.journal.submit(self.repo, self.repair)
        class Uncertain:
            image = "python:3.13-slim"
            def run(self, *args):
                raise TimeoutError("uncertain state")
        self.journal.execute(self.repo, "tenantA", "task01", self.token(info),
                             self.command, Uncertain())
        evidence = "f" * 64
        recovery_token = self.journal.token(
            "tenantA", "task01", self.base, info["patch_sha256"],
            ("RECONCILE", "NO_EFFECT_DECLARED", evidence), "operator")
        with self.assertRaises(OmniDevError):
            self.journal.reconcile("tenantA", "task01", evidence,
                                   "EFFECT_DECLARED", recovery_token)
        self.assertEqual(self.journal.reconcile(
            "tenantA", "task01", evidence, "NO_EFFECT_DECLARED",
            recovery_token)["status"], "RECONCILED_NO_EFFECT_DECLARED")
        with self.assertRaises(OmniDevError):
            self.journal.execute(self.repo, "tenantA", "task01", self.token(info),
                                 self.command, DockerExecutor())
        self.assertTrue(self.journal.verify())

    def test_tamper_detection_and_docker_restrictions(self):
        self.journal.submit(self.repo, self.repair)
        self.journal.db.execute("UPDATE events SET event='forged' WHERE serial=1")
        self.assertFalse(self.journal.verify())
        args = DockerExecutor().arguments(Path("/tmp/work"), "sandbox01",
                                          ("python3", "-c", "print(1)"))
        for value in ("--read-only", "none", "ALL", "no-new-privileges",
                      "65534:65534", "512m"):
            self.assertIn(value, args)
        self.assertNotIn("docker.sock", " ".join(args))


class DockerIntegration(Stage056PipelineTests):

    def test_real_rust_candidate_is_compiled_and_tested_in_isolated_guest(self):
        if not shutil.which("docker"):
            self.skipTest("Docker required for genuine sandbox compilation CI")
        rustrepo = self.root / "rustrepo"
        rustrepo.mkdir()
        git(rustrepo, "init", "-q")
        git(rustrepo, "config", "user.email", "test@example.invalid")
        git(rustrepo, "config", "user.name", "OCAF Rust Fixture")
        (rustrepo / "src").mkdir()
        (rustrepo / "src" / "worker.rs").write_text(
            "pub fn value() -> u32 { 1 }\n"
            "#[cfg(test)] mod tests { #[test] fn check() { "
            "assert_eq!(super::value(), 2); } }\n")
        (rustrepo / "Cargo.toml").write_text(
            "[package]\nname = \"ocaf_sandbox_fixture\"\n"
            "version = \"0.1.0\"\nedition = \"2021\"\n"
            "[lib]\npath = \"src/worker.rs\"\n")
        (rustrepo / "Cargo.lock").write_text(
            "version = 4\n\n[[package]]\nname = \"ocaf_sandbox_fixture\"\n"
            "version = \"0.1.0\"\n")
        git(rustrepo, "add", ".")
        git(rustrepo, "commit", "-qm", "base")
        base = git(rustrepo, "rev-parse", "HEAD")
        journal = Stage056(self.root / "rust.db", self.key)
        try:
            request = Repair("tenantA", "rust01", base, "src/worker.rs",
                             "pub fn value() -> u32 { 1 }",
                             "pub fn value() -> u32 { 2 }")
            item = journal.submit(rustrepo, request)
            command = ("cargo", "test", "--locked", "--offline", "--quiet")
            image = "rust:1.99-slim"
            token = journal.token("tenantA", "rust01", base,
                                  item["patch_sha256"], command, image)
            result = journal.execute(rustrepo, "tenantA", "rust01", token,
                                     command, DockerExecutor(image))
            self.assertEqual(result["status"], "TESTED_PASS", result)
            self.assertEqual(result["result"]["exit_code"], 0)
            self.assertEqual(git(rustrepo, "status", "--porcelain"), "")
        finally:
            journal.db.close()

    def test_real_docker_executes_patched_program_in_no_network_sandbox(self):
        if not shutil.which("docker"):
            self.skipTest("Docker unavailable; must run on dedicated CI")
        info = self.journal.submit(self.repo, self.repair)
        command = ("python3", "-c",
                   "from pathlib import Path; p=Path('src/worker.py').read_text();"
                   "assert 'return 2' in p;compile(p,'worker.py','exec')")
        token = self.token(info, command)
        result = self.journal.execute(
            self.repo, "tenantA", "task01", token, command, DockerExecutor())
        self.assertEqual(result["status"], "TESTED_PASS", result)
        self.assertEqual(result["result"]["exit_code"], 0)


if __name__ == "__main__":
    unittest.main()
