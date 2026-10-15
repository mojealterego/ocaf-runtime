"""Stage 0.39 signed release dossier cannot self-approve."""
import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_signed_acceptance as sig


class SignedAcceptance(unittest.TestCase):
    def setUp(self):
        if subprocess.run(["which","openssl"],capture_output=True).returncode:
            self.skipTest("openssl missing")
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        self.keys={}
        for name in ("auditor","operator"):
            private=self.root/(name+".key")
            public=self.root/(name+".pub")
            subprocess.run(["openssl","genpkey","-algorithm","Ed25519",
                 "-out",str(private)],check=True,capture_output=True)
            subprocess.run(["openssl","pkey","-in",str(private),
                "-pubout","-out",str(public)],check=True,capture_output=True)
            self.keys[name]=(private,public,hashlib.sha256(public.read_bytes()).hexdigest())
        self.manifest={
            "schema_version":1,"stage":"0.39","branch":"main",
            "git_commit":"a"*40,"audit_result":"PASS",
            "operator_decision":"ACCEPT",
            "evidence":{}
        }
        for name in sig.REQUIRED:
            file=self.root/(name+".json")
            file.write_text('{"evidence":"synthetic fixture, no real runtime authority"}')
            self.manifest["evidence"][name]={
                "artifact_file":file.name,
                "sha256":hashlib.sha256(file.read_bytes()).hexdigest(),
            }
        self.payload=sig.canonical_manifest(self.manifest)
        self.signatures={}
        (self.root/"payload.bin").write_bytes(self.payload)
        for name,(private,public,pin) in self.keys.items():
            dest=self.root/(name+".sig")
            subprocess.run(["openssl","pkeyutl","-sign","-inkey",str(private),
                "-rawin","-in",str(self.root/"payload.bin"),
                "-out",str(dest)],check=True,capture_output=True)
            self.signatures[name]=dest

    def verify(self,**options):
        return sig.verify_signed_dossier(
            self.manifest,self.root,
            auditor_pub=self.keys["auditor"][1],operator_pub=self.keys["operator"][1],
            auditor_pin=self.keys["auditor"][2],operator_pin=self.keys["operator"][2],
            auditor_signature=self.signatures["auditor"],
            operator_signature=self.signatures["operator"],**options)

    def test_two_distinct_valid_external_test_signatures_are_integrity_only(self):
        r=self.verify()
        self.assertEqual(r["status"],"SIGNED_DOSSIER_INTEGRITY_VERIFIED_NOT_DEPLOYED")
        self.assertFalse(r["production_approved"])
        self.assertFalse(r["stage_039_complete"])
        self.assertFalse(r["auditor_independence_established"])

    def test_unsigned_tampered_or_same_signer_is_rejected(self):
        self.assertTrue(self.verify()["both_signatures_verified"])
        old=self.manifest["git_commit"]
        self.manifest["git_commit"]="b"*40
        with self.assertRaises(sig.SignedAcceptanceError):
            self.verify()
        self.manifest["git_commit"]=old
        p=self.keys["auditor"][1];pin=self.keys["auditor"][2]
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.verify_signed_dossier(self.manifest,self.root,
                auditor_pub=p,operator_pub=p,auditor_pin=pin,operator_pin=pin,
                auditor_signature=self.signatures["auditor"],
                operator_signature=self.signatures["operator"])
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.verify_signed_dossier(self.manifest,self.root,
                auditor_pub=p,operator_pub=self.keys["operator"][1],
                auditor_pin="0"*64,operator_pin=self.keys["operator"][2],
                auditor_signature=self.signatures["auditor"],
                operator_signature=self.signatures["operator"])

    def test_missing_artifact_or_self_attested_verdict_denied(self):
        to_remove=self.root/(sig.REQUIRED[0]+".json")
        to_remove.unlink()
        with self.assertRaises(sig.SignedAcceptanceError):
            self.verify()
        self.assertEqual(set(self.manifest["evidence"]),set(sig.REQUIRED))

    def test_duplicate_json_and_unknown_fields_denied(self):
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.parse_manifest(b'{"schema_version":1,"schema_version":1}')
        with self.assertRaises(sig.SignedAcceptanceError):
            sig.canonical_manifest({**self.manifest,"production_approved":True})
