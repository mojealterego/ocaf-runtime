#!/usr/bin/env python3
"""OCAF 0.41.2: verify GitHub artifact handoff, restore real PostgreSQL on a new kernel.

Both jobs are ephemeral GitHub Actions Linux VMs. The source must have
stopped its own PostgreSQL process BEFORE the recovery job begins.
This checks SHA-256 artifact transfer, unique Linux boot IDs, real
pg_restore into a FRESH database and exact 25-row content digest.
No claim of a cryptographically signed trusted backup, physical host
disaster recovery, automatic RTO or production readiness is made.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import socket
import stat
import tempfile
import time

import stage041_backup_restore as backup

UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z")
SHA = re.compile(r"[0-9a-f]{64}\Z")
NONCE = re.compile(r"[0-9a-f]{16}\Z")
MANIFEST_FIELDS = {"schema_version","source_boot_id","source_report_sha256",
                   "snapshot_sha256","snapshot_size_bytes","run_nonce","production_approved"}


class CrossHostRestoreError(ValueError):
    pass


def _unique(pairs):
    d={}
    for key,value in pairs:
        if key in d:
            raise CrossHostRestoreError("duplicate evidence JSON key")
        d[key]=value
    return d


def _json_regular(file:Path,max_bytes:int)->tuple[dict,bytes]:
    if not isinstance(file,Path) or not file.is_absolute() or file.is_symlink():
        raise CrossHostRestoreError("evidence JSON requires nonsymlink absolute file")
    try:
        if not file.is_file() or not 0<file.stat().st_size<=max_bytes:
            raise CrossHostRestoreError("missing or oversized evidence JSON")
        fd=os.open(file,os.O_RDONLY|getattr(os,"O_NOFOLLOW",0))
        try:
            before=os.fstat(fd)
            if not stat.S_ISREG(before.st_mode) or before.st_nlink!=1:
                raise CrossHostRestoreError("unsafe evidence inode")
            blob=os.read(fd,max_bytes+1)
            after=os.fstat(fd)
            if len(blob)>max_bytes or after.st_size!=len(blob) or before.st_ino!=after.st_ino:
                raise CrossHostRestoreError("evidence mutated during read")
        finally:
            os.close(fd)
        data=json.loads(blob,object_pairs_hook=_unique,
                        parse_constant=lambda _:(_ for _ in ()).throw(CrossHostRestoreError("nonfinite JSON")))
    except (OSError,UnicodeError,TypeError,ValueError) as exc:
        raise CrossHostRestoreError("invalid or mutated transfer JSON") from exc
    if not isinstance(data,dict):
        raise CrossHostRestoreError("transfer evidence must be an object")
    return data,blob


def validate_transfer(snapshot:Path, source_report:Path,
                      manifest_file:Path, destination_boot_id:str)->dict:
    if (not isinstance(destination_boot_id,str)
            or UUID.fullmatch(destination_boot_id) is None):
        raise CrossHostRestoreError("real destination Linux boot_id missing")
    source,raw_source=_json_regular(source_report,131072)
    manifest,_=_json_regular(manifest_file,4096)
    if (set(manifest)!=MANIFEST_FIELDS
            or type(manifest["schema_version"]) is not int or manifest["schema_version"]!=1
            or type(manifest["production_approved"]) is not bool or manifest["production_approved"] is not False
            or not isinstance(manifest["source_boot_id"],str)
            or UUID.fullmatch(manifest["source_boot_id"]) is None
            or manifest["source_boot_id"]==destination_boot_id
            or not isinstance(manifest["run_nonce"],str) or NONCE.fullmatch(manifest["run_nonce"]) is None
            or not isinstance(manifest["source_report_sha256"],str)
            or SHA.fullmatch(manifest["source_report_sha256"]) is None
            or hashlib.sha256(raw_source).hexdigest()!=manifest["source_report_sha256"]
            or type(manifest["snapshot_size_bytes"]) is not int
            or not 5<manifest["snapshot_size_bytes"]<=64*1024*1024
            or not isinstance(manifest["snapshot_sha256"],str)
            or SHA.fullmatch(manifest["snapshot_sha256"]) is None):
        raise CrossHostRestoreError("artifact manifest, source/destination host or SHA256 invalid")
    try:
        proof=backup.verify_restore(source)
    except (ValueError,TypeError) as exc:
        raise CrossHostRestoreError("source host local pg_dump/pg_restore not proven") from exc
    if (source.get("schema_version")!=1
            or source.get("status")!="POSTGRES_REAL_BACKUP_RESTORE_IDENTICAL_RECORDS_PASSED"
            or source.get("backup_file")!="snapshot.pgcustom"
            or source.get("same_kernel_source_and_restore") is not True
            or source.get("cross_host_restore_tested") is not False
            or source.get("stage_041_complete") is not False
            or source.get("backup_sha256")!=manifest["snapshot_sha256"]
            or source.get("backup_size_bytes")!=manifest["snapshot_size_bytes"]
            or type(source.get("original_rows")) is not int
            or source["original_rows"]!=25):
        raise CrossHostRestoreError("source report does not prove bounded 25-row lab snapshot")
    if not isinstance(snapshot,Path) or not snapshot.is_absolute() or snapshot.is_symlink():
        raise CrossHostRestoreError("snapshot must be a nonsymlinked absolute file")
    if snapshot.name!="snapshot.pgcustom":
        raise CrossHostRestoreError("unexpected archive name")
    try:
        if snapshot.stat().st_size!=manifest["snapshot_size_bytes"]:
            raise CrossHostRestoreError("cross-host snapshot size differs")
        backup.require_backup_digest(snapshot,manifest["snapshot_sha256"])
        with snapshot.open("rb") as f:
            if f.read(5)!=b"PGDMP":
                raise CrossHostRestoreError("not PostgreSQL custom archive magic")
    except (OSError,ValueError) as exc:
        raise CrossHostRestoreError("transfer blob failed size/SHA/custom-format verification") from exc
    return {
        "original_rows":source["original_rows"],
        "original_checksum":source["original_checksum"],
        "source_boot_id":manifest["source_boot_id"],
        "recovery_boot_id":destination_boot_id,
        "run_nonce":manifest["run_nonce"],
        "source_report_sha256":manifest["source_report_sha256"],
        "snapshot_sha256":manifest["snapshot_sha256"],
        "snapshot_size_bytes":manifest["snapshot_size_bytes"],
        "distinct_linux_kernels":True,
        "source_verified_local_restore":proof["restore_exact_match"],
        "source_reported_at_utc":source.get("observed_at_utc"),
        "production_approved":False
    }


def validate_recovered_rows(evidence:dict,rows:int,checksum:str,restore_ms:int)->dict:
    if (not isinstance(evidence,dict)
            or evidence.get("distinct_linux_kernels") is not True
            or evidence.get("source_verified_local_restore") is not True
            or evidence.get("production_approved") is not False
            or type(rows) is not int or rows!=evidence.get("original_rows")
            or type(restore_ms) is not int or not 0<=restore_ms<=300000
            or not isinstance(checksum,str) or SHA.fullmatch(checksum) is None
            or checksum!=evidence.get("original_checksum")):
        raise CrossHostRestoreError("actual independent kernel pg_restore row/hash mismatch")
    return {
        "schema_version":1,
        "status":"REAL_TWO_KERNEL_POSTGRESQL_SHA256_RESTORE_PASSED",
        "distinct_linux_kernels":True,
        "actual_postgres_restore_confirmed":True,
        "source_report_sha256":evidence["source_report_sha256"],
        "snapshot_sha256":evidence["snapshot_sha256"],
        "source_boot_id":evidence["source_boot_id"],
        "recovery_boot_id":evidence["recovery_boot_id"],
        "run_nonce":evidence["run_nonce"],
        "original_rows":evidence["original_rows"],
        "recovered_rows":rows,
        "original_checksum":evidence["original_checksum"],
        "recovered_checksum":checksum,
        "pg_restore_duration_ms":restore_ms,
        "rpo_at_backup_boundary_rows_lost":0,
        "physical_host_disaster_recovery_tested":False,
        "automated_live_failover_tested":False,
        "encrypted_offsite_backup_tested":False,
        "stage_041_complete":False,
        "production_approved":False
    }


def restore_on_new_kernel(*,snapshot:Path,source_report:Path,manifest:Path,
                          pg_bin:Path,report:Path)->dict:
    if (os.geteuid()==0 or os.uname().sysname!="Linux"
            or not pg_bin.is_absolute() or pg_bin.is_symlink() or not pg_bin.is_dir()):
        raise CrossHostRestoreError("dedicated nonroot Linux PostgreSQL restore node required")
    for binary in ("initdb","pg_ctl","psql","pg_restore","createdb"):
        if not (pg_bin/binary).is_file():
            raise CrossHostRestoreError("missing PostgreSQL restore tool "+binary)
    if report.exists() or report.is_symlink() or not report.parent.is_dir():
        raise CrossHostRestoreError("preexisting or missing recovery report directory")
    boot=Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    evidence=validate_transfer(snapshot,source_report,manifest,boot)
    binary=lambda x:str(pg_bin/x)
    work=Path(tempfile.mkdtemp(prefix="ocaf41-cross-kernel-restore-",dir="/tmp"))
    cluster=work/"data"
    port=backup._available_port()
    started=False
    try:
        backup._run([binary("initdb"),"-D",str(cluster),
                      "--auth-local=trust","--auth-host=trust","--no-instructions"],timeout=60)
        backup._run([binary("pg_ctl"),"-D",str(cluster),
             "-o",f"-p {port} -h 127.0.0.1","-l",str(work/"pg.log"),"-w","start"])
        started=True
        base=["-h","127.0.0.1","-p",str(port),"-U","postgres"]
        backup._run([binary("createdb"),*base,"ocaf41_recovered"])
        # Revalidate artifact from a separate inode before passing to pg_restore.
        backup.require_backup_digest(snapshot,evidence["snapshot_sha256"])
        started_restore=time.monotonic()
        backup._run([binary("pg_restore"),*base,"-d","ocaf41_recovered",
                     "--exit-on-error","--no-owner","--no-acl",str(snapshot)],timeout=75)
        restore_ms=int((time.monotonic()-started_restore)*1000)
        def sql(expr):
            return backup._run([binary("psql"),"-X","-q","-A","-t",
                "-v","ON_ERROR_STOP=1",*base,"-d","ocaf41_recovered",
                "-c",expr],timeout=10).stdout.strip()
        count=int(sql("SELECT COUNT(*) FROM durable_receipts"))
        body=sql("SELECT id::text || ':' || payload FROM durable_receipts ORDER BY id")
        checksum=hashlib.sha256(body.encode()).hexdigest()
        result=validate_recovered_rows(evidence,count,checksum,restore_ms)
        result.update({
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "pg_version":sql("SHOW server_version"),
            "fresh_postgres_process_started":True,
            "new_database_created":True,
            "source_host_process_not_reused":True,
            "transfer_type":"github_actions_immutable_artifact_SHA256_not_signed",
        })
        with report.open("x") as out:
            json.dump(result,out,sort_keys=True,indent=2)
            out.write("\n")
        return result
    finally:
        if started:
            backup._run([binary("pg_ctl"),"-D",str(cluster),"-m","immediate",
                         "-w","stop"],timeout=15,check=False)
        shutil.rmtree(work,ignore_errors=True)


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    for key in ("snapshot","source-report","manifest","pg-bin-dir","report"):
        parser.add_argument("--"+key,type=Path,required=True)
    parser.add_argument("--ack-cross-kernel-ephemeral-restore",action="store_true")
    a=parser.parse_args()
    if not a.ack_cross_kernel_ephemeral_restore:
        parser.error("explicit temporary nonproduction independent-kernel restore acknowledgement required")
    try:
        result=restore_on_new_kernel(
            snapshot=a.snapshot,source_report=a.source_report,
            manifest=a.manifest,pg_bin=a.pg_bin_dir,report=a.report)
    except (OSError,ValueError,backup.BackupRestoreError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
