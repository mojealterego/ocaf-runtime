"""Ensure the roadmap integrity security test cannot be silently excluded from steward CI."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/ocaf-repository-steward.yml'
REQUIRED = {
    'scripts/roadmap_integrity_gate.py',
    'scripts/tests/test_roadmap_integrity_security.py',
    'scripts/tests/test_steward_trigger_coverage.py',
}


class StewardTriggerCoverageTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = WORKFLOW.read_text(encoding='utf-8')

    def paths_for(self, event: str, successor: str) -> set[str]:
        start = self.source.index(f'  {event}:\n')
        end = self.source.index(f'  {successor}:\n', start)
        fragment = self.source[start:end]
        self.assertIn('    paths:\n', fragment)
        return set(re.findall(r'^      - ([^\n]+)$', fragment, re.MULTILINE))

    def test_push_paths_cover_validator_and_both_regression_suites(self) -> None:
        self.assertTrue(REQUIRED.issubset(self.paths_for('push', 'pull_request')))

    def test_pr_paths_cover_validator_and_both_regression_suites(self) -> None:
        self.assertTrue(REQUIRED.issubset(self.paths_for('pull_request', 'schedule')))

    def test_governance_job_executes_both_security_suites_before_validator(self) -> None:
        for name in ('test_roadmap_integrity_security.py', 'test_steward_trigger_coverage.py'):
            command = f'python3 -m unittest discover -s scripts/tests -p {name} -v'
            self.assertIn(command, self.source)
            self.assertLess(self.source.index(command),
                            self.source.index('python3 scripts/roadmap_integrity_gate.py --repo .'))


if __name__ == '__main__':
    unittest.main()
