"""Integration tests for complete source selection and fail-closed ZIP publication."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / 'package_agents_builder.py'
SPEC = importlib.util.spec_from_file_location('packager', SCRIPT)
assert SPEC and SPEC.loader
packager = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(packager)

# Independent acceptance anchors. Desktop host is shared across Windows,
# macOS, Linux and Kali; Kali has a separate packaging path.
ANCHORS = (
    'apps/agents-builder-desktop/src-tauri/src/main.rs',
    'apps/agents-builder-desktop/src-tauri/tauri.conf.json',
    'scripts/package_builder_kali.py',
    'apps/android/app/src/main/java/com/ocaf/builder/MainActivity.kt',
    'apps/ios/Sources/OCAFBuilderApp.swift',
    'crates/ocaf-builder-mobile-ffi/src/lib.rs',
    'apps/agents-builder-harmonyos/entry/src/main/ets/pages/BuilderPage.ets',
    'apps/agents-builder-kaios/manifest.webapp',
    'apps/agents-builder-web/strict-json.mjs',
    'apps/agents-builder-web/preview-gate.mjs',
)


class PlatformCoverage(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name) / 'repo'
        self.repo.mkdir()
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.name', 'OCAF test')
        self.git('config', 'user.email', 'fixture@example.invalid')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], stderr=subprocess.STDOUT)

    def create_fixture(self, missing=None):
        for path in set(packager.REQUIRED_BUILDER_FILES) | set(ANCHORS) | {'unrelated.txt'}:
            if path != missing:
                output = self.repo / path
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text('fixture: ' + path + '\n', encoding='utf-8')
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')

    def package(self, out):
        return subprocess.run([sys.executable, str(SCRIPT), '--repo', str(self.repo),
                               '--out', str(out), '--expected-sha', self.git('rev-parse', 'HEAD').decode().strip()],
                              check=False, capture_output=True, text=True, timeout=30)

    def test_all_platform_anchors_are_selected_and_required(self):
        for path in ANCHORS:
            with self.subTest(path=path):
                self.assertTrue(packager.select_builder(path))
                self.assertIn(path, packager.REQUIRED_BUILDER_FILES)
        self.assertFalse(packager.select_builder('apps/android-other/private.txt'))
        self.assertFalse(packager.select_builder('crates/other/src/lib.rs'))

    def test_real_zip_contains_all_anchors_and_hashes(self):
        self.create_fixture()
        out = Path(self.tmp.name) / 'release'
        result = self.package(out)
        self.assertEqual(result.returncode, 0, result.stderr)
        bundle = next(out.glob('ocaf-agents-builder-*.zip'))
        with zipfile.ZipFile(bundle) as archive:
            manifest_name = next(name for name in archive.namelist() if name.endswith('/RELEASE_MANIFEST.json'))
            manifest = json.loads(archive.read(manifest_name))
            records = {row['path']: row for row in manifest['files']}
            prefix = manifest_name.removesuffix('RELEASE_MANIFEST.json')
            self.assertEqual(manifest['source_commit'], self.git('rev-parse', 'HEAD').decode().strip())
            for path in ANCHORS:
                self.assertIn(path, records)
                self.assertEqual(records[path]['sha256'], hashlib.sha256(archive.read(prefix + path)).hexdigest())
            self.assertNotIn('unrelated.txt', records)
        other = Path(self.tmp.name) / 'repeat'
        self.assertEqual(self.package(other).returncode, 0)
        self.assertEqual(bundle.read_bytes(), (other / bundle.name).read_bytes())

    def test_missing_platform_source_prevents_any_release(self):
        self.create_fixture()
        for number, path in enumerate(ANCHORS):
            with self.subTest(path=path):
                self.git('rm', '-q', '--', path)
                self.git('commit', '-qm', 'missing anchor')
                out = Path(self.tmp.name) / f'release-{number}'
                result = self.package(out)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(path, result.stderr)
                self.assertFalse(out.exists())
                self.git('revert', '--no-edit', 'HEAD')


if __name__ == '__main__':
    unittest.main()
