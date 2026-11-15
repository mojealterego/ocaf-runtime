#!/usr/bin/env python3
"""Independent, fail-closed GATE-10 *evidence-integrity* verifier.

Validates ten signed product evidence bundles against a pinned main commit.
It NEVER builds an app, installs on a device, executes an agent, confirms a
physical runner, or authorizes production. Signed assertions are only as
credible as the custodian of the trusted verifier key.
"""
from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
from typing import Any

try:
    from cryptography.exceptions import InvalidSignature
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
except ImportError:  # optional for ordinary OCAF checks; mandatory to accept evidence
    InvalidSignature = None
    Ed25519PublicKey = None


EXPECTED_PRODUCTS = {
    'cloud-code': ('Cloud', 'Code'),
    'cloud-nocode': ('Cloud', 'NoCode'),
    'windows-code': ('Windows', 'Code'),
    'windows-nocode': ('Windows', 'NoCode'),
    'macos-code': ('macOS', 'Code'),
    'macos-nocode': ('macOS', 'NoCode'),
    'android-code': ('Android', 'Code'),
    'android-nocode': ('Android', 'NoCode'),
    'ios-code': ('iOS', 'Code'),
    'ios-nocode': ('iOS', 'NoCode'),
}

TARGET_TRIPLES = {
    'Cloud': frozenset({'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'}),
    'Windows': frozenset({'x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc'}),
    'macOS': frozenset({'aarch64-apple-darwin', 'x86_64-apple-darwin'}),
    'Android': frozenset({'aarch64-linux-android'}),
    'iOS': frozenset({'aarch64-apple-ios'}),
}

REQUIRED_STEPS = ('create', 'edit', 'validate', 'compile', 'export', 'import')
MAX_BYTES = {'binary': 512 * 1024 * 1024, 'gui_e2e': 8 * 1024 * 1024,
             'sbom': 8 * 1024 * 1024, 'install': 8 * 1024 * 1024}
SHA256_RE = re.compile(r'^[0-9a-f]{64}$')
GIT_SHA_RE = re.compile(r'^[0-9a-f]{40}$')


class VerificationError(ValueError):
    """Reject evidence rather than upgrading any milestone status."""


def _required(condition: bool, message: str) -> None:
    if not condition:
        raise VerificationError(message)


def _canonical(value: dict) -> bytes:
    return json.dumps(value, sort_keys=True, ensure_ascii=False,
                      separators=(',', ':'), allow_nan=False).encode('utf-8')


def _unique_pairs(pairs: list[tuple[str, Any]]) -> dict:
    result: dict[str, Any] = {}
    for name, value in pairs:
        _required(name not in result, 'duplicate JSON field')
        result[name] = value
    return result


def _strict_json(raw: bytes, limit: int) -> dict:
    _required(0 < len(raw) <= limit, 'missing, empty or oversized JSON evidence')
    try:
        obj = json.loads(raw.decode('utf-8'), object_pairs_hook=_unique_pairs,
                         parse_constant=lambda s: (_ for _ in ()).throw(VerificationError('nonfinite JSON constant')))
    except (UnicodeError, ValueError, TypeError) as exc:
        raise VerificationError('malformed evidence JSON') from exc
    _required(type(obj) is dict, 'expected JSON object')
    return obj


def _exact_fields(obj: dict, fields: set[str]) -> None:
    _required(set(obj) == fields, 'missing or unknown critical fields')


def _git(repo: Path, *args: str, input_bytes: bytes | None = None) -> str:
    try:
        p = subprocess.run(['git', '-C', str(repo), *args], input=input_bytes,
                           capture_output=True, timeout=5, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise VerificationError('git unavailable') from exc
    _required(p.returncode == 0, 'git provenance command failed')
    try:
        return p.stdout.decode('ascii').strip()
    except UnicodeError as exc:
        raise VerificationError('invalid Git output') from exc


def _file_bytes(root: Path, relative: str, limit: int) -> bytes:
    _required(isinstance(relative, str) and relative and len(relative) <= 200,
              'invalid file path')
    _required('\\' not in relative and ':' not in relative and '\x00' not in relative,
              'non-POSIX or unsafe evidence path')
    parts = PurePosixPath(relative).parts
    _required(parts and all(p not in ('', '.', '..') for p in parts)
              and not PurePosixPath(relative).is_absolute(), 'unsafe evidence path')
    path = root
    for i, part in enumerate(parts):
        path = path / part
        try:
            meta = path.lstat()
        except OSError as exc:
            raise VerificationError('evidence path missing') from exc
        _required(not stat.S_ISLNK(meta.st_mode), 'symlinked evidence path')
        if i < len(parts)-1:
            _required(stat.S_ISDIR(meta.st_mode), 'unsafe evidence directory')
        else:
            _required(stat.S_ISREG(meta.st_mode) and 0 < meta.st_size <= limit,
                      'missing/empty/nonregular/oversized evidence')
    try:
        # O_NOFOLLOW closes the final-component symlink race. Parent components
        # also require a trusted, non-writable release staging directory.
        flags = os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0)
        fd = os.open(path, flags)
        with os.fdopen(fd, 'rb') as f:
            metadata = os.fstat(f.fileno())
            _required(stat.S_ISREG(metadata.st_mode) and 0 < metadata.st_size <= limit,
                      'unsafe evidence handle')
            raw = f.read(limit + 1)
    except OSError as exc:
        raise VerificationError('cannot read bounded evidence') from exc
    _required(0 < len(raw) <= limit, 'evidence size limit')
    return raw


