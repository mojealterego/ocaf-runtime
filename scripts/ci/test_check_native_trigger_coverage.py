import tempfile
import unittest
from pathlib import Path
from check_native_trigger_coverage import REQUIRED, audit, push_paths


class TriggerAuditTests(unittest.TestCase):
    def test_extracts_push_only_not_dispatch_or_pr(self):
        source = 'name: sample\non:\n  push:\n    branches: [main]\n    paths:\n      - "Cargo.toml"\n  pull_request:\n    paths:\n      - "Cargo.lock"\n'
        self.assertEqual(push_paths(source), {'Cargo.toml'})

    def test_rejects_missing_push_paths(self):
        with self.assertRaises(ValueError):
            push_paths('on:\n  workflow_dispatch:\n  pull_request:\n    paths:\n      - "Cargo.toml"\n')

    def test_rejects_nonliteral_path(self):
        with self.assertRaises(ValueError):
            push_paths('on:\n  push:\n    paths:\n      - *aliases\n')

    def test_rejects_duplicate_on_after_jobs(self):
        source = 'on:\n  push:\n    paths:\n      - "Cargo.toml"\njobs:\n  build:\n    runs-on: ubuntu-latest\non:\n  workflow_dispatch:\n'
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            push_paths(source)

    def test_rejects_duplicate_on_after_name(self):
        source = 'on:\n  push:\n    paths:\n      - "Cargo.toml"\nname: audit\non:\n  push:\n    paths:\n      - "Cargo.lock"\n'
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            push_paths(source)

    def test_accepts_single_on_with_unrelated_top_level_fields(self):
        source = 'name: audit\non:\n  push:\n    paths:\n      - "Cargo.toml"\njobs:\n  build:\n    runs-on: ubuntu-latest\n'
        self.assertEqual(push_paths(source), {'Cargo.toml'})

    def test_rejects_duplicate_push_mapping(self):
        source = 'on:\\n  push:\\n    paths:\\n      - "Cargo.toml"\\n  push:\\n    paths:\\n      - "Cargo.lock"\\n'
        with self.assertRaisesRegex(ValueError, 'duplicate push'):
            push_paths(source)

    def test_rejects_duplicate_push_paths_mapping(self):
        source = 'on:\\n  push:\\n    paths:\\n      - "Cargo.toml"\\n    paths:\\n      - "Cargo.lock"\\n'
        with self.assertRaisesRegex(ValueError, 'duplicate push paths'):
            push_paths(source)

    def test_detects_all_missing_dependencies(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            folder = root / '.github' / 'workflows'
            folder.mkdir(parents=True)
            for filename in REQUIRED:
                (folder / filename).write_text('on:\n  push:\n    paths:\n      - "Cargo.toml"\n')
            result = audit(root)
            self.assertEqual(set(result), set(REQUIRED))
            for missing in result.values():
                self.assertEqual(missing, ['Cargo.lock', 'rust-toolchain.toml'])

    def test_complete_coverage_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            folder = root / '.github' / 'workflows'
            folder.mkdir(parents=True)
            for filename, deps in REQUIRED.items():
                (folder / filename).write_text('on:\n  push:\n    paths:\n' + ''.join(f'      - "{dep}"\n' for dep in deps))
            self.assertEqual(audit(root), {})


if __name__ == '__main__':
    unittest.main()
