"""Isolated security regressions for the roadmap integrity CLI (no real repo mutation)."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


GATE = Path(__file__).resolve().parents[1] / 'roadmap_integrity_gate.py'


class RoadmapIntegritySecurityTests(unittest.TestCase):
    def setUp(self) -> None:
        workspace = tempfile.TemporaryDirectory()
        self.addCleanup(workspace.cleanup)
        self.root = Path(workspace.name)
        self.repo = self.root / 'repo'
        self.manifest_path = self.repo / 'docs/roadmap/canonical_v11_inventory.json'
        self.manifest_path.parent.mkdir(parents=True)
        self.manifest = {
            'main_count': 334,
            'technical_count': 64,
            'reference_count': 33,
            'main': [{'id': str(i)} for i in range(1, 335)],
        }
        self.write_manifest()

    def write_manifest(self) -> None:
        self.manifest_path.write_text(json.dumps(self.manifest), encoding='utf-8')

    def run_gate(self, *, optimized: bool = False, snapshot: Path | None = None,
                 receipt: Path | None = None) -> subprocess.CompletedProcess[str]:
        args = [sys.executable]
        if optimized:
            args.append('-O')
        args.extend([str(GATE), '--repo', str(self.repo)])
        if snapshot is not None:
            args.extend(['--snapshot', str(snapshot)])
        if receipt is not None:
            args.extend(['--receipt', str(receipt)])
        return subprocess.run(args, capture_output=True, text=True, timeout=8)

    def test_valid_manifest_passes_in_normal_and_optimized_modes(self) -> None:
        for optimized in (False, True):
            with self.subTest(optimized=optimized):
                result = self.run_gate(optimized=optimized)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(result.stdout)['status'], 'PASS')

    def test_wrong_count_rejected_even_with_python_optimization(self) -> None:
        self.manifest['main_count'] = 333
        self.write_manifest()
        receipt = self.root / 'should-not-exist.json'
        result = self.run_gate(optimized=True, receipt=receipt)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertFalse(receipt.exists())

    def test_duplicate_ids_rejected_even_with_python_optimization(self) -> None:
        self.manifest['main'][-1]['id'] = self.manifest['main'][0]['id']
        self.write_manifest()
        result = self.run_gate(optimized=True)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_noncanonical_alias_id_rejected(self) -> None:
        self.manifest['main'][1]['id'] = '01'
        self.write_manifest()
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_empty_snapshot_refused_without_receipt(self) -> None:
        snap = self.root / 'snap'
        snap.mkdir()
        (snap / 'source.sha256').write_text('', encoding='utf-8')
        receipt = self.root / 'receipt.json'
        result = self.run_gate(snapshot=snap, receipt=receipt)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertFalse(receipt.exists())

    def test_snapshot_path_escape_is_refused(self) -> None:
        outside = self.root / 'external'
        outside.write_bytes(b'unrelated external content')
        snap = self.root / 'snap'
        snap.mkdir()
        digest = hashlib.sha256(outside.read_bytes()).hexdigest()
        # From repo/ this escapes to workspace/external.
        (snap / 'source.sha256').write_text(f'{digest}  ../external\n', encoding='utf-8')
        result = self.run_gate(snapshot=snap)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_duplicate_snapshot_entries_rejected(self) -> None:
        snap = self.root / 'snap'
        snap.mkdir()
        digest = hashlib.sha256(self.manifest_path.read_bytes()).hexdigest()
        entry = f'{digest}  docs/roadmap/canonical_v11_inventory.json\n'
        (snap / 'source.sha256').write_text(entry + entry, encoding='utf-8')
        result = self.run_gate(snapshot=snap)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_snapshot_symlink_target_rejected(self) -> None:
        outside = self.root / 'external'
        outside.write_bytes(b'outside')
        (self.repo / 'docs' / 'linked').symlink_to(outside)
        snap = self.root / 'snap'
        snap.mkdir()
        digest = hashlib.sha256(outside.read_bytes()).hexdigest()
        (snap / 'source.sha256').write_text(f'{digest}  docs/linked\n', encoding='utf-8')
        result = self.run_gate(snapshot=snap)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_reversed_stage_order_is_rejected_optimized(self) -> None:
        self.manifest['main'][0], self.manifest['main'][1] = (
            self.manifest['main'][1], self.manifest['main'][0])
        self.write_manifest()
        result = self.run_gate(optimized=True)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_wrong_snapshot_digest_is_rejected(self) -> None:
        snap = self.root / 'snap'
        snap.mkdir()
        (snap / 'source.sha256').write_text(
            f'{"0" * 64}  docs/roadmap/canonical_v11_inventory.json\n', encoding='utf-8')
        result = self.run_gate(snapshot=snap)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_malformed_snapshot_digest_is_rejected(self) -> None:
        snap = self.root / 'snap'
        snap.mkdir()
        (snap / 'source.sha256').write_text(
            'not-a-sha256  docs/roadmap/canonical_v11_inventory.json\n', encoding='utf-8')
        result = self.run_gate(snapshot=snap)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_symlink_parent_of_snapshot_target_is_rejected(self) -> None:
        snap = self.root / 'snap'
        snap.mkdir()
        (self.repo / 'alias').symlink_to(self.repo / 'docs', target_is_directory=True)
        digest = hashlib.sha256(self.manifest_path.read_bytes()).hexdigest()
        (snap / 'source.sha256').write_text(
            f'{digest}  alias/roadmap/canonical_v11_inventory.json\n', encoding='utf-8')
        result = self.run_gate(snapshot=snap)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_invalid_json_manifest_cannot_return_pass(self) -> None:
        self.manifest_path.write_text('{bad json', encoding='utf-8')
        result = self.run_gate(optimized=True)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_missing_snapshot_target_is_rejected(self) -> None:
        snap = self.root / 'snap'
        snap.mkdir()
        (snap / 'source.sha256').write_text(
            f'{"0" * 64}  docs/no-such-file.json\n', encoding='utf-8')
        result = self.run_gate(snapshot=snap)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_correct_snapshot_passes_and_writes_receipt(self) -> None:
        snap = self.root / 'snap'
        snap.mkdir()
        digest = hashlib.sha256(self.manifest_path.read_bytes()).hexdigest()
        (snap / 'source.sha256').write_text(
            f'{digest}  docs/roadmap/canonical_v11_inventory.json\n', encoding='utf-8')
        receipt = self.root / 'receipt.json'
        result = self.run_gate(snapshot=snap, receipt=receipt)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(receipt.read_text())['status'], 'PASS')


if __name__ == '__main__':
    unittest.main()