def _file_digest(root: Path, relative: str, limit: int) -> str:
    """Stream large binary evidence; never allocate the declared 512 MiB cap."""
    _required(isinstance(relative, str) and relative and len(relative) <= 200,
              'invalid file path')
    _required('\\' not in relative and ':' not in relative and '\x00' not in relative,
              'non-POSIX or unsafe evidence path')
    parts = PurePosixPath(relative).parts
    _required(parts and all(p not in ('', '.', '..') for p in parts)
              and not PurePosixPath(relative).is_absolute(), 'unsafe evidence path')
    path = root
    for i, part in enumerate(parts):
        path = path / part
        try:
            meta = path.lstat()
        except OSError as exc:
            raise VerificationError('evidence path missing') from exc
        _required(not stat.S_ISLNK(meta.st_mode), 'symlinked evidence path')
        if i < len(parts)-1:
            _required(stat.S_ISDIR(meta.st_mode), 'unsafe evidence directory')
        else:
            _required(stat.S_ISREG(meta.st_mode) and 0 < meta.st_size <= limit,
                      'missing/empty/nonregular/oversized evidence')
    try:
        fd = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0))
        with os.fdopen(fd, 'rb') as f:
            st = os.fstat(f.fileno())
            _required(stat.S_ISREG(st.st_mode) and 0 < st.st_size <= limit,
                      'unsafe evidence handle')
            h = hashlib.sha256()
            total = 0
            while True:
                chunk = f.read(min(1 << 20, limit + 1 - total))
                if not chunk:
                    break
                total += len(chunk)
                _required(total <= limit, 'evidence size limit')
                h.update(chunk)
            _required(total > 0, 'empty evidence')
            return h.hexdigest()
    except OSError as exc:
        raise VerificationError('cannot stream evidence digest') from exc


def _sha(value: str) -> bool:
    return type(value) is str and SHA256_RE.fullmatch(value) is not None


def _verify_manifest(repo: Path) -> None:
    path='docs/roadmap/agents_builder_products_v11.json'
    raw=_file_bytes(repo,path,2 * 1024 * 1024)
    tree_sha = _git(repo, 'rev-parse', '--verify', f'HEAD:{path}')
    observed = _git(repo, 'hash-object', '--stdin', '--no-filters', input_bytes=raw)
    _required(re.fullmatch(r'[0-9a-f]{40}', tree_sha) is not None
              and hmac.compare_digest(tree_sha, observed),
              'canonical matrix differs from pinned Git HEAD')
    obj=_strict_json(raw,2 * 1024 * 1024)
    _required(type(obj.get('schema_version')) is int and obj['schema_version'] == 1,
              'invalid product matrix schema')
    _required(obj.get('repository') == 'mojealterego/ocaf-runtime' and
              obj.get('branch') == 'main', 'wrong canonical matrix repository')
    products=obj.get('products')
    _required(type(products) is list and len(products) == 10, 'not ten canonical products')
    found=set()
    for item in products:
        _required(type(item) is dict, 'invalid product descriptor')
        id_=item.get('id')
        _required(type(id_) is str and id_.startswith('AB-'), 'invalid product ID')
        name=id_[3:].lower()
        _required(name in EXPECTED_PRODUCTS and name not in found,
                  'unknown or duplicated product ID')
        _required((item.get('platform'), item.get('mode')) == EXPECTED_PRODUCTS[name],
                  'wrong platform/mode mapping')
        found.add(name)
    _required(found == set(EXPECTED_PRODUCTS), 'incomplete canonical matrix')


