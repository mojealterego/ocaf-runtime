"""Canonical inventory integrity regressions for SOURCE crosswalk validation."""
import json
from pathlib import Path
import tempfile
import unittest

from scripts.verify_source_155_206 import EXPECTED_POINTS, evaluate


class CanonicalInventoryIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'docs/roadmap').mkdir(parents=True)
        (self.root / 'src').mkdir()
        (self.root / 'src/document_state.rs').write_text('synthetic')
        self.inventory = {
            'schema_version': 1, 'repository': 'mojealterego/ocaf-runtime', 'branch': 'main',
            'main_count': 334, 'technical_count': 64, 'production_approved': False,
            'main': [{'id': point, 'title': f'Canonical stage {point}'} for point in EXPECTED_POINTS]
                    + [{'id': f'synthetic-{n}', 'title': f'Canonical stage {n}'} for n in range(314)],
            'technical': [{'id': f'0.38.{n}', 'title': f'Technical stage {n}'} for n in range(64)],
        }
        self.manifest = {
            'schema_version': 1,
            'source_kind': 'EXTERNAL_ARCHITECTURE_RECONCILIATION',
            'source_name': 'External architecture points',
            'canonical_inventory': 'docs/roadmap/canonical_v11_inventory.json',
            'production_approved': False,
            'items': [{
                'source_id': f'SOURCE-{point}',
                'source_title': f'External requirement {point}',
                'canonical_id': point,
                'canonical_title': f'Canonical stage {point}',
                'status': 'EXISTING_PARTIAL',
                'implementation_paths': ['src/document_state.rs'],
                'remaining_gaps': ['Acceptance pending'],
            } for point in EXPECTED_POINTS],
        }
        self.save()

    def save(self):
        (self.root / 'docs/roadmap/canonical_v11_inventory.json').write_text(json.dumps(self.inventory))
        (self.root / 'docs/roadmap/ocaf_source_155_206_manifest.json').write_text(json.dumps(self.manifest))

    def test_clean_inventory_still_validates_crosswalk_only(self):
        self.assertEqual(evaluate(self.root)['status'], 'SOURCE_MAPPING_VALIDATED_ONLY')

    def test_duplicate_technical_stage_rejected(self):
        self.inventory['technical'][1]['id'] = self.inventory['technical'][0]['id']
        self.save()
        self.assertIn('canonical_inventory_invalid', evaluate(self.root)['problems'])

    def test_missing_technical_stage_title_rejected(self):
        del self.inventory['technical'][0]['title']
        self.save()
        self.assertIn('canonical_inventory_invalid', evaluate(self.root)['problems'])

    def test_nonstring_unmapped_main_id_rejected(self):
        self.inventory['main'][100]['id'] = 123
        self.save()
        self.assertIn('canonical_inventory_invalid', evaluate(self.root)['problems'])

    def test_wrong_repository_or_branch_rejected(self):
        self.inventory['repository'] = 'attacker/repo'
        self.save()
        self.assertIn('canonical_inventory_invalid', evaluate(self.root)['problems'])
        self.inventory['repository'] = 'mojealterego/ocaf-runtime'
        self.inventory['branch'] = 'untrusted'
        self.save()
        self.assertIn('canonical_inventory_invalid', evaluate(self.root)['problems'])


if __name__ == '__main__':
    unittest.main()
