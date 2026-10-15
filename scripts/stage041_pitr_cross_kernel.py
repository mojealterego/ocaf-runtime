#!/usr/bin/env python3
"""0.41.4 — REAL PostgreSQL physical PITR on a second independent Linux kernel.

GitHub artifact carries actual pg_basebackup and archived WAL from source
postmaster after its IMMEDIATE crash. On another ephemeral CI runner:
verify two SHA256-pinned tar archives, source LSN + 35-row checksum, and
two different /proc/sys/kernel/random/boot_id values; reject symlink/tar
path traversal/duplicate PAX contents; start a *new* PostgreSQL process
from physical backup, replay archived WAL to the original LSN, verify
35 restored rows with none of the 5 post-target transactions.

NOT a physical power loss, immutable offsite WAL, trusted signed backup,
or production-level continuous RPO/HA/soak proof.
"""
from __future__ import annotations

import argparse
from datetime import datetime,timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import time

import stage041_pitr as pitr

class CrossPITRError(ValueError):
    pass

UUID=re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z")
HASH=re.compile(r"[a-f0-9]{64}\Z")
NONCE=re.compile(r"[a-f0-9]{16}\Z")
SAFE_PART=re.compile(r"[A-Za-z0-9_.-]{1,120}\Z")
FIELDS={"schema_version","source_boot_id","source_report_sha256",
        "basebackup_sha256","wal_archive_sha256","target_lsn",
        "target_checksum","expected_target_rows","later_committed_rows_excluded",
        "run_nonce","production_approved"}
MAX_ARCHIVE_BYTES=256*1024*1024
MAX_UNCOMPRESSED=512*1024*1024
MAX_MEMBERS=12000

def _unique(pairs):
    record={}
    for k,v in pairs:
        if k in record:raise CrossPITRError("duplicate JSON evidence property")
        record[k]=v
    return record

def _load_json(path:Path,*,limit:int=131072)->tuple[dict,bytes]:
    if path.is_symlink() or not path.is_file() or not 0<path.stat().st_size<=limit:
        raise CrossPITRError("invalid or oversized JSON input")
    fd=os.open(path,os.O_RDONLY|getattr(os,"O_NOFOLLOW",0))
    try:
        before=os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink!=1:
            raise CrossPITRError("unsafe JSON evidence inode")
        content=os.read(fd,limit+1)
        after=os.fstat(fd)
        if (len(content)!=before.st_size or before.st_size!=after.st_size
            or before.st_ino!=after.st_ino or before.st_mtime_ns!=after.st_mtime_ns):
            raise CrossPITRError("JSON evidence mutated during read")
    finally:os.close(fd)
    try:
        result=json.loads(content,object_pairs_hook=_unique,
                          parse_constant=lambda _:(_ for _ in ()).throw(CrossPITRError("nonfinite JSON")))
    except (ValueError,UnicodeError,TypeError) as exc:
        raise CrossPITRError("invalid JSON evidence") from exc
    if not isinstance(result,dict):
        raise CrossPITRError("JSON evidence must be object")
    return result,content

def _hash_archive(path:Path)->str:
    if path.is_symlink() or not path.is_file() or not 0<path.stat().st_size<=MAX_ARCHIVE_BYTES:
        raise CrossPITRError("bounded regular archive required")
    fd=os.open(path,os.O_RDONLY|getattr(os,"O_NOFOLLOW",0))
    try:
        before=os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink!=1:
            raise CrossPITRError("hardlink/device archive denied")
        digest=hashlib.sha256()
        count=0
        while part:=os.read(fd,1024*1024):
            count+=len(part)
            if count>MAX_ARCHIVE_BYTES:raise CrossPITRError("archive byte limit")
            digest.update(part)
        after=os.fstat(fd)
        if (count!=before.st_size or count!=after.st_size or
                before.st_mtime_ns!=after.st_mtime_ns or before.st_ino!=after.st_ino):
            raise CrossPITRError("archive modified during SHA256 hashing")
        return digest.hexdigest()
    finally:os.close(fd)