def _verify_payload(ref: dict, root: Path, kind: str) -> tuple[bytes, str]:
    _required(type(ref) is dict, 'invalid evidence descriptor')
    _exact_fields(ref, {'path','sha256'})
    _required(_sha(ref['sha256']), 'malformed SHA-256')
    if kind == 'binary':
        observed = _file_digest(root, ref['path'], MAX_BYTES[kind])
        _required(hmac.compare_digest(ref['sha256'], observed), 'evidence SHA-256 mismatch')
        return b'', observed
    raw = _file_bytes(root, ref['path'], MAX_BYTES[kind])
    observed=hashlib.sha256(raw).hexdigest()
    _required(hmac.compare_digest(ref['sha256'],observed), 'evidence SHA-256 mismatch')
    return raw, observed


def _verify_log(raw: bytes, name: str, platform: str, mode: str, head: str) -> None:
    data=_strict_json(raw,MAX_BYTES['gui_e2e'])
    _exact_fields(data, {'schema_version','product_id','git_commit_sha','status',
                         'platform','mode','runner_id','steps'})
    _required(type(data['schema_version']) is int and data['schema_version']==1,
              'invalid GUI E2E schema')
    _required((data['product_id'],data['git_commit_sha'],data['status'],
               data['platform'],data['mode']) == (name,head,'PASS',platform,mode),
              'GUI E2E proof does not match product/commit')
    _required(isinstance(data['runner_id'],str) and 3 <= len(data['runner_id']) <= 256,
              'missing runner provenance')
    steps=data['steps']
    _required(type(steps) is list and len(steps)==len(REQUIRED_STEPS),
              'missing GUI E2E workflow steps')
    for step,expected in zip(steps,REQUIRED_STEPS):
        _required(type(step) is dict and set(step)=={'step','status'}
                  and step.get('step')==expected and step.get('status')=='PASS',
                  'failed or missing GUI E2E step')


def _verify_sbom(raw: bytes) -> None:
    obj=_strict_json(raw,MAX_BYTES['sbom'])
    _required(obj.get('bomFormat')=='CycloneDX' and obj.get('specVersion') in ('1.5','1.6')
              and type(obj.get('version')) is int and obj['version'] >= 1,
              'invalid CycloneDX SBOM')
    parts=obj.get('components')
    _required(type(parts) is list and len(parts)>0, 'empty SBOM components')
    for c in parts:
        _required(type(c) is dict and type(c.get('name')) is str and c['name'].strip()
                  and type(c.get('version')) is str and c['version'].strip(),
                  'malformed SBOM component')


def _verify_install(raw:bytes, name:str, triple:str,head:str)->None:
    obj=_strict_json(raw,MAX_BYTES['install'])
    _exact_fields(obj, {'schema_version','product_id','git_commit_sha','target_triple',
                        'status','runner_id','device_id'})
    _required(type(obj['schema_version']) is int and obj['schema_version']==1
              and (obj['product_id'],obj['git_commit_sha'],obj['target_triple'],obj['status'])
              == (name,head,triple,'PASS'), 'wrong or failed install proof')
    _required(type(obj['runner_id']) is str and len(obj['runner_id']) >= 3
              and type(obj['device_id']) is str and len(obj['device_id']) >= 3,
              'missing native runner/device identifier')


