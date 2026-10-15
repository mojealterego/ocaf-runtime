#!/usr/bin/env python3
"""OCAF 0.41.3 — real PostgreSQL 16 physical base backup + archived WAL PITR.

On a single ephemeral Linux kernel, execute a real base backup, commit
transactions on both sides of a recorded LSN, force archival, crash the
original postmaster IMMEDIATELY, start a FRESH postmaster from the base
backup and replay WAL to the earlier target LSN. Assert that desired
transactions are recovered and later committed transactions are excluded.
This is NOT a physical host outage, a remote/immutable WAL archive,
a production fencing test, or continuous RPO/SLO certification.
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
import subprocess
import tempfile
import time

class PITRError(ValueError):
    pass

SHA256=re.compile(r"[0-9a-f]{64}\Z")
WALNAME=re.compile(r"[0-9A-F]{24}\Z")
LSN=re.compile(r"[0-9A-F]+/[0-9A-F]+\Z")
TRUES=(
    "postgres_server_real","physical_basebackup_taken_before_wal_transactions",
    "postgres_wal_archive_written","primary_immediate_crash_executed",
    "recovery_signal_and_target_lsn_used","restored_postmaster_started",
    "post_target_rows_excluded","last_backup_recovery_mode_exited",
)

def safe_wal_name(name:str)->str:
    if not isinstance(name,str) or WALNAME.fullmatch(name) is None:
        raise PITRError("invalid bounded PostgreSQL WAL archive segment")
    return name

def validate_pitr(report:dict)->dict:
    if (not isinstance(report,dict)
        or type(report.get("schema_version")) is not int or report["schema_version"]!=1
        or report.get("status")!="POSTGRES_REAL_WAL_POINT_IN_TIME_RECOVERY_PASSED"
        or any(report.get(k) is not True for k in TRUES)
        or type(report.get("expected_rows")) is not int or report["expected_rows"]!=35
        or type(report.get("actual_recovered_rows")) is not int or report["actual_recovered_rows"]!=35
        or type(report.get("post_target_rows")) is not int or report["post_target_rows"]!=5
        or not isinstance(report.get("original_target_checksum"),str)
        or SHA256.fullmatch(report["original_target_checksum"]) is None
        or report.get("restored_target_checksum")!=report["original_target_checksum"]
        or report.get("same_kernel") is not True
        or report.get("physical_host_crash_tested") is not False
        or report.get("offsite_wal_archive_tested") is not False
        or report.get("production_approved") is not False
        or report.get("stage_041_complete") is not False):
        raise PITRError("incomplete PITR, forbidden post-target rows or false production claim")
    return {"pitr_exact_target_restored":True,
            "post_target_committed_transactions_excluded":True,
            "production_approved":False}

def _run(args:list[str],*,timeout:int=40,check:bool=True)->subprocess.CompletedProcess:
    try:
        r=subprocess.run(args,capture_output=True,text=True,check=False,timeout=timeout)
    except (OSError,subprocess.TimeoutExpired) as exc:
        raise PITRError("bounded PostgreSQL PITR subprocess failed: "+args[0]) from exc
    if check and r.returncode!=0:
        raise PITRError("PostgreSQL PITR subprocess failed: "+args[0]+": "+r.stderr[-1300:])
    return r

def _wait(test,*,seconds:int=20,reason:str="timed out")->None:
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        try:
            if test():return
        except (PITRError,ValueError):
            pass
        time.sleep(.15)
    raise PITRError(reason)

def _available_ports()->tuple[int,int]:
    with socket.socket() as a, socket.socket() as b:
        a.bind(("127.0.0.1",0))
        b.bind(("127.0.0.1",0))
        return (a.getsockname()[1],b.getsockname()[1])

def execute(pg_bin:Path, report_path:Path)->dict:
    if (os.geteuid()==0 or os.uname().sysname!="Linux"
        or not isinstance(pg_bin,Path) or not pg_bin.is_absolute()
        or pg_bin.is_symlink() or not pg_bin.is_dir()
        or not report_path.is_absolute() or report_path.exists()
        or report_path.parent.is_symlink() or not report_path.parent.is_dir()
        or report_path.parent.stat().st_mode & 0o077):
        raise PITRError("dedicated unprivileged Linux PG with private output directory required")
    for binary in ("initdb","pg_ctl","pg_basebackup","psql"):
        if not (pg_bin/binary).is_file():
            raise PITRError("missing PostgreSQL 16 tool: "+binary)
    bin=lambda name:str(pg_bin/name)
    work=Path(tempfile.mkdtemp(prefix="ocaf41-pitr-",dir="/tmp"))
    data=work/"original"
    base=work/"basebackup"
    dest=work/"recovered"
    archive=work/"archive"
    archive.mkdir(mode=0o700)
    primary_port,recovery_port=_available_ports()
    primary_started=False
    recovery_started=False
    primary_crashed=False
    try:
        _run([bin("initdb"),"-D",str(data),"--auth-local=trust",
              "--auth-host=trust","--no-instructions"],timeout=60)
        c=data/"postgresql.conf"
        c.write_text(c.read_text()+
             "\nwal_level = replica\nmax_wal_senders = 8\n"
             "archive_mode = on\n"
             "archive_command = 'test ! -f "+str(archive)+"/%f && cp %p "+str(archive)+"/%f'\n"
             "full_page_writes = on\nfsync = on\n")
        _run([bin("pg_ctl"),"-D",str(data),
              "-o",f"-p {primary_port} -h 127.0.0.1",
              "-l",str(work/"primary.log"),"-w","start"],timeout=35)
        primary_started=True
        def query(port:int,sql:str,*,check:bool=True)->str:
            r=_run([bin("psql"),"-X","-A","-t","-q","-v","ON_ERROR_STOP=1",
                    "-h","127.0.0.1","-p",str(port),"-U","postgres",
                    "-d","postgres","-c",sql],timeout=6,check=check)
            if not check:return str(r.returncode)
            return r.stdout.strip()
        if query(primary_port,"SELECT pg_is_in_recovery()")!="f":
            raise PITRError("original PostgreSQL postmaster not writable")
        query(primary_port,"CREATE TABLE ocaf41_recovery(id INT PRIMARY KEY, payload TEXT NOT NULL)")
        query(primary_port,
              "INSERT INTO ocaf41_recovery "
              "SELECT n, repeat('ocaf41_pitr_',7)||n::text "
              "FROM generate_series(1,25) n")
        if query(primary_port,"SELECT count(*) FROM ocaf41_recovery")!="25":
            raise PITRError("base snapshot did not contain 25 committed rows")
        start=time.monotonic()
        _run([bin("pg_basebackup"),"-h","127.0.0.1","-p",str(primary_port),
              "-U","postgres","-D",str(base),"-Fp","-Xs","-P"],timeout=90)
        basebackup_ms=int((time.monotonic()-start)*1000)
        if not (base/"PG_VERSION").is_file():
            raise PITRError("actual PG basebackup not created")
        query(primary_port,
              "INSERT INTO ocaf41_recovery "
              "SELECT n, repeat('ocaf41_pitr_',7)||n::text "
              "FROM generate_series(26,35) n")
        target_lsn=query(primary_port,"SELECT pg_current_wal_lsn()")
        if LSN.fullmatch(target_lsn) is None:
            raise PITRError("invalid real recovery target WAL LSN")
        target_wal=safe_wal_name(query(primary_port,
            "SELECT pg_walfile_name(pg_current_wal_lsn())"))
        snapshot=query(primary_port,
             "SELECT id::text || ':' || payload FROM ocaf41_recovery ORDER BY id")
        target_digest=hashlib.sha256(snapshot.encode()).hexdigest()
        if query(primary_port,"SELECT count(*) FROM ocaf41_recovery")!="35":
            raise PITRError("expected 35 target records before harmful later writes")
        query(primary_port,
              "INSERT INTO ocaf41_recovery "
              "SELECT n, repeat('ocaf41_pitr_',7)||n::text "
              "FROM generate_series(36,40) n")
        if query(primary_port,"SELECT count(*) FROM ocaf41_recovery")!="40":
            raise PITRError("later transaction was not committed")
        # Force a complete WAL segment so the target and later commits are
        # genuinely available from archived WAL, not only local pg_wal.
        query(primary_port,"SELECT pg_switch_wal()")
        _wait(lambda:(archive/target_wal).is_file(),
              seconds=25,reason="target WAL segment never archived")
        archived_segments=len([x for x in archive.iterdir() if WALNAME.fullmatch(x.name)])
        if archived_segments<1:raise PITRError("no actual PG WAL segments archived")
        crash=_run([bin("pg_ctl"),"-D",str(data),
             "-m","immediate","-w","stop"],timeout=30)
        primary_started=False
        primary_crashed=True
        if query(primary_port,"SELECT 1",check=False)=="0":
            raise PITRError("original primary still serving after IMMEDIATE stop")
        shutil.copytree(base,dest,symlinks=False)
        # Cold physical recovery, not standby replication to original primary.
        if (dest/"standby.signal").exists():
            raise PITRError("unexpected standby.signal in basebackup")
        (dest/"recovery.signal").touch(mode=0o600,exist_ok=False)
        conf=dest/"postgresql.conf"
        conf.write_text(conf.read_text()+
            "\narchive_mode = off\n"
            "restore_command = 'cp "+str(archive)+"/%f %p'\n"
            "recovery_target_lsn = '"+target_lsn+"'\n"
            "recovery_target_inclusive = on\n"
            "recovery_target_action = promote\n")
        start=time.monotonic()
        _run([bin("pg_ctl"),"-D",str(dest),
              "-o",f"-p {recovery_port} -h 127.0.0.1",
              "-l",str(work/"restore.log"),"-w","start"],timeout=50)
        recovery_started=True
        _wait(lambda:query(recovery_port,"SELECT pg_is_in_recovery()")=="f",
              seconds=25,reason="PITR did not exit recovery at target LSN")
        elapsed=int((time.monotonic()-start)*1000)
        count=int(query(recovery_port,"SELECT count(*) FROM ocaf41_recovery"))
        excluded=int(query(recovery_port,
            "SELECT count(*) FROM ocaf41_recovery WHERE id BETWEEN 36 AND 40"))
        after=query(recovery_port,
            "SELECT id::text || ':' || payload FROM ocaf41_recovery ORDER BY id")
        recovered_digest=hashlib.sha256(after.encode()).hexdigest()
        if count!=35 or excluded!=0 or recovered_digest!=target_digest:
            raise PITRError("real WAL PITR target mismatch or post-target transaction replayed")
        proof={
            "schema_version":1,
            "status":"POSTGRES_REAL_WAL_POINT_IN_TIME_RECOVERY_PASSED",
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "postgres_version":query(recovery_port,"SHOW server_version"),
            "host_boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "postgres_server_real":True,
            "physical_basebackup_taken_before_wal_transactions":True,
            "postgres_wal_archive_written":True,
            "archived_wal_segments":archived_segments,
            "recovery_target_lsn":target_lsn,
            "target_wal_archived":target_wal,
            "primary_immediate_crash_executed":primary_crashed,
            "primary_tcp_unreachable":True,
            "recovery_signal_and_target_lsn_used":True,
            "restored_postmaster_started":True,
            "original_rows_at_basebackup":25,
            "expected_rows":35,
            "actual_recovered_rows":count,
            "post_target_rows":5,
            "post_target_rows_excluded":excluded==0,
            "original_target_checksum":target_digest,
            "restored_target_checksum":recovered_digest,
            "last_backup_recovery_mode_exited":True,
            "basebackup_duration_ms":basebackup_ms,
            "pitr_restart_duration_ms":elapsed,
            "same_kernel":True,
            "physical_host_crash_tested":False,
            "offsite_wal_archive_tested":False,
            "production_approved":False,
            "stage_041_complete":False,
        }
        validate_pitr(proof)
        with report_path.open("x") as fp:
            json.dump(proof,fp,sort_keys=True,indent=2)
            fp.write("\n")
        return proof
    finally:
        if recovery_started:
            _run([bin("pg_ctl"),"-D",str(dest),"-m","immediate",
                 "-w","stop"],timeout=15,check=False)
        if primary_started:
            _run([bin("pg_ctl"),"-D",str(data),"-m","immediate",
                 "-w","stop"],timeout=15,check=False)
        shutil.rmtree(work,ignore_errors=True)

def main()->int:
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pg-bin-dir",type=Path,required=True)
    p.add_argument("--report",type=Path,required=True)
    p.add_argument("--ack-ephemeral-postgres-pitr-crash-lab",action="store_true")
    args=p.parse_args()
    if not args.ack_ephemeral_postgres_pitr_crash_lab:
        p.error("explicit isolated ephemeral real PostgreSQL crash acknowledgment required")
    try:
        result=execute(args.pg_bin_dir,args.report)
    except (OSError,ValueError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