def _tar_members(path:Path,label:str)->list[tarfile.TarInfo]:
    count=0
    total=0
    seen=set()
    names=[]
    try:
        with tarfile.open(path,"r:gz") as t:
            for item in t:
                count+=1
                if count>MAX_MEMBERS:raise CrossPITRError("too many physical PG archive members")
                name=item.name.rstrip("/")
                parts=name.split("/")
                if (not name or parts[0]!=label or
                    any(p in (".","..","") or SAFE_PART.fullmatch(p) is None for p in parts)
                    or not (item.isfile() or item.isdir())
                    or item.size<0 or item.size>MAX_UNCOMPRESSED
                    or name in seen):
                    raise CrossPITRError("nonregular or unsafe PostgreSQL tar member")
                seen.add(name)
                total+=item.size
                if total>MAX_UNCOMPRESSED:
                    raise CrossPITRError("too much unpacked PostgreSQL data")
                names.append(item)
    except (OSError,tarfile.TarError,EOFError) as exc:
        raise CrossPITRError("unreadable, damaged or hostile PG tar") from exc
    if not names or not any(item.isfile() for item in names):
        raise CrossPITRError("empty physical PostgreSQL backup")
    return names

def inspect_bundle(bundle_dir:Path,destination_boot_id:str,*,
                   validate_tar:bool=True)->dict:
    if (not isinstance(bundle_dir,Path) or not bundle_dir.is_absolute()
        or bundle_dir.is_symlink() or not bundle_dir.is_dir()
        or not isinstance(destination_boot_id,str)
        or UUID.fullmatch(destination_boot_id) is None):
        raise CrossPITRError("new Linux kernel and absolute bundle path required")
    manifest,_=_load_json(bundle_dir/"manifest.json",limit=4096)
    source,source_bytes=_load_json(bundle_dir/"source.json")
    if (set(manifest)!=FIELDS or
        type(manifest["schema_version"]) is not int or manifest["schema_version"]!=1 or
        manifest["production_approved"] is not False or
        not isinstance(manifest["source_boot_id"],str) or
        UUID.fullmatch(manifest["source_boot_id"]) is None or
        manifest["source_boot_id"]==destination_boot_id or
        not isinstance(manifest["run_nonce"],str) or NONCE.fullmatch(manifest["run_nonce"]) is None or
        type(manifest["expected_target_rows"]) is not int or manifest["expected_target_rows"]!=35 or
        type(manifest["later_committed_rows_excluded"]) is not int or
        manifest["later_committed_rows_excluded"]!=5 or
        not isinstance(manifest["target_lsn"],str) or pitr.LSN.fullmatch(manifest["target_lsn"]) is None or
        not isinstance(manifest["target_checksum"],str) or HASH.fullmatch(manifest["target_checksum"]) is None):
        raise CrossPITRError("incomplete or forged PITR transfer identity and target")
    for key in ("source_report_sha256","basebackup_sha256","wal_archive_sha256"):
        if not isinstance(manifest[key],str) or HASH.fullmatch(manifest[key]) is None:
            raise CrossPITRError("noncanonical PITR manifest SHA256")
    if hashlib.sha256(source_bytes).hexdigest()!=manifest["source_report_sha256"]:
        raise CrossPITRError("PITR source report mutated in transit")
    try:
        pitr.validate_pitr(source)
    except (ValueError,TypeError) as exc:
        raise CrossPITRError("source host did not prove real WAL PITR") from exc
    if (source.get("host_boot_id")!=manifest["source_boot_id"]
        or source.get("recovery_target_lsn")!=manifest["target_lsn"]
        or source.get("original_target_checksum")!=manifest["target_checksum"]
        or source.get("expected_rows")!=manifest["expected_target_rows"]
        or source.get("post_target_rows")!=manifest["later_committed_rows_excluded"]
        or not isinstance(source.get("target_wal_archived"),str)
        or pitr.WALNAME.fullmatch(source["target_wal_archived"]) is None
        or source.get("primary_immediate_crash_executed") is not True):
        raise CrossPITRError("PITR source record does not match transfer manifest")
    for file,pin,label in (
       ("basebackup.tar.gz",manifest["basebackup_sha256"],"base"),
       ("archived-wal.tar.gz",manifest["wal_archive_sha256"],"wal"),
    ):
        path=bundle_dir/file
        if _hash_archive(path)!=pin:
            raise CrossPITRError("modified physical PG archive "+file)
        if validate_tar:_tar_members(path,label)
    return {
        "target_lsn":manifest["target_lsn"],
        "target_checksum":manifest["target_checksum"],
        "target_wal_archived":source["target_wal_archived"],
        "expected_target_rows":35,
        "later_committed_rows_excluded":5,
        "source_boot_id":manifest["source_boot_id"],
        "recovery_boot_id":destination_boot_id,
        "distinct_linux_kernels":True,
        "run_nonce":manifest["run_nonce"],
        "source_report_sha256":manifest["source_report_sha256"],
        "basebackup_sha256":manifest["basebackup_sha256"],
        "wal_archive_sha256":manifest["wal_archive_sha256"],
        "source_primary_crash_and_local_pitr_proved":True,
        "production_approved":False,
    }

