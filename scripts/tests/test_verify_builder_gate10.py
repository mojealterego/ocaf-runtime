"""Security regressions for independent OCAF Agents Builder GATE-10 evidence verification.

All artifacts in this suite are SYNTHETIC fixtures; no device or product is
certified by these tests.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from copy import deepcopy

try:
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
except ImportError:
    Ed25519PrivateKey = None

from scripts.verify_builder_gate10 import EXPECTED_PRODUCTS, VerificationError, verify_release


def canonical(data: dict) -> bytes:
    return json.dumps(data, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()


def write_json(path: Path, obj: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(obj) + b'\n')


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@unittest.skipUnless(Ed25519PrivateKey is not None, "cryptography==46.0.4 required for signed-fixture regressions")
class Gate10Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name) / 'source'
        self.repo.mkdir()
        self.rel = Path(self.tmp.name) / 'release'
        self.rel.mkdir()
        self.private = Ed25519PrivateKey.generate()
        from cryptography.hazmat.primitives import serialization
        self.pub = self.private.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        ).hex()
        subprocess.run(['git', 'init', '-q', '-b', 'main', str(self.repo)], check=True)
        subprocess.run(['git', '-C', str(self.repo), 'config', 'user.email', 'tester@example.invalid'], check=True)
        subprocess.run(['git', '-C', str(self.repo), 'config', 'user.name', 'Gate Tester'], check=True)
        manifest = {'schema_version': 1, 'repository':'mojealterego/ocaf-runtime',
                    'branch':'main', 'products':[
                        {'id':'AB-'+n.upper(), 'platform':v[0], 'mode':v[1],
                         'status':'NOT_IMPLEMENTED','production_approved':False}
                        for n,v in EXPECTED_PRODUCTS.items()]}
        write_json(self.repo / 'docs/roadmap/agents_builder_products_v11.json', manifest)
        subprocess.run(['git', '-C', str(self.repo), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.repo), 'commit', '-qm', 'test fixture'], check=True)
        self.sha = subprocess.check_output(['git', '-C', str(self.repo), 'rev-parse', 'HEAD'], text=True).strip()
        for name, (platform, mode) in EXPECTED_PRODUCTS.items():
            d = self.rel / name
            d.mkdir()
            # Fixtures are not deployable binaries and must never be represented as such.
            binary = d/'binary.bin'; binary.write_bytes(b'unit-test-artifact-'+name.encode())
            log = d/'gui_e2e.json'
            write_json(log, {'schema_version':1,'product_id':name,'git_commit_sha':self.sha,
                             'status':'PASS','platform':platform,'mode':mode,
                             'runner_id':'synthetic-unit-fixture',
                             'steps':[{'step':step,'status':'PASS'} for step in
                                      ('create','edit','validate','compile','export','import')]})
            sbom = d/'SBOM.cdx.json'
            write_json(sbom, {'bomFormat':'CycloneDX','specVersion':'1.6','version':1,
                              'components':[{'type':'application','name':name,'version':'1.0.0'}]})
            files = {'binary': {'path':'binary.bin','sha256':digest(binary)},
                     'gui_e2e': {'path':'gui_e2e.json','sha256':digest(log)},
                     'sbom': {'path':'SBOM.cdx.json','sha256':digest(sbom)}}
            if platform != 'Cloud':
                install = d/'install.json'
                write_json(install, {'schema_version':1,'product_id':name,'git_commit_sha':self.sha,
                                     'target_triple':next(iter(allowed_triples(platform))),
                                     'status':'PASS','runner_id':'synthetic-unit-fixture',
                                     'device_id':'unit-test-device'})
                files['install']={'path':'install.json','sha256':digest(install)}
            cert = {'schema_version':1,'product_id':name,'platform':platform,'mode':mode,
                    'pinned_git_commit_sha':self.sha,'target_triple':next(iter(allowed_triples(platform))),
                    'toolchain':'unit test only','status':'TESTED_INTEGRATION',
                    'files':files,'verifier_id':'unit-test-verifier'}
            self._write_signed_cert(name, cert)

    def _write_signed_cert(self, name, cert):
        content = {k:v for k,v in cert.items() if k!='attestation'}
        content['attestation'] = {'algorithm':'Ed25519',
                                  'signature_hex':self.private.sign(canonical(content)).hex()}
        write_json(self.rel/name/'CERTIFICATE_RECORD.json',content)

    def read_cert(self, name):
        return json.loads((self.rel/name/'CERTIFICATE_RECORD.json').read_text())

    def update_cert(self, name, function, resign=True):
        cert=self.read_cert(name);function(cert)
        if resign:self._write_signed_cert(name,cert)
        else:write_json(self.rel/name/'CERTIFICATE_RECORD.json',cert)

    def verify(self, **kw):
        return verify_release(self.repo,self.rel,self.sha,self.pub,**kw)

    def test_all_ten_synthetic_signed_evidence_receipts_have_only_scoped_result(self):
        result=self.verify()
        self.assertEqual(result['evidence_bundles_verified'],10)
        self.assertEqual(result['status'],'GATE10_EVIDENCE_INTEGRITY_VERIFIED')
        self.assertFalse(result['production_approved'])
        self.assertFalse(result['native_builds_executed_by_verifier'])

    def test_refuse_missing_product(self):
        (self.rel/'ios-nocode'/'CERTIFICATE_RECORD.json').unlink()
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_empty_binary(self):
        (self.rel/'cloud-code'/'binary.bin').write_bytes(b'')
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_modified_binary_even_when_cert_has_old_hash(self):
        (self.rel/'android-code'/'binary.bin').write_bytes(b'tampered')
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_fake_pass_log_without_steps(self):
        name='cloud-code';p=self.rel/name/'gui_e2e.json'
        log=json.loads(p.read_text());log['steps']=[];write_json(p,log)
        self.update_cert(name,lambda c:c['files']['gui_e2e'].update(sha256=digest(p)))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_failing_gui_step(self):
        name='android-nocode';p=self.rel/name/'gui_e2e.json'
        log=json.loads(p.read_text());log['steps'][2]['status']='FAIL';write_json(p,log)
        self.update_cert(name,lambda c:c['files']['gui_e2e'].update(sha256=digest(p)))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_git_sha_mismatch(self):
        with self.assertRaises(VerificationError):
            verify_release(self.repo,self.rel,'0'*40,self.pub)

    def test_refuse_untrusted_signer(self):
        other=Ed25519PrivateKey.generate()
        from cryptography.hazmat.primitives import serialization
        pub=other.public_key().public_bytes(encoding=serialization.Encoding.Raw,format=serialization.PublicFormat.Raw).hex()
        with self.assertRaises(VerificationError):
            verify_release(self.repo,self.rel,self.sha,pub)

    def test_refuse_unsigned_or_tampered_certificate(self):
        self.update_cert('cloud-code',lambda c:c.update(toolchain='FAKE'),resign=False)
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_path_traversal_even_if_certificate_signed(self):
        self.update_cert('cloud-code',lambda c:c['files']['binary'].update(path='../another/binary.bin'))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_symlinked_binary(self):
        p=self.rel/'cloud-code'/'binary.bin'
        outside=Path(self.tmp.name)/'outside.bin';outside.write_bytes(p.read_bytes())
        p.unlink();p.symlink_to(outside)
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_unrecognized_product_in_manifest(self):
        p=self.repo/'docs/roadmap/agents_builder_products_v11.json'
        manifest=json.loads(p.read_text());manifest['products'][0]['id']='AB-UNKNOWN';write_json(p,manifest)
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_unsigned_status_promotion(self):
        self.update_cert('cloud-code',lambda c:c.update(status='PRODUCTION_APPROVED'))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_truncated_or_unsigned_evidence_bundle(self):
        cert_path=self.rel/'cloud-code'/'CERTIFICATE_RECORD.json'
        cert_path.write_text('{')
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_install_log_wrong_platform(self):
        name='ios-code';p=self.rel/name/'install.json'
        log=json.loads(p.read_text());log['target_triple']='x86_64-pc-windows-msvc';write_json(p,log)
        self.update_cert(name,lambda c:c['files']['install'].update(sha256=digest(p)))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_git_branch_other_than_main(self):
        subprocess.run(['git','-C',str(self.repo),'checkout','-qb','other'],check=True)
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_malformed_sha_in_cert_even_signed(self):
        self.update_cert('cloud-code',lambda c:c['files']['binary'].update(sha256='123'))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_schema_boolean_version(self):
        self.update_cert('cloud-code',lambda c:c.update(schema_version=True))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_extra_critical_fields(self):
        self.update_cert('cloud-code',lambda c:c.update(override_status='PASS'))
        with self.assertRaises(VerificationError):self.verify()

    def test_refuse_oversized_sparse_binary_without_loading_it(self):
        # Sparse allocation verifies the cap without allocating 512 MiB of RAM.
        with (self.rel/'cloud-code'/'binary.bin').open('wb') as f:
            f.truncate(512 * 1024 * 1024 + 1)
        with self.assertRaises(VerificationError):
            self.verify()

    def test_refuse_aliased_evidence_paths_even_with_valid_signatures(self):
        name='cloud-code';d=self.rel/name
        self.update_cert(name, lambda c:c['files']['sbom'].update(path='gui_e2e.json'))
        with self.assertRaises(VerificationError):
            self.verify()

    def test_manifest_changed_but_hidden_by_assume_unchanged_rejected(self):
        p=self.repo/'docs/roadmap/agents_builder_products_v11.json'
        subprocess.run(['git','-C',str(self.repo),'update-index','--assume-unchanged',
                        'docs/roadmap/agents_builder_products_v11.json'],check=True)
        data=json.loads(p.read_text());data['effective_date']='2030-01-01';write_json(p,data)
        with self.assertRaises(VerificationError):
            self.verify()

    def test_cli_denies_without_key(self):
        program=Path(__file__).resolve().parents[1]/'verify_builder_gate10.py'
        result=subprocess.run([sys.executable,str(program),'--repo',str(self.repo),'--release-dir',str(self.rel),'--head-sha',self.sha],capture_output=True,text=True,timeout=10)
        self.assertNotEqual(result.returncode,0)


def allowed_triples(platform):
    return {'Cloud':('x86_64-unknown-linux-gnu',),
            'Windows':('x86_64-pc-windows-msvc',),
            'macOS':('aarch64-apple-darwin',),
            'Android':('aarch64-linux-android',),
            'iOS':('aarch64-apple-ios',)}[platform]


if __name__ == '__main__':unittest.main()
