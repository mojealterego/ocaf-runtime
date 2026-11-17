"""Real ZIP/ELF fixture regressions, not an Android device test."""
from pathlib import Path
import struct
import tempfile
import unittest
import warnings
import zipfile

from scripts.verify_builder_mobile_bundle import ASSETS, verify_mobile_apk


class MobileBundleTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.web = self.root / 'web'
        self.web.mkdir()
        for name in ASSETS:
            (self.web / name).write_text('fixture ' + name, encoding='utf-8')
        self.entries = {'AndroidManifest.xml': b'manifest', 'classes.dex': b'dex\n039\x00' + bytes(128)}
        self.entries.update({'assets/www/' + name: (self.web / name).read_bytes() for name in ASSETS})
        for abi, machine in [('arm64-v8a', 183), ('x86_64', 62)]:
            header = bytearray(64)
            header[:7] = b'\x7fELF\x02\x01\x01'
            struct.pack_into('<HHI', header, 16, 3, machine, 1)
            self.entries['lib/' + abi + '/libocaf_builder_mobile_ffi.so'] = bytes(header) + bytes(1024)
        self.path = self.root / 'app.apk'

    def package(self, extra=None):
        with zipfile.ZipFile(self.path, 'w') as z:
            for name, data in self.entries.items():
                z.writestr(name, data)
            if extra:
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore', UserWarning)
                    z.writestr(*extra)
        return verify_mobile_apk(self.path, self.web)

    def test_complete_package_has_both_abis_and_exact_assets(self):
        report = self.package()
        self.assertEqual(set(report['abis']), {'arm64-v8a', 'x86_64'})
        self.assertEqual(set(report['assets_sha256']), set(ASSETS))
        self.assertFalse(report['production_approved'])
        self.assertFalse(report['native_execution_verified'])

    def test_missing_x86_64_ffi_refused(self):
        del self.entries['lib/x86_64/libocaf_builder_mobile_ffi.so']
        with self.assertRaisesRegex(ValueError, 'missing'):
            self.package()

    def test_wrong_architecture_even_with_elf_magic_refused(self):
        self.entries['lib/x86_64/libocaf_builder_mobile_ffi.so'] = self.entries['lib/arm64-v8a/libocaf_builder_mobile_ffi.so']
        with self.assertRaisesRegex(ValueError, 'architecture'):
            self.package()

    def test_stale_preview_module_refused(self):
        self.entries['assets/www/preview-gate.mjs'] = b'old asset'
        with self.assertRaisesRegex(ValueError, 'asset bytes'):
            self.package()

    def test_missing_strict_json_refused(self):
        del self.entries['assets/www/strict-json.mjs']
        with self.assertRaisesRegex(ValueError, 'missing'):
            self.package()

    def test_unexpected_executable_asset_refused(self):
        self.entries['assets/www/extra.js'] = b'unknown code'
        with self.assertRaisesRegex(ValueError, 'unexpected web asset'):
            self.package()

    def test_duplicate_member_refused(self):
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.package(('assets/www/app.mjs', b'alt'))

    def test_missing_dex_refused(self):
        del self.entries['classes.dex']
        with self.assertRaisesRegex(ValueError, 'DEX'):
            self.package()


if __name__ == '__main__':
    unittest.main()
