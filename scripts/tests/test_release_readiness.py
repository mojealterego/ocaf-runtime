import hashlib
import json
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

from scripts.release_readiness import REQUIRED_EVIDENCE, evaluate

NOW = datetime(2026, 10, 13, 12, 0, tzinfo=timezone.utc)


class ReleaseReadinessTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.evidence = self.root / 'private-evidence'
        self.evidence.mkdir()

    def write_dependency_lock(self):
        (self.repo / 'Cargo.lock').write_text('''version = 4
[[package]]
name = "ocaf_runtime"
version = "0.26.0"
[[package]]
name = "serde"
version = "1.0.200"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0123456789abcdef" * 4
'''.replace('"0123456789abcdef" * 4', '"' + '0123456789abcdef' * 4 + '"'), encoding='utf-8')

    def make_manifest(self, stale=False):
        checks = {}
        for name in REQUIRED_EVIDENCE:
            path = self.evidence / (name + '.txt')
            path.write_text('Independent operator evidence for ' + name, encoding='utf-8')
            observed = NOW - (timedelta(days=60) if stale else timedelta(days=1))
            checks[name] = {
                'path': path.name,
                'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                'observed_at_utc': observed.isoformat().replace('+00:00', 'Z'),
            }
        manifest = self.evidence / 'manifest.json'
        manifest.write_text(json.dumps({'schema_version': 1, 'checks': checks}), encoding='utf-8')
        return manifest

    def test_missing_evidence_and_lockfile_fail_closed(self):
        report = evaluate(self.repo, None, NOW)
        self.assertEqual(report['status'], 'NO_GO')
        self.assertIn('cargo_lock_missing', report['blockers'])
        self.assertIn('evidence_manifest_missing', report['blockers'])

    def test_fresh_complete_evidence_is_not_a_production_certification(self):
        self.write_dependency_lock()
        report = evaluate(self.repo, self.make_manifest(), NOW)
        self.assertEqual(report['status'], 'EVIDENCE_COMPLETE_UNVERIFIED')
        self.assertEqual(report['blockers'], [])
        self.assertFalse(report['production_approved'])

    def test_tampered_artifact_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        (self.evidence / (REQUIRED_EVIDENCE[0] + '.txt')).write_text('tampered', encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertEqual(report['status'], 'NO_GO')
        self.assertIn('evidence_hash_mismatch:' + REQUIRED_EVIDENCE[0], report['blockers'])

    def test_stale_evidence_rejected(self):
        self.write_dependency_lock()
        report = evaluate(self.repo, self.make_manifest(stale=True), NOW)
        self.assertEqual(report['status'], 'NO_GO')
        self.assertTrue(any(x.startswith('evidence_stale:') for x in report['blockers']))

    def test_path_escape_rejected_even_when_hash_matches(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text(encoding='utf-8'))
        data['checks'][REQUIRED_EVIDENCE[0]]['path'] = '../escape.txt'
        (self.root / 'escape.txt').write_text('external', encoding='utf-8')
        data['checks'][REQUIRED_EVIDENCE[0]]['sha256'] = hashlib.sha256(b'external').hexdigest()
        manifest.write_text(json.dumps(data), encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_path_unsafe:' + REQUIRED_EVIDENCE[0], report['blockers'])

    def test_duplicate_manifest_keys_are_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        manifest.write_text('{"schema_version": 1, "schema_version": 1, "checks": {}}', encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_manifest_invalid', report['blockers'])

    def test_unknown_checks_are_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        data = json.loads(manifest.read_text(encoding='utf-8'))
        data['checks']['fake_test'] = data['checks'][REQUIRED_EVIDENCE[0]]
        manifest.write_text(json.dumps(data), encoding='utf-8')
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_manifest_invalid', report['blockers'])

    def test_symlink_evidence_rejected(self):
        self.write_dependency_lock()
        manifest = self.make_manifest()
        filename = self.evidence / (REQUIRED_EVIDENCE[0] + '.txt')
        target = self.evidence / 'target.txt'
        target.write_bytes(filename.read_bytes())
        filename.unlink()
        filename.symlink_to(target)
        report = evaluate(self.repo, manifest, NOW)
        self.assertIn('evidence_path_unsafe:' + REQUIRED_EVIDENCE[0], report['blockers'])


if __name__ == '__main__':
    unittest.main()