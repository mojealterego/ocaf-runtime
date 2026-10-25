#!/usr/bin/env python3
"""0.39.17: verify the owner's signed release dossier, not deployment.

The project owner waived the independent audit on 2026-10-25. Schema v2
requires that waiver in the signed payload and one out-of-band pinned Ed25519
operator key. Artifact bytes are rehashed from private local files. A valid
signature alone cannot prove host evidence or set a production release flag.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tempfile

REQUIRED=(
    "two_jailed_tenant_guest_kernel_ebpf",
    "independent_kernel_kvm_hosts",
    "two_kernel_cold_restart",
    "kernel_tls13_mtls_partition_fencing",
    "production_interhost_mtls_ha",
    "durable_host_partition_soak",
    "operator_formal_approval",
)
HEX64=re.compile(r"[a-f0-9]{64}\Z")
SHA40=re.compile(r"[a-f0-9]{40}\Z")
BASENAME=re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,120}\Z")


class SignedAcceptanceError(ValueError):
    pass


def _unique(pairs):
    result={}
    for k,v in pairs:
        if k in result:raise SignedAcceptanceError("duplicate signed JSON property")
        result[k]=v
    return result


def parse_manifest(raw:bytes)->dict:
    if not isinstance(raw,bytes) or not 1<=len(raw)<=32768:
        raise SignedAcceptanceError("invalid dossier JSON size")
    try:
        m=json.loads(raw,object_pairs_hook=_unique,
                     parse_constant=lambda _:(_ for _ in ()).throw(SignedAcceptanceError("JSON NaN")))
    except (ValueError,UnicodeError,TypeError) as exc:
        raise SignedAcceptanceError("malformed signed dossier JSON") from exc
    if (not isinstance(m,dict) or
            set(m)!={"schema_version","stage","branch","git_commit",
                    "audit_waiver","operator_decision","evidence"} or
            type(m["schema_version"]) is not int or m["schema_version"]!=2 or
            m["stage"]!="0.39" or m["branch"]!="main" or
            not isinstance(m["git_commit"],str) or
            SHA40.fullmatch(m["git_commit"]) is None or
            m["audit_waiver"]!="OWNER_WAIVES_INDEPENDENT_AUDIT" or
            m["operator_decision"]!="ACCEPT" or
            not isinstance(m["evidence"],dict) or
            set(m["evidence"])!=set(REQUIRED)):
        raise SignedAcceptanceError("incomplete or unauthorized dossier schema")
    for key,ref in m["evidence"].items():
        if (not isinstance(ref,dict) or set(ref)!={"artifact_file","sha256"} or
                not isinstance(ref["artifact_file"],str) or
                BASENAME.fullmatch(ref["artifact_file"]) is None or
                ref["artifact_file"] in (".","..") or
                not isinstance(ref["sha256"],str) or
                HEX64.fullmatch(ref["sha256"]) is None):
            raise SignedAcceptanceError("invalid signed artifact pointer: "+key)
    if len({m["evidence"][k]["artifact_file"] for k in REQUIRED})!=len(REQUIRED):
        raise SignedAcceptanceError("signed acceptance artifacts must be separate")
    return m


def canonical_manifest(data:dict)->bytes:
    try:
        raw=json.dumps(data,sort_keys=True,separators=(",",":"),
                       ensure_ascii=True,allow_nan=False).encode("ascii")
    except (ValueError,TypeError,UnicodeError) as exc:
        raise SignedAcceptanceError("unserializable dossier") from exc
    parse_manifest(raw)
    return raw


def _regular(path:Path,*,max_bytes:int)->bytes:
    if not isinstance(path,Path) or not path.is_absolute() or ".." in path.parts:
        raise SignedAcceptanceError("absolute nontraversing file required")
    if path.is_symlink():
        raise SignedAcceptanceError("signature or artifact symlinks forbidden")
    flags=os.O_RDONLY|getattr(os,"O_NOFOLLOW",0)|getattr(os,"O_CLOEXEC",0)
    try:
        fd=os.open(path,flags)
        try:
            st=os.fstat(fd)
            if (not stat.S_ISREG(st.st_mode) or st.st_nlink!=1 or
                    not 0<st.st_size<=max_bytes or st.st_mode & 0o022):
                raise SignedAcceptanceError("unsafe evidence file inode")
            payload=b""
            while len(payload)<=max_bytes:
                part=os.read(fd,min(131072,max_bytes+1-len(payload)))
                if not part:break
                payload+=part
            if len(payload)!=st.st_size:
                raise SignedAcceptanceError("evidence file changed during read")
            if (os.fstat(fd).st_ino!=st.st_ino or
                os.fstat(fd).st_size!=st.st_size):
                raise SignedAcceptanceError("evidence TOCTOU detected")
            return payload
        finally:
            os.close(fd)
    except OSError as exc:
        raise SignedAcceptanceError("unavailable signed evidence file") from exc


def verify_signed_dossier(manifest:dict,root:Path,*,operator_pub:Path,
                          operator_pin:str,operator_signature:Path)->dict:
    canonical=canonical_manifest(manifest)
    if (not isinstance(root,Path) or not root.is_absolute() or root.is_symlink()
            or not root.is_dir() or root.stat().st_mode & 0o077):
        raise SignedAcceptanceError("private evidence directory required")
    for name in REQUIRED:
        entry=manifest["evidence"][name]
        file=root/entry["artifact_file"]
        if hashlib.sha256(_regular(file,max_bytes=10*1024*1024)).hexdigest()!=entry["sha256"]:
            raise SignedAcceptanceError("signed artifact content does not match pin: "+name)
    if (not isinstance(operator_pin,str) or HEX64.fullmatch(operator_pin) is None or
            hashlib.sha256(_regular(operator_pub,max_bytes=4096)).hexdigest()!=operator_pin):
        raise SignedAcceptanceError("operator key differs from out-of-band pin")
    if not shutil_which_openssl():
        raise SignedAcceptanceError("OpenSSL Ed25519 signature verifier unavailable")
    # Ed25519 pkeyutl requires a *regular* input, not a pipe/stdin.
    tmp_path=None
    try:
        with tempfile.NamedTemporaryFile(dir=root,prefix=".verify039-",delete=False) as fp:
            tmp_path=Path(fp.name)
            fp.write(canonical)
        os.chmod(tmp_path,0o600)
        _regular(operator_signature,max_bytes=1024)
        try:
            proc=subprocess.run([
                "openssl","pkeyutl","-verify","-pubin",
                "-inkey",str(operator_pub),"-sigfile",str(operator_signature),
                "-rawin","-in",str(tmp_path)],
                stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                timeout=5,check=False)
        except (OSError,subprocess.TimeoutExpired) as exc:
            raise SignedAcceptanceError("cryptographic verification unavailable") from exc
        if proc.returncode!=0 or b"Signature Verified Successfully" not in proc.stdout:
            raise SignedAcceptanceError("operator Ed25519 signature invalid")
    finally:
        if tmp_path is not None:tmp_path.unlink(missing_ok=True)
    return {
        "schema_version":2,
        "status":"SIGNED_DOSSIER_INTEGRITY_VERIFIED_NOT_DEPLOYED",
        "stage":"0.39",
        "git_commit":manifest["git_commit"],
        "operator_signature_verified":True,
        "independent_audit_waived_by_owner":True,
        "signed_manifest_sha256":hashlib.sha256(canonical).hexdigest(),
        "artifacts_matched_signed_hashes":True,
        "evidence_authenticity_independently_verified":False,
        "durable_host_production_deployed":False,
        "stage_039_complete":False,
        "production_approved":False,
    }


def shutil_which_openssl()->bool:
    import shutil
    return shutil.which("openssl") is not None


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    for key in ("manifest","evidence-root","operator-pub",
                "operator-signature","report"):
        ap.add_argument("--"+key,type=Path,required=True)
    for key in ("operator-pin",):
        ap.add_argument("--"+key,required=True)
    x=ap.parse_args()
    try:
        raw=_regular(x.manifest,max_bytes=32768)
        m=parse_manifest(raw)
        r=verify_signed_dossier(
            m,x.evidence_root,operator_pub=x.operator_pub,
            operator_pin=x.operator_pin,
            operator_signature=x.operator_signature)
        if x.report.exists():raise SignedAcceptanceError("report exists")
        with x.report.open("x") as file:
            json.dump(r,file,sort_keys=True,indent=2);file.write("\n")
    except (OSError,ValueError) as exc:
        ap.error(str(exc))
    print(json.dumps({"status":r["status"],"stage_039_complete":False,
                      "production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
