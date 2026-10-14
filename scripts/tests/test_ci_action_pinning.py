"""Supply-chain admission for GitHub Actions participating in OCAF stage 0.38.

This checks mutable action/container references and overbroad OIDC grants before
a security-sensitive workflow is permitted to merge on main.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
ACTION_REF = re.compile(r"^[^@\s]+@[0-9a-f]{40}$")
IMAGE_DIGEST = re.compile(r"^.+@sha256:[0-9a-f]{64}$")


class CiReferenceIntegrityTests(unittest.TestCase):
    def test_all_external_actions_are_immutable_commit_refs(self):
        files = sorted(WORKFLOWS.glob("*.yml"))
        self.assertTrue(files, "expected GitHub Actions workflow files")
        problems = []
        for workflow in files:
            for lineno, line in enumerate(workflow.read_text().splitlines(), 1):
                match = re.match(r"^\s*(?:-\s*)?uses:\s*([^\s#]+)", line)
                if match:
                    ref = match.group(1)
                    if ref.startswith("./"):
                        continue
                    if not ACTION_REF.fullmatch(ref):
                        problems.append(f"{workflow.name}:{lineno}: {ref}")
        self.assertFalse(problems, "mutable GitHub Action refs:\n" + "\n".join(problems))

    def test_all_service_images_are_digest_pinned(self):
        problems = []
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            for lineno, line in enumerate(workflow.read_text().splitlines(), 1):
                match = re.match(r"^\s*image:\s*([^\s#]+)", line)
                if match and not IMAGE_DIGEST.fullmatch(match.group(1)):
                    problems.append(f"{workflow.name}:{lineno}: {match.group(1)}")
        self.assertFalse(problems, "mutable container images:\n" + "\n".join(problems))

    def test_workflow_level_permissions_do_not_grant_oidc(self):
        problems = []
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            body = workflow.read_text()
            for match in re.finditer(r"(?m)^permissions:\s*\n((?:[ \t]+[^\n]*\n)*)", body):
                if re.search(r"(?m)^\s+id-token:\s*write\s*$", match.group(1)):
                    problems.append(workflow.name)
        self.assertFalse(problems, "workflow-wide OIDC grants:\n" + "\n".join(problems))


    def test_checkout_never_persists_repository_credentials(self):
        failures = []
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            rows = workflow.read_text().splitlines()
            for index, line in enumerate(rows):
                if re.search(r"^\s*(?:-\s*)?uses:\s*actions/checkout@", line):
                    indent = len(line) - len(line.lstrip())
                    next_lines = rows[index + 1:index + 6]
                    if not any(re.match(r"^\s+persist-credentials:\s*false\s*$", row) for row in next_lines):
                        failures.append(f"{workflow.name}:{index + 1}")
        self.assertFalse(failures, "checkout retains repository credentials:\n" + "\n".join(failures))


if __name__ == "__main__":
    unittest.main()
