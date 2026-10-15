#!/usr/bin/env python3
"""Stage 0.41: actual PostgreSQL custom-format backup, restore and checksum lab.

Requires an EPHEMERAL dedicated non-root PostgreSQL instance and its
separate throwaway databases. Measures custom-format pg_dump, pg_restore,
exact ordered record replay, backup hash pinning and deliberate tamper
rejection. One Linux host only; does NOT imply physical DR or RPO after
a remote host loss, external audit, or production release readiness.
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
import socket
import stat
import subprocess
import tempfile
import time

class BackupRestoreError(ValueError):pass


def hash_backup(path:Path)->str:
    if not isinstance(path,Path) or path.is_symlink() or not path.is_file():
        raise BackupRestoreError("backup regular file absent or symlinked")
    fd=os.open(path,os.O_RDONLY|getattr(os,"O_NOFOLLOW",0))
    try:
        before=os.fstat(fd)
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink!=1 or
                not 0<before.st_size<=64*1024*1024):
            raise BackupRestoreError("backup exceeds bounded safe inode")
        digest=hashlib.sha256()
        while True:
            part=os.read(fd,65536)
            if not part:break
            digest.update(part)
        after=os.fstat(fd)
        if (before.st_size!=after.st_size or before.st_ino!=after.st_ino
                or before.st_mtime_ns!=after.st_mtime_ns):
            raise BackupRestoreError("backup changed while hashing")
        return digest.hexdigest()
    finally:os.close(fd)


def require_backup_digest(path:Path,digest:str)->None:
    if (not isinstance(digest,str) or
            re.fullmatch(r"[0-9a-f]{64}",digest) is None
            or hash_backup(path)!=digest):
        raise BackupRestoreError("backup SHA256 mismatch or invalid pin")


def verify_restore(proof:dict)->dict:
    if (not isinstance(proof,dict) or
        type(proof.get("backup_size_bytes")) is not int
        or not 0<proof["backup_size_bytes"]<=64*1024*1024
        or type(proof.get("restored_rows")) is not int
        or type(proof.get("original_rows")) is not int
        or proof["original_rows"]<=0
        or proof["original_rows"]!=proof["restored_rows"]
        or not isinstance(proof.get("original_checksum"),str)
        or re.fullmatch("[0-9a-f]{64}",proof["original_checksum"]) is None
        or proof.get("restored_checksum")!=proof["original_checksum"]
        or any(proof.get(key) is not True for key in
            ("tampered_backup_rejected","backup_was_custom_pg_dump",
             "restore_was_pg_restore","postgres_server_real"))
        or proof.get("production_approved") is not False):
        raise BackupRestoreError("real PostgreSQL restore evidence incomplete, mismatched or falsely approved")
    return {
        "restore_exact_match":True,
        "measured_backup_restore":True,
        "data_durable_for_local_pg_dump_test":True,
        "physical_host_dr_tested":False,
        "production_approved":False,
    }


def _run(argv:list[str],timeout=30,check=True)->subprocess.CompletedProcess:
    try:
        r=subprocess.run(argv,capture_output=True,text=True,check=False,timeout=timeout)
    except (OSError,subprocess.TimeoutExpired) as exc:
        raise BackupRestoreError("bounded PostgreSQL subprocess unavailable: "+argv[0]) from exc
    if check and r.returncode!=0:
        raise BackupRestoreError("PostgreSQL command failed: "+argv[0]+": "+r.stderr[-650:])
    return r


def _available_port()->int:
    with socket.socket(socket.AF_INET,socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1",0))
        return s.getsockname()[1]


def execute(pg_bin:Path,output_dir:Path)->dict:
    if os.geteuid()==0 or os.uname().sysname!="Linux":
        raise BackupRestoreError("non-root ephemeral Linux postgres user required")
    if (not isinstance(pg_bin,Path) or not pg_bin.is_absolute()
            or pg_bin.is_symlink() or not pg_bin.is_dir()):
        raise BackupRestoreError("absolute regular PostgreSQL binary directory required")
    for name in ("initdb","pg_ctl","psql","pg_dump","pg_restore","createdb"):
        if not (pg_bin/name).is_file():
            raise BackupRestoreError("PostgreSQL tool missing: "+name)
    if (not output_dir.is_absolute() or output_dir.is_symlink()
            or not output_dir.is_dir() or output_dir.stat().st_mode & 0o077
            or any(output_dir.iterdir())):
        raise BackupRestoreError("empty private evidence output directory required")
    bin=lambda name:str(pg_bin/name)
    temp=Path(tempfile.mkdtemp(prefix="ocaf41-pg-restore-",dir="/tmp"))
    dbdir=temp/"pgdata"
    port=_available_port()
    started=False
    try:
        _run([bin("initdb"),"-D",str(dbdir),
              "--auth-local=trust","--auth-host=trust","--no-instructions"],timeout=60)
        _run([bin("pg_ctl"),"-D",str(dbdir),
            "-o",f"-p {port} -h 127.0.0.1",
            "-l",str(temp/"pg.log"),"-w","start"])
        started=True
        base=[ "-h","127.0.0.1","-p",str(port),"-U","postgres"]
        def sql(db,text):
            return _run([bin("psql"),"-X","-q","-A","-t",
                "-v","ON_ERROR_STOP=1",*base,"-d",db,"-c",text]).stdout.strip()
        for db in ("ocaf41_source","ocaf41_restored"):
            _run([bin("createdb"),*base,db])
        sql("ocaf41_source","CREATE TABLE durable_receipts (id int PRIMARY KEY, payload text NOT NULL)")
        sql("ocaf41_source","INSERT INTO durable_receipts SELECT n, repeat('ocaf41_',10) || n::text FROM generate_series(1,25) AS n")
        ordered="SELECT id::text || ':' || payload FROM durable_receipts ORDER BY id"
        before=sql("ocaf41_source",ordered)
        original_count=int(sql("ocaf41_source","SELECT COUNT(*) FROM durable_receipts"))
        original_hash=hashlib.sha256(before.encode()).hexdigest()
        dump=output_dir/"snapshot.pgcustom"
        if dump.exists():
            raise BackupRestoreError("snapshot unexpectedly exists")
        start=time.monotonic()
        _run([bin("pg_dump"),*base,"-d","ocaf41_source",
             "--format=custom","--no-owner","--no-acl","--file",str(dump)],timeout=30)
        measured_dump_ms=int((time.monotonic()-start)*1000)
        expected_hash=hash_backup(dump)
        require_backup_digest(dump,expected_hash)
        bad=output_dir/"corrupted.pgcustom"
        shutil.copyfile(dump,bad)
        with bad.open("r+b") as handle:
            start_byte=handle.read(1)
            if not start_byte:raise BackupRestoreError("snapshot empty")
            handle.seek(0)
            handle.write(bytes([start_byte[0]^0x01]))
        denied=False
        try:require_backup_digest(bad,expected_hash)
        except BackupRestoreError:denied=True
        bad.unlink()
        if not denied:raise BackupRestoreError("corrupted pg_dump unexpectedly accepted")
        start=time.monotonic()
        _run([bin("pg_restore"),*base,"-d","ocaf41_restored",
              "--exit-on-error","--no-owner","--no-acl",str(dump)],timeout=30)
        measured_restore_ms=int((time.monotonic()-start)*1000)
        after=sql("ocaf41_restored",ordered)
        restored_count=int(sql("ocaf41_restored","SELECT COUNT(*) FROM durable_receipts"))
        proof={
          "schema_version":1,
          "status":"POSTGRES_REAL_BACKUP_RESTORE_IDENTICAL_RECORDS_PASSED",
          "observed_at_utc":datetime.now(timezone.utc).isoformat(),
          "postgres_version":sql("ocaf41_restored","SHOW server_version"),
          "backup_file":dump.name,
          "backup_sha256":expected_hash,
          "backup_size_bytes":dump.stat().st_size,
          "measured_dump_ms":measured_dump_ms,
          "measured_restore_ms":measured_restore_ms,
          "original_rows":original_count,
          "restored_rows":restored_count,
          "original_checksum":original_hash,
          "restored_checksum":hashlib.sha256(after.encode()).hexdigest(),
          "tampered_backup_rejected":denied,
          "backup_was_custom_pg_dump":True,
          "restore_was_pg_restore":True,
          "postgres_server_real":True,
          "same_kernel_source_and_restore":True,
          "rpo_records_lost":0 if original_hash==hashlib.sha256(after.encode()).hexdigest() else None,
          "cross_host_restore_tested":False,
          "network_partition_tested":False,
          "production_approved":False,
          "stage_041_complete":False,
        }
        verify_restore(proof)
        report=output_dir/"backup-restore-report.json"
        with report.open("x") as fp:
            json.dump(proof,fp,indent=2,sort_keys=True)
            fp.write("\n")
        return proof
    finally:
        if started:
            _run([bin("pg_ctl"),"-D",str(dbdir),"-m","immediate","-w","stop"],
                 timeout=15,check=False)
        shutil.rmtree(temp,ignore_errors=True)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pg-bin-dir",required=True,type=Path)
    p.add_argument("--output-dir",required=True,type=Path)
    p.add_argument("--ack-ephemeral-postgres-backup-lab",action="store_true")
    args=p.parse_args()
    if not args.ack_ephemeral_postgres_backup_lab:
        p.error("ephemeral local test database authorization required")
    try:
        result=execute(args.pg_bin_dir,args.output_dir)
    except (OSError,ValueError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
