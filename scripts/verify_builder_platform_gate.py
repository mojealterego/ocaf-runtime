#!/usr/bin/env python3
"""Eight-platform signed evidence gate; integrity only, never production approval."""
import argparse
import hashlib
import hmac
import json
import os
from pathlib import Path
import re
import stat
import subprocess

MATRIX = 'docs/roadmap/agents_builder_target_platforms_v1.json'
PLATFORMS = {
    'windows': 'windows-native', 'macos': 'macos-native', 'linux': 'linux-elf',
    'kali-linux': 'kali-deb-amd64', 'android': 'android-apk', 'ios': 'ios-ipa',
    'harmonyos': 'harmonyos-hap', 'kaios': 'kaios-webapp',
}
COMBINATIONS = tuple((m, w) for m in ('Code', 'NoCode') for w in ('Local', 'Cloud'))
JSON_LIMIT = 8 * 1024 * 1024
BINARY_LIMIT = 512 * 1024 * 1024


class GateError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise GateError(message)


def steps_for(workspace):
    prefix = ('authenticate', 'tenant_isolation') if workspace == 'Cloud' else ('offline',)
    return prefix + ('create', 'edit', 'validate', 'compile', 'export', 'import')


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode('utf-8')


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def parse(raw):
    def invalid(_):
        raise GateError('nonfinite JSON')
    try:
        value = json.loads(raw.decode('utf-8'), object_pairs_hook=unique, parse_constant=invalid)
        require(type(value) is dict, 'expected JSON object')
        return value
    except (ValueError, RecursionError) as exc:
        raise GateError('invalid evidence JSON') from exc


def fields(value, expected):
    require(type(value) is dict and set(value) == set(expected.split()), 'unexpected evidence fields')


def hex_value(value, length):
    return type(value) is str and re.fullmatch('[0-9a-f]{' + str(length) + '}', value) is not None


def identity(value):
    return type(value) is str and 3 <= len(value) <= 256 and bool(value.strip())


def git(repo, *args):
    try:
        # Do not allow inherited Git controls or replacement refs to redirect provenance.
        env = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
        return subprocess.check_output(
            ['git', '--no-replace-objects', '-C', str(repo), *args],
            stderr=subprocess.PIPE, timeout=10, env=env)
    except (OSError, subprocess.SubprocessError) as exc:
        raise GateError('Git provenance check failed') from exc


def read_checked(root, path, limit, collect=True):
    require(type(path) is str and 0 < len(path) <= 240 and all(
        re.fullmatch(r'[A-Za-z0-9_.-]+', part) and part not in ('.', '..')
        for part in path.split('/')), 'unsafe evidence path')
    require(hasattr(os, 'O_NOFOLLOW') and hasattr(os, 'O_DIRECTORY'), 'secure file opening unavailable')
    fds = []
    try:
        flags = os.O_RDONLY | os.O_NOFOLLOW
        fds.append(os.open(root, flags | os.O_DIRECTORY))
        parts = path.split('/')
        for part in parts[:-1]:
            fds.append(os.open(part, flags | os.O_DIRECTORY, dir_fd=fds[-1]))
        fds.append(os.open(parts[-1], flags | os.O_NONBLOCK, dir_fd=fds[-1]))
        before = os.fstat(fds[-1])
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit, 'invalid evidence file size or kind')
        digest = hashlib.sha256()
        chunks = []
        total = 0
        with os.fdopen(os.dup(fds[-1]), 'rb') as handle:
            while chunk := handle.read(min(1024 * 1024, limit + 1 - total)):
                total += len(chunk)
                require(total <= limit, 'oversized evidence')
                digest.update(chunk)
                if collect:
                    chunks.append(chunk)
        after = os.fstat(fds[-1])
        require(total == before.st_size == after.st_size and before.st_mtime_ns == after.st_mtime_ns
                and before.st_ctime_ns == after.st_ctime_ns, 'evidence changed during read')
        return b''.join(chunks), digest.hexdigest()
    except OSError as exc:
        raise GateError('cannot securely read evidence') from exc
    finally:
        for fd in reversed(fds):
            os.close(fd)


def check_head(repo, head):
    root = git(repo, 'rev-parse', '--show-toplevel').rstrip(b'\n')
    require(root == os.fsencode(Path(repo).resolve()), 'wrong repository root')
    require(git(repo, 'rev-parse', 'HEAD^{commit}').strip() == head.encode() and
            git(repo, 'symbolic-ref', 'HEAD').strip() == b'refs/heads/main', 'wrong branch or SHA')


def check_matrix(repo, head):
    raw, _ = read_checked(repo, MATRIX, JSON_LIMIT)
    require(raw == git(repo, 'show', head + ':' + MATRIX), 'matrix differs from pinned Git tree')
    value = parse(raw)
    require(type(value.get('schema_version')) is int and value['schema_version'] == 1
            and value.get('platform_target_count') == 8 and value.get('mode_switch') == ['Code', 'NoCode']
            and value.get('workspace_switch') == ['Local', 'Cloud'] and type(value.get('cloud')) is dict
            and value['cloud'].get('separate_native_application') is False, 'wrong platform policy')
    targets = value.get('targets')
    require(type(targets) is list and len(targets) == 8, 'incomplete target matrix')
    found = {}
    for item in targets:
        require(type(item) is dict and type(item.get('platform_id')) is str, 'invalid platform matrix row')
        require(item['platform_id'] not in found, 'duplicate matrix platform')
        found[item['platform_id']] = item.get('distribution')
    require(found == PLATFORMS, 'platform matrix mismatch')