def validate_restored(evidence:dict,rows:int,sha256:str,later:int,elapsed_ms:int)->dict:
    if (not isinstance(evidence,dict)
        or evidence.get("distinct_linux_kernels") is not True
        or evidence.get("source_primary_crash_and_local_pitr_proved") is not True
        or evidence.get("production_approved") is not False
        or type(rows) is not int or rows!=35
        or type(later) is not int or later!=0
        or type(elapsed_ms) is not int or not 0<=elapsed_ms<300000
        or not isinstance(sha256,str) or HASH.fullmatch(sha256) is None
        or sha256!=evidence.get("target_checksum")):
        raise CrossPITRError("target LSN, 35 recovered records or later-record exclusion not proven")
    return {
        "schema_version":1,
        "status":"TWO_LINUX_KERNELS_REAL_PHYSICAL_WAL_PITR_RECOVERY_PASSED",
        "real_separate_kernel_wal_pitr_proved":True,
        "distinct_linux_kernels":True,
        "source_boot_id":evidence["source_boot_id"],
        "recovery_boot_id":evidence["recovery_boot_id"],
        "target_lsn":evidence["target_lsn"],
        "source_target_checksum":evidence["target_checksum"],
        "recovered_target_checksum":sha256,
        "expected_rows":35,
        "recovered_rows":rows,
        "post_target_committed_rows":5,
        "recovered_post_target_rows":later,
        "pitr_duration_ms":elapsed_ms,
        "run_nonce":evidence["run_nonce"],
        "basebackup_sha256":evidence["basebackup_sha256"],
        "wal_archive_sha256":evidence["wal_archive_sha256"],
        "source_report_sha256":evidence["source_report_sha256"],
        "source_primary_crash_proven":True,
        "target_wal_from_source_archive_replayed":True,
        "physical_host_power_loss_tested":False,
        "immutable_encrypted_offsite_storage_tested":False,
        "production_approved":False,
        "stage_041_complete":False,
    }

def _extract_archive(file:Path,work:Path,label:str)->Path:
    _tar_members(file,label)
    with tarfile.open(file,"r:gz") as tf:
        for member in tf:
            name=member.name.rstrip("/")
            parts=name.split("/")
            if not parts or parts[0]!=label or any(
                part in ("",".","..") or SAFE_PART.fullmatch(part) is None
                for part in parts):
                raise CrossPITRError("archive path traversal")
            target=work.joinpath(*parts)
            if member.isdir():
                target.mkdir(parents=True,exist_ok=True,mode=0o700)
                target.chmod(0o700)
            elif member.isfile():
                target.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
                source=tf.extractfile(member)
                if source is None:
                    raise CrossPITRError("archive regular file cannot be opened")
                with source, target.open("xb") as dest:
                    copied=0
                    while True:
                        chunk=source.read(1024*1024)
                        if not chunk:break
                        copied+=len(chunk)
                        if copied>member.size:
                            raise CrossPITRError("archive decompressed size exceeded")
                        dest.write(chunk)
                    if copied!=member.size:
                        raise CrossPITRError("short archive member")
                target.chmod(0o600)
            else:
                raise CrossPITRError("hostile physical backup archive member")
    return work/label

