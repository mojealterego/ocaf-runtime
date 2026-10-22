import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class SbomProvenanceTests(unittest.TestCase):
    def test_generate_and_verify_is_bound_to_commit(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "sbom.json"
            generated = subprocess.run(
                [sys.executable, "scripts/generate_sbom.py", "--repo", str(ROOT), "--output", str(output)],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            payload = json.loads(generated.stdout)
            self.assertGreater(payload["components"], 0)
            sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
            verified = subprocess.run(
                [sys.executable, "scripts/verify_sbom_provenance.py", str(output), "--expected-sha", sha],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertEqual(json.loads(verified.stdout)["license_review"], "OPEN")

    def test_wrong_commit_binding_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "sbom.json"
            subprocess.run(
                [sys.executable, "scripts/generate_sbom.py", "--repo", str(ROOT), "--output", str(output)],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            result = subprocess.run(
                [sys.executable, "scripts/verify_sbom_provenance.py", str(output), "--expected-sha", "0" * 40],
                cwd=ROOT,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 4)


if __name__ == "__main__":
    unittest.main()
