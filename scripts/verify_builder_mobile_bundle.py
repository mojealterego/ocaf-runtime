#!/usr/bin/env python3
"""Canonical Android Builder bundle integrity; no installation or signing claim."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import zipfile

if __package__:
    from .verify_native_artifact import verify
else:
    from verify_native_artifact import verify

ASSETS = ('index.html', 'style.css', 'app.mjs', 'editor.mjs', 'strict-json.mjs',
          'preview-gate.mjs', 'cloud.mjs', 'i18n.mjs', 'credentials.mjs',
          'models.mjs', 'integrations-ui.mjs', 'runner-client.mjs',
          'manifest.webmanifest', 'sw.js')
ABIS = {'arm64-v8a': 183, 'x86_64': 62}
ASSET_LIMIT = 2 * 1024 * 1024
LIB_LIMIT = 128 * 1024 * 1024


def verify_mobile_apk(path: Path, web: Path) -> dict:
    """Require the actual shared UI and both architecture-specific Rust libraries."""
    verify('android', path)
    assets = {}
    libraries = {}
    with zipfile.ZipFile(path) as archive:
        names = set(archive.namelist())
        wanted = {'assets/www/' + name for name in ASSETS}
        actual = {name for name in names if name.startswith('assets/www/') and not name.endswith('/')}
        if wanted - actual:
            raise ValueError('missing shared web asset: ' + ', '.join(sorted(wanted - actual)))
        if actual - wanted:
            raise ValueError('unexpected web asset: ' + ', '.join(sorted(actual - wanted)))
        for name in ASSETS:
            source = web / name
            info = archive.getinfo('assets/www/' + name)
            if source.is_symlink() or not source.is_file() or not 0 < source.stat().st_size <= ASSET_LIMIT:
                raise ValueError('invalid shared source asset: ' + name)
            if not 0 < info.file_size <= ASSET_LIMIT:
                raise ValueError('invalid packaged asset size: ' + name)
            raw = archive.read(info)
            if raw != source.read_bytes():
                raise ValueError('asset bytes differ from shared source: ' + name)
            assets[name] = hashlib.sha256(raw).hexdigest()
        for abi, machine in ABIS.items():
            member = f'lib/{abi}/libocaf_builder_mobile_ffi.so'
            if member not in names:
                raise ValueError('missing Rust UniFFI library: ' + abi)
            info = archive.getinfo(member)
            if not 1024 <= info.file_size <= LIB_LIMIT:
                raise ValueError('invalid Rust library size: ' + abi)
            with archive.open(info) as handle:
                header = handle.read(64)
                if len(header) != 64 or header[:7] != b'\x7fELF\x02\x01\x01' or struct.unpack_from('<HHI', header, 16) != (3, machine, 1):
                    raise ValueError('invalid Rust library architecture/type: ' + abi)
                digest = hashlib.sha256(header)
                for chunk in iter(lambda: handle.read(1024 * 1024), b''):
                    digest.update(chunk)
                libraries[abi] = digest.hexdigest()
    return {'status': 'MOBILE_APK_BUNDLE_INTEGRITY_VERIFIED', 'abis': libraries,
            'assets_sha256': assets, 'native_execution_verified': False,
            'production_approved': False}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('apk', type=Path)
    parser.add_argument('--web', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(verify_mobile_apk(args.apk, args.web), sort_keys=True))


if __name__ == '__main__':
    main()