def recover(bundle_dir:Path,pg_bin:Path,report:Path)->dict:
    if (os.geteuid()==0 or os.uname().sysname!="Linux" or
        not pg_bin.is_absolute() or pg_bin.is_symlink() or not pg_bin.is_dir() or
        not report.is_absolute() or report.exists() or
        report.parent.is_symlink() or not report.parent.is_dir() or
        report.parent.stat().st_mode & 0o077):
        raise CrossPITRError("private nonroot PostgreSQL recovery runner required")
    for binary in ("pg_ctl","psql"):
        if not (pg_bin/binary).is_file():
            raise CrossPITRError("PostgreSQL tool missing: "+binary)
    boot=Path("/proc/sys/kernel/random/boot_id").read_text(encoding="ascii").strip()
    verified=inspect_bundle(bundle_dir,boot)
    work=Path(tempfile.mkdtemp(prefix="ocaf41-pitr-new-kernel-",dir="/tmp"))
    started=False
    bin=lambda k:str(pg_bin/k)
    try:
        data=_extract_archive(bundle_dir/"basebackup.tar.gz",work,"base")
        wal=_extract_archive(bundle_dir/"archived-wal.tar.gz",work,"wal")
        if (not (data/"PG_VERSION").is_file() or
            not (data/"global"/"pg_control").is_file() or
            not (wal/verified["target_wal_archived"]).is_file()
            or (data/"standby.signal").exists()):
            raise CrossPITRError("physical basebackup or target archived WAL segment absent")
        (data/"recovery.signal").touch(mode=0o600,exist_ok=False)
        config=data/"postgresql.conf"
        if config.is_symlink() or not config.is_file():
            raise CrossPITRError("physical recovery config missing")
        with config.open("a",encoding="utf-8") as o:
            o.write("\narchive_mode = off\n")
            o.write(f"restore_command = 'cp {wal}/%f %p'\n")
            o.write(f"recovery_target_lsn = '{verified['target_lsn']}'\n")
            o.write("recovery_target_inclusive = on\n")
            o.write("recovery_target_action = promote\n")
        port=pitr._available_ports()[0]
        start=time.monotonic()
        pitr._run([bin("pg_ctl"),"-D",str(data),
                   "-o",f"-p {port} -h 127.0.0.1",
                   "-l",str(work/"recovery.log"),"-w","start"],timeout=60)
        started=True
        def query(sql:str)->str:
            return pitr._run([bin("psql"),"-X","-A","-t","-q","-v","ON_ERROR_STOP=1",
                 "-h","127.0.0.1","-p",str(port),"-U","postgres",
                 "-d","postgres","-c",sql],timeout=8).stdout.strip()
        pitr._wait(lambda:query("SELECT pg_is_in_recovery()")=="f",
                   seconds=30,reason="new Linux kernel WAL replay did not promote")
        elapsed=int((time.monotonic()-start)*1000)
        count=int(query("SELECT count(*) FROM ocaf41_recovery"))
        unwanted=int(query("SELECT count(*) FROM ocaf41_recovery WHERE id BETWEEN 36 AND 40"))
        rows=query("SELECT id::text || ':' || payload FROM ocaf41_recovery ORDER BY id")
        checksum=hashlib.sha256(rows.encode("utf-8")).hexdigest()
        result=validate_restored(verified,count,checksum,unwanted,elapsed)
        result.update({
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "postgres_version":query("SHOW server_version"),
            "real_restored_postmaster_started":True,
            "real_wal_replay_exited_recovery":True,
            "transfer_type":"github_actions_SHA256_immutable_artifact_not_signed",
        })
        with report.open("x",encoding="utf-8") as out:
            json.dump(result,out,sort_keys=True,indent=2)
            out.write("\n")
        return result
    finally:
        if started:
            pitr._run([bin("pg_ctl"),"-D",str(work/"base"),"-m","immediate",
                       "-w","stop"],timeout=15,check=False)
        shutil.rmtree(work,ignore_errors=True)

def main()->int:
    ap=argparse.ArgumentParser(description=__doc__)
    for name in ("bundle-dir","pg-bin-dir","report"):
        ap.add_argument("--"+name,type=Path,required=True)
    ap.add_argument("--ack-ephemeral-two-kernel-physical-wal-pitr",action="store_true")
    x=ap.parse_args()
    if not x.ack_ephemeral_two_kernel_physical_wal_pitr:
        ap.error("explicit ephemeral two-kernel physical PITR acknowledgment required")
    try:
        result=recover(x.bundle_dir,x.pg_bin_dir,x.report)
    except (OSError,ValueError,tarfile.TarError) as exc:
        ap.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
