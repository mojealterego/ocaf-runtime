"""Regression guard for the CycloneDX SBOM GitHub Actions artifact path.

cargo-cyclonedx 0.5.9 treats --override-filename as a basename and
appends the selected format extension even if the basename contains a dot.
"""
from pathlib import Path
import re
import unittest


WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/supply-chain.yml"


class SbomWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_sbom_filename_matches_cyclonedx_059_output(self):
        self.assertIn(
            "cargo install --locked cargo-cyclonedx --version 0.5.9",
            self.workflow,
        )
        self.assertIn("cargo cyclonedx --format json --all-features --target all --all", self.workflow)
        self.assertIn("--spec-version 1.5", self.workflow)
        match = re.search(r"--override-filename\s+([A-Za-z0-9._-]+)", self.workflow)
        self.assertIsNotNone(match)
        # The CLI unconditionally appends .json for JSON format, even for
        # overrides such as 'bom.json' (which becomes 'bom.json.json').
        self.assertEqual(match.group(1) + ".json", "bom.json")

    def test_expected_sbom_is_validated_and_attested(self):
        self.assertIn("test -s bom.json", self.workflow)
        self.assertIn("python3 -m scripts.verify_workspace_sbom --repo .", self.workflow)
        self.assertIn('b.get("bomFormat") == "CycloneDX"', self.workflow)
        self.assertIn('b.get("specVersion") == "1.5"', self.workflow)
        self.assertIn("mv bom.json dist/ocaf_runtime.cdx.json", self.workflow)
        self.assertIn("sbom-path: dist/ocaf_runtime.cdx.json", self.workflow)
        self.assertIn("if-no-files-found: error", self.workflow)


if __name__ == "__main__":
    unittest.main()