def verify(repo, release, head, public_hex):
    from cryptography.exceptions import InvalidSignature
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
    require(hex_value(head, 40) and hex_value(public_hex, 64), 'invalid pinned SHA or trust key')
    check_head(repo, head)
    check_matrix(repo, head)
    raw, _ = read_checked(release, 'PLATFORM_EVIDENCE.json', JSON_LIMIT)
    manifest = parse(raw)
    fields(manifest, 'schema_version git_commit_sha platforms signature_hex')
    require(type(manifest['schema_version']) is int and manifest['schema_version'] == 2 and
            manifest['git_commit_sha'] == head and hex_value(manifest['signature_hex'], 128), 'invalid evidence manifest')
    unsigned = {key: value for key, value in manifest.items() if key != 'signature_hex'}
    try:
        Ed25519PublicKey.from_public_bytes(bytes.fromhex(public_hex)).verify(
            bytes.fromhex(manifest['signature_hex']), canonical(unsigned))
    except (InvalidSignature, ValueError, UnicodeError) as exc:
        raise GateError('invalid signed manifest') from exc
    records = manifest['platforms']
    require(type(records) is list and len(records) == 8, 'exactly eight platforms required')
    seen = set()
    paths = {'PLATFORM_EVIDENCE.json'}
    packages = set()
    count = 0

    def payload(ref, binary=False):
        fields(ref, 'path sha256')
        require(type(ref['path']) is str and ref['path'] not in paths and hex_value(ref['sha256'], 64), 'duplicate or invalid evidence reference')
        paths.add(ref['path'])
        data, digest = read_checked(release, ref['path'], BINARY_LIMIT if binary else JSON_LIMIT, not binary)
        require(hmac.compare_digest(digest, ref['sha256']), 'payload SHA-256 mismatch')
        return digest if binary else parse(data)

    for record in records:
        fields(record, 'platform distribution binary install sbom suites')
        p = record['platform']
        require(type(p) is str and p in PLATFORMS and p not in seen and record['distribution'] == PLATFORMS[p], 'wrong or duplicate platform')
        seen.add(p)
        binary = payload(record['binary'], True)
        require(binary not in packages, 'identical cross-platform package')
        packages.add(binary)
        basis = {'platform': p, 'git_commit_sha': head, 'binary_sha256': binary, 'status': 'PASS'}
        install = payload(record['install'])
        fields(install, 'platform git_commit_sha binary_sha256 status distribution runner_id device_id')
        require(all(install[k] == v for k, v in basis.items()) and install['distribution'] == PLATFORMS[p]
                and identity(install['runner_id']) and identity(install['device_id']), 'invalid install evidence')
        sbom = payload(record['sbom'])
        require(sbom.get('bomFormat') == 'CycloneDX' and sbom.get('specVersion') in ('1.5', '1.6') and
                type(sbom.get('version')) is int and sbom['version'] >= 1 and type(sbom.get('components')) is list
                and bool(sbom['components']) and all(type(x) is dict and identity(x.get('name')) and
                type(x.get('version')) is str and bool(x['version'].strip()) for x in sbom['components']), 'invalid SBOM')
        suites = record['suites']
        require(type(suites) is list and len(suites) == 4, 'four mode/workspace suites required')
        covered = set()
        for suite in suites:
            fields(suite, 'mode workspace log')
            pair = (suite['mode'], suite['workspace'])
            require(all(type(x) is str for x in pair) and pair in COMBINATIONS and pair not in covered, 'invalid or duplicate mode/workspace')
            covered.add(pair)
            log = payload(suite['log'])
            fields(log, 'platform git_commit_sha binary_sha256 status mode workspace runner_id steps')
            require(all(log[k] == v for k, v in basis.items()) and (log['mode'], log['workspace']) == pair
                    and identity(log['runner_id']) and log['steps'] == [{'step': s, 'status': 'PASS'} for s in steps_for(pair[1])], 'invalid GUI suite evidence')
            count += 1
    check_head(repo, head)
    check_matrix(repo, head)
    return {'status': 'PLATFORM_EVIDENCE_INTEGRITY_VERIFIED', 'git_commit_sha': head,
            'platforms_verified': len(seen), 'suites_verified': count, 'production_approved': False,
            'native_builds_executed_by_verifier': False, 'gui_e2e_executed_by_verifier': False,
            'physical_platform_provenance_independently_confirmed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path('.'))
    parser.add_argument('--release-dir', type=Path, required=True)
    parser.add_argument('--head-sha', required=True)
    parser.add_argument('--trusted-public-key-hex', required=True)
    args = parser.parse_args()
    try:
        report = verify(args.repo, args.release_dir, args.head_sha, args.trusted_public_key_hex)
    except (GateError, ImportError) as exc:
        print(json.dumps({'status': 'PLATFORM_GATE_NO_GO', 'reason': str(exc), 'production_approved': False}))
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
