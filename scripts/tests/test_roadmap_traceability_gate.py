"""Deterministic fail-closed checks for canonical OCAF source traceability."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from roadmap_traceability_gate import COUNTS, MATRIX, SOURCES, ScopeError, validate


def blob_digest(content: bytes) -> str:
    return hashlib.sha1(b'blob ' + str(len(content)).encode() + b'\0' + content).hexdigest()


class TraceabilityGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.data = {
            'canonical': {
                'main': [{'id': f'{n}', 'title': f'Main {n}', 'status': 'PLANNED',
                          'acceptance': f'Criterion {n}', 'dependencies': []} for n in range(334)],
                'technical': [{'id': f'0.38.{n}', 'title': f'Tech {n}', 'status': 'NO_GO'}
                              for n in range(64)],
                'reference_repositories': [{'name': f'library-{n}', 'test_scope': f'Test {n}'}
                                           for n in range(33)],
            },
            'evolution': {'requirements': [{'id': f'AB-OM-{n:02}', 'title': f'Agent {n}',
                                            'status': 'OPEN_UNVERIFIED'} for n in range(1, 21)]},
            'products': {'products': [{'id': f'AB-TEST-{n}', 'product': f'Product {n}',
                                      'status': 'PARTIAL', 'real_acceptance': f'Accept {n}'}
                                     for n in range(10)]},
            'gaps': {'items': [{'id': f'INV-{n:03}', 'component': f'Gap {n}',
                               'condition': 'MISSING'} for n in range(1, 65)]},
        }
        self.matrix = self.build_matrix()
        self.save()

    def build_matrix(self):
        records = []
        for kind, group, section, key, title in (
            ('main_stage', 'canonical', 'main', 'id', 'title'),
            ('technical_stage', 'canonical', 'technical', 'id', 'title'),
            ('reference_adaptation', 'canonical', 'reference_repositories', 'name', 'name'),
            ('om_evolution_requirement', 'evolution', 'requirements', 'id', 'title'),
            ('agents_builder_variant', 'products', 'products', 'id', 'product'),
            ('missing_capability', 'gaps', 'items', 'id', 'component'),
        ):
            for i, item in enumerate(self.data[group][section], 1):
                item_id = item[key]
                identity = (f'{kind}:{i:03d}:{item_id}' if kind == 'reference_adaptation'
                            else f'{kind}:{item_id}')
                records.append({
                    'record_key': identity,
                    'kind': kind, 'id': item_id, 'title': item[title],
                    'source_manifest': SOURCES[group],
                    'declared_status': item.get('status') or None,
                    'acceptance_source_text': (item.get('acceptance') or
                                               item.get('real_acceptance') or item.get('test_scope')),
                    'scope_source_text': item.get('scope') or item.get('implementation_targets'),
                    'dependencies': item.get('dependencies') or [],
                    'implementation_verified_by_this_matrix': False,
                    'tests_executed_by_this_matrix': False,
                    'release_approved_by_this_matrix': False,
                })
        return {
            'schema_version': 1, 'repository': 'mojealterego/ocaf-runtime', 'branch': 'main',
            'production_approved': False, 'source_commit': 'a' * 40,
            'source_overlaps_expected': True,
            'unique_engineering_work_items_not_claimed': True,
            'counts': COUNTS.copy(), 'total_source_records': len(records), 'records': records,
            'source_manifest_blobs': {},
        }

    def save(self):
        self.matrix['source_manifest_blobs'] = {}
        for label, path in SOURCES.items():
            file = self.root / path
            file.parent.mkdir(parents=True, exist_ok=True)
            data = (json.dumps(self.data[label], ensure_ascii=False) + '\n').encode()
            file.write_bytes(data)
            self.matrix['source_manifest_blobs'][path] = blob_digest(data)
        dest = self.root / MATRIX
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(json.dumps(self.matrix), encoding='utf-8')

    def test_all_525_source_records_are_validated_without_acceptance_claim(self):
        result = validate(self.root)
        self.assertEqual(result['records_verified'], 525)
        self.assertEqual(result['status'], 'PASS_SOURCE_TRACEABILITY_ONLY')
        self.assertFalse(result['implementation_tested'])
        self.assertFalse(result['production_approved'])

    def test_rejects_forged_implementation_status(self):
        self.matrix['records'][0]['implementation_verified_by_this_matrix'] = True
        self.save()
        with self.assertRaisesRegex(ScopeError, 'mismatch'):
            validate(self.root)

    def test_rejects_forged_release_status(self):
        self.matrix['production_approved'] = True
        self.save()
        with self.assertRaisesRegex(ScopeError, 'unsafe acceptance'):
            validate(self.root)

    def test_rejects_modified_acceptance_without_fresh_matrix(self):
        self.matrix['records'][0]['acceptance_source_text'] = 'Never tested but PASS'
        self.save()
        with self.assertRaisesRegex(ScopeError, 'mismatch'):
            validate(self.root)

    def test_rejects_dropped_source_stage(self):
        self.data['canonical']['technical'].pop()
        self.save()
        with self.assertRaisesRegex(ScopeError, 'source count'):
            validate(self.root)

    def test_rejects_missing_record(self):
        self.matrix['records'].pop()
        self.save()
        with self.assertRaisesRegex(ScopeError, 'count'):
            validate(self.root)

    def test_rejects_duplicated_record(self):
        self.matrix['records'][1] = self.matrix['records'][0].copy()
        self.save()
        with self.assertRaisesRegex(ScopeError, 'duplicate|mismatch'):
            validate(self.root)

    def test_rejects_swapped_record_order(self):
        self.matrix['records'][5], self.matrix['records'][6] = self.matrix['records'][6], self.matrix['records'][5]
        self.save()
        with self.assertRaisesRegex(ScopeError, 'mismatch'):
            validate(self.root)

    def test_rejects_source_blob_drift(self):
        self.save()
        p = self.root / SOURCES['canonical']
        p.write_bytes(p.read_bytes() + b' ')
        with self.assertRaisesRegex(ScopeError, 'Git blob drift'):
            validate(self.root)

    def test_rejects_duplicate_json_key(self):
        self.save()
        (self.root / MATRIX).write_text('{"schema_version":1,"schema_version":1}')
        with self.assertRaisesRegex(ScopeError, 'duplicate JSON key'):
            validate(self.root)

    def test_rejects_symlinked_parent_directory(self):
        self.save()
        original = self.root / 'docs' / 'roadmap'
        target = self.root / 'outside-repository-root'
        original.rename(target)
        original.symlink_to(target, target_is_directory=True)
        with self.assertRaisesRegex(ScopeError, 'symlinked'):
            validate(self.root)

    def test_rejects_symlinked_matrix(self):
        self.save()
        p = self.root / MATRIX
        target = self.root / 'external.json'
        p.rename(target)
        p.symlink_to(target)
        with self.assertRaisesRegex(ScopeError, 'symlinked'):
            validate(self.root)

    def test_cli_survives_optimization_and_remains_fail_closed(self):
        self.save()
        cli = Path(__file__).resolve().parents[1] / 'roadmap_traceability_gate.py'
        for optimize in (False, True):
            args = [sys.executable, '-O' if optimize else '-B', str(cli), '--repo', str(self.root)]
            ok = subprocess.run(args, capture_output=True, text=True, timeout=10)
            self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)
            self.assertEqual(json.loads(ok.stdout)['records_verified'], 525)
        self.matrix['records'][20]['tests_executed_by_this_matrix'] = True
        self.save()
        bad = subprocess.run([sys.executable, '-O', str(cli), '--repo', str(self.root)],
                             capture_output=True, text=True, timeout=10)
        self.assertNotEqual(bad.returncode, 0)
        self.assertEqual(json.loads(bad.stdout)['status'], 'FAIL')


if __name__ == '__main__':
    unittest.main()
