"""Fail-closed tests for source-ID / canonical-ID reconciliation."""
import json
import tempfile
import unittest
from pathlib import Path

from scripts.verify_source_155_206 import EXPECTED_POINTS, evaluate

CANONICAL = (
    'Transport benchmark SHM/TCP', 'SHM tenancy and quota',
    'Dual CRDT convergence corpus', 'Edge Zenoh interoperability',
    'Transport and CRDT release matrix', 'OTel resource precedence',
    'CoALA typed memory API', 'RQGM immutable examiner epoch',
    'SDNN arithmetic property corpus', 'In guest Aya BPF verifier gate',
    'L402 settlement provenance', 'Cargo Workspace migration boundary',
    'Hyperon/MeTTa neuro-symbolic research adapter',
    'Taskweaving dependency graph and multi-agent rollback',
    'AI Scientist reproducible research lifecycle',
    'OpenHands agent server and legacy repository adapter',
    'Theory-of-Mind evaluation and human approval gates',
    'EideticEngine metacognitive memory-control evaluation',
    'Agent-Zero and Bonfire self-programming research sandbox',
    'NuNet PRIMUS and GodMode experimental orchestration interfaces',
)


class SourceReconciliationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'docs/roadmap').mkdir(parents=True)
        (self.root / 'src').mkdir()
        (self.root / 'src/document_state.rs').write_text('// synthetic fixture', encoding='utf-8')
        self.inventory = {
            'main_count': 334, 'technical_count': 64, 'production_approved': False,
            'main': [
                {'id': point, 'title': title}
                for point, title in zip(EXPECTED_POINTS, CANONICAL)
            ],
        }
        self.manifest = {
            'schema_version': 1,
            'source_kind': 'EXTERNAL_ARCHITECTURE_RECONCILIATION',
            'source_name': 'Architektura Wykonawcza OCAF-Runtime (1.55–2.06)',
            'canonical_inventory': 'docs/roadmap/canonical_v11_inventory.json',
            'production_approved': False,
            'items': [
                {
                    'source_id': f'SOURCE-{point}',
                    'source_title': f'Source requirement {point}',
                    'canonical_id': point,
                    'canonical_title': title,
                    'status': 'EXISTING_PARTIAL',
                    'implementation_paths': ['src/document_state.rs'],
                    'remaining_gaps': ['Runtime integration and acceptance not verified'],
                }
                for point, title in zip(EXPECTED_POINTS, CANONICAL)
            ],
        }
        self.write()

    def write(self):
        (self.root / 'docs/roadmap/canonical_v11_inventory.json').write_text(
            json.dumps(self.inventory), encoding='utf-8'
        )
        (self.root / 'docs/roadmap/ocaf_source_155_206_manifest.json').write_text(
            json.dumps(self.manifest), encoding='utf-8'
        )

    def check(self):
        return evaluate(self.root)

    def test_valid_source_mapping_is_not_stage_acceptance(self):
        report = self.check()
        self.assertEqual(report['status'], 'SOURCE_MAPPING_VALIDATED_ONLY')
        self.assertEqual(report['source_points'], 20)
        self.assertFalse(report['production_approved'])
        self.assertEqual(report['problems'], [])

    def test_duplicate_source_id_cannot_be_hidden(self):
        self.manifest['items'][1]['source_id'] = self.manifest['items'][0]['source_id']
        self.write()
        self.assertEqual(self.check()['status'], 'INVALID_SOURCE_MAPPING')

    def test_mislabelled_canonical_stage_fails(self):
        self.manifest['items'][0]['canonical_title'] = 'OCAFLabelTree'
        self.write()
        self.assertIn('canonical_mismatch:SOURCE-1.55', self.check()['problems'])

    def test_source_cannot_overwrite_canonical_title(self):
        self.manifest['items'][0]['source_title'] = CANONICAL[0]
        self.write()
        self.assertIn('source_collides_with_canonical:SOURCE-1.55', self.check()['problems'])

    def test_stage_count_and_production_approval_fail_closed(self):
        self.inventory['main_count'] = 333
        self.inventory['production_approved'] = True
        self.write()
        self.assertIn('canonical_inventory_invalid', self.check()['problems'])

    def test_missing_canonical_stage_fails(self):
        self.inventory['main'] = self.inventory['main'][1:]
        self.write()
        self.assertIn('canonical_mismatch:SOURCE-1.55', self.check()['problems'])

    def test_unverified_claim_cannot_be_promoted(self):
        self.manifest['items'][0]['status'] = 'CI_VERIFIED'
        self.write()
        self.assertIn('invalid_source_status:SOURCE-1.55', self.check()['problems'])

    def test_missing_code_path_fails(self):
        self.manifest['items'][0]['implementation_paths'] = ['src/missing.rs']
        self.write()
        self.assertIn('implementation_missing:SOURCE-1.55', self.check()['problems'])

    def test_path_traversal_fails(self):
        self.manifest['items'][0]['implementation_paths'] = ['../secret.txt']
        self.write()
        self.assertIn('unsafe_implementation_path:SOURCE-1.55', self.check()['problems'])

    def test_symlink_fails(self):
        (self.root / 'src/link.rs').symlink_to('document_state.rs')
        self.manifest['items'][0]['implementation_paths'] = ['src/link.rs']
        self.write()
        self.assertIn('unsafe_implementation_path:SOURCE-1.55', self.check()['problems'])

    def test_gaps_cannot_be_silently_removed(self):
        self.manifest['items'][0]['remaining_gaps'] = []
        self.write()
        self.assertIn('missing_limitations:SOURCE-1.55', self.check()['problems'])

    def test_source_cannot_approve_production(self):
        self.manifest['production_approved'] = True
        self.write()
        self.assertIn('source_manifest_invalid', self.check()['problems'])

    def test_duplicate_json_properties_rejected(self):
        path = self.root / 'docs/roadmap/ocaf_source_155_206_manifest.json'
        path.write_text('{"schema_version":1,"schema_version":1}', encoding='utf-8')
        self.assertEqual(self.check()['status'], 'INVALID_SOURCE_MAPPING')

    def test_absent_manifest_fails(self):
        (self.root / 'docs/roadmap/ocaf_source_155_206_manifest.json').unlink()
        self.assertEqual(self.check()['status'], 'INVALID_SOURCE_MAPPING')


if __name__ == '__main__':
    unittest.main()