def _verify_cert(root:Path,name:str,platform:str,mode:str,head:str,pub:Ed25519PublicKey)->dict:
    raw=_file_bytes(root,'CERTIFICATE_RECORD.json',256 * 1024)
    cert=_strict_json(raw,256 * 1024)
    _exact_fields(cert,{'schema_version','product_id','platform','mode','pinned_git_commit_sha',
                        'target_triple','toolchain','status','files','verifier_id','attestation'})
    _required(type(cert['schema_version']) is int and cert['schema_version']==1,
              'invalid cert version')
    _required((cert['product_id'],cert['platform'],cert['mode'],
               cert['pinned_git_commit_sha'],cert['status'])
              ==(name,platform,mode,head,'TESTED_INTEGRATION'),
              'certificate identity/status mismatch')
    _required(type(cert['target_triple']) is str and
              cert['target_triple'] in TARGET_TRIPLES[platform],
              'wrong target triple')
    _required(type(cert['toolchain']) is str and 3<=len(cert['toolchain'])<=256
              and type(cert['verifier_id']) is str and 3<=len(cert['verifier_id'])<=128,
              'missing toolchain or verifier identity')
    att=cert['attestation'];_required(type(att) is dict,'missing Ed25519 attestation')
    _exact_fields(att,{'algorithm','signature_hex'})
    _required(att['algorithm']=='Ed25519'
              and type(att['signature_hex']) is str
              and re.fullmatch(r'[0-9a-f]{128}',att['signature_hex']) is not None,
              'invalid attestation syntax')
    unsigned={k:v for k,v in cert.items() if k!='attestation'}
    try:
        pub.verify(bytes.fromhex(att['signature_hex']),_canonical(unsigned))
    except (InvalidSignature,ValueError,TypeError) as exc:
        raise VerificationError('untrusted or tampered signed product certificate') from exc
    refs=cert['files']
    _required(type(refs) is dict,'invalid file references')
    required={'binary','gui_e2e','sbom'} | ({'install'} if platform!='Cloud' else set())
    _exact_fields(refs,required)
    _required(len({v.get('path') for v in refs.values() if isinstance(v, dict)}) == len(required),
              'evidence paths must be distinct')
    evidence={}
    for kind in sorted(required):
        payload,hash_= _verify_payload(refs[kind],root,kind)
        if kind=='gui_e2e':_verify_log(payload,name,platform,mode,head)
        if kind=='sbom':_verify_sbom(payload)
        if kind=='install':_verify_install(payload,name,cert['target_triple'],head)
        evidence[kind]=hash_
    return {'product_id':name,'platform':platform,'mode':mode,'verified_sha256':evidence}


def verify_release(repo:Path,release_dir:Path,head_sha:str,
                   trusted_verifier_public_key_hex:str)->dict:
    """Verify signed evidence. Do not infer actual native build/hardware use."""
    repo=Path(repo);release_dir=Path(release_dir)
    _required(type(head_sha) is str and GIT_SHA_RE.fullmatch(head_sha) is not None,
              'invalid pinned full Git SHA')
    _required(_git(repo,'rev-parse','--verify','HEAD^{commit}')==head_sha,
              'HEAD does not match pinned SHA')
    _required(_git(repo,'symbolic-ref','--short','HEAD')=='main',
              'release must originate from main')
    _verify_manifest(repo)
    _required(Ed25519PublicKey is not None, 'cryptography dependency unavailable: GATE10_NO_GO')
    _required(type(trusted_verifier_public_key_hex) is str
              and re.fullmatch(r'[0-9a-f]{64}',trusted_verifier_public_key_hex) is not None,
              'trusted verification public key must be 32-byte lowercase hex')
    try:
        pub=Ed25519PublicKey.from_public_bytes(bytes.fromhex(trusted_verifier_public_key_hex))
    except ValueError as exc:
        raise VerificationError('invalid Ed25519 trust root') from exc
    _required(release_dir.is_dir() and not release_dir.is_symlink(),
              'release directory not an ordinary directory')
    products=[]
    for name,(platform,mode) in EXPECTED_PRODUCTS.items():
        directory=release_dir/name
        _required(directory.is_dir() and not directory.is_symlink(),
                  f'missing product directory {name}')
        products.append(_verify_cert(directory,name,platform,mode,head_sha,pub))
    return {
        'schema_version':1,'status':'GATE10_EVIDENCE_INTEGRITY_VERIFIED',
        'pinned_git_commit_sha':head_sha,
        'trusted_verifier_public_key_sha256':hashlib.sha256(bytes.fromhex(trusted_verifier_public_key_hex)).hexdigest(),
        'evidence_bundles_verified':len(products), 'products':products,
        'native_builds_executed_by_verifier':False,'gui_e2e_executed_by_verifier':False,
        'physical_platform_provenance_independently_confirmed':False,
        'production_approved':False,
        'limitations':'signed_evidence_integrity_only;not_hardware_attestation;not_external_audit;not_production_go',
    }


def main(argv:list[str]|None=None)->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo',type=Path,default=Path('.'))
    parser.add_argument('--release-dir',type=Path,default=Path('release'))
    parser.add_argument('--head-sha',required=True)
    parser.add_argument('--trusted-public-key-hex',required=True,
                        help='32-byte pinned Ed25519 trust root from operator, not from release bundle')
    args=parser.parse_args(argv)
    try:
        report=verify_release(args.repo,args.release_dir,args.head_sha,args.trusted_public_key_hex)
    except VerificationError as exc:
        print(json.dumps({'status':'GATE10_NO_GO','reason':str(exc),
                          'production_approved':False},sort_keys=True))
        return 1
    print(json.dumps(report,sort_keys=True))
    return 0


if __name__=='__main__':
    sys.exit(main())
