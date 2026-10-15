#!/usr/bin/env python3
"""Stage 0.40 — real PostgreSQL 16 1-primary/2-standby WAL failover LAB.

This operates on THREE independent PostgreSQL POSTMASTER PROCESSES running
on ONE ephemeral Linux kernel, NOT three physical hosts or a production
consensus cluster. It tests actual streaming WAL replay, an IMMEDIATE
primary server crash, standby promotion and standby reparent/replay.
The old primary remains STOPPED, not externally fenced from future start.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import tempfile
import time


class PostgresHAError(ValueError):
    pass


REQUIRED_TRUE=(
    "distinct_postgres_ports",
    "primary_was_writable",
    "both_standbys_replayed_pre_crash_wal",
    "old_primary_killed_immediate",
    "old_primary_unreachable_after_crash",
    "standby_b_promoted_writable",
    "standby_c_followed_promoted_b",
    "standby_c_replayed_post_promotion_wal",
)


def validate_local_pg_ports(ports:tuple[int,int,int])->None:
    if (not isinstance(ports,tuple) or len(ports)!=3
            or any(type(x) is not int or x<1024 or x>65535 for x in ports)
            or len(set(ports))!=3):
        raise PostgresHAError("three distinct nonprivileged local PostgreSQL ports required")


def validate_report(r:dict)->dict:
    if (not isinstance(r,dict) or
            r.get("schema_version")!=1 or
            r.get("status")!="POSTGRES_3_PROCESSES_STREAMING_FAILOVER_PASSED" or
            type(r.get("nodes_count")) is not int or r["nodes_count"]!=3 or
            any(r.get(k) is not True for k in REQUIRED_TRUE) or
            r.get("all_three_same_kernel") is not True or
            r.get("host_power_loss_tested") is not False or
            r.get("production_approved") is not False or
            r.get("stage_040_complete") is not False):
        raise PostgresHAError("incomplete or falsely approved actual PostgreSQL lab report")
    return r


def _run(argv:list[str],timeout:int=30,check:bool=True)->subprocess.CompletedProcess:
    try:
        result=subprocess.run(argv,check=False,text=True,capture_output=True,timeout=timeout)
    except (OSError,subprocess.TimeoutExpired) as exc:
        raise PostgresHAError("missing or timed-out PostgreSQL lab command: "+argv[0]) from exc
    if check and result.returncode!=0:
        raise PostgresHAError("PostgreSQL lab command failed: "+argv[0]+": "+result.stderr[-700:])
    return result


def _ports()->tuple[int,int,int]:
    for _ in range(10):
        sockets=[]
        try:
            for _ in range(3):
                s=socket.socket(socket.AF_INET,socket.SOCK_STREAM)
                s.bind(("127.0.0.1",0))
                sockets.append(s)
            ports=tuple(s.getsockname()[1] for s in sockets)
            validate_local_pg_ports(ports)
            return ports
        finally:
            for s in sockets:s.close()
    raise PostgresHAError("cannot allocate three distinct local PostgreSQL ports")


def _wait(test,seconds:int=15)->None:
    limit=time.monotonic()+seconds
    last=None
    while time.monotonic()<limit:
        try:
            if test():return
        except PostgresHAError as exc:
            last=exc
        time.sleep(.15)
    raise PostgresHAError("bounded PostgreSQL WAL catch-up timed out: "+str(last))


def run_lab(*,pg_bin:Path,report:Path)->dict:
    if (os.geteuid()==0 or os.uname().sysname!="Linux"
            or not isinstance(pg_bin,Path) or not pg_bin.is_absolute()
            or pg_bin.is_symlink() or not pg_bin.is_dir()):
        raise PostgresHAError("nonroot dedicated ephemeral Linux + absolute PostgreSQL binary dir required")
    needed=("initdb","pg_ctl","pg_basebackup","psql")
    for binary in needed:
        if not (pg_bin/binary).is_file():
            raise PostgresHAError("PostgreSQL tools not installed: "+binary)
    bindir=lambda k:str(pg_bin/k)
    ports=_ports()
    state=Path(tempfile.mkdtemp(prefix="ocaf40-pg-ha-",dir="/tmp"))
    for node in ("a","b","c"):
        (state/node).mkdir(mode=0o700)
    paths={key:state/key for key in ("a","b","c")}
    started=set()
    failed_immediate=False
    try:
        def pgctl(node:str,*args,timeout=25):
            return _run([bindir("pg_ctl"),"-D",str(paths[node]),*args],timeout=timeout)
        def start(node:str):
            index={"a":0,"b":1,"c":2}[node]
            pgctl(node,"-o",f"-p {ports[index]} -h 127.0.0.1",
                  "-l",str(state/(node+".log")),"-w","start",timeout=25)
            started.add(node)
        def query(node:str,sql:str,check=True)->str:
            port=ports[{"a":0,"b":1,"c":2}[node]]
            res=_run([bindir("psql"),"-X","-q","-A","-t",
                 "-v","ON_ERROR_STOP=1","-h","127.0.0.1","-p",str(port),
                 "-U","postgres","-d","postgres","-c",sql],
                check=check,timeout=5)
            if not check:return str(res.returncode)
            return res.stdout.strip()
        _run([bindir("initdb"),"-D",str(paths["a"]),
              "--auth-local=trust","--auth-host=trust",
              "--no-instructions"],timeout=45)
        conf=paths["a"]/"postgresql.conf"
        conf.write_text(conf.read_text()+"\nwal_level = replica\nmax_wal_senders = 8\n"
                        "max_replication_slots = 8\nhot_standby = on\n"
                        "synchronous_commit = on\nfsync = on\n")
        start("a")
        if query("a","SELECT pg_is_in_recovery()")!="f":
            raise PostgresHAError("original primary reports recovery mode")
        query("a","CREATE TABLE ocaf40_events (id integer PRIMARY KEY, payload text NOT NULL)")
        query("a","INSERT INTO ocaf40_events VALUES (1,'pre-standby')")
        for name in ("b","c"):
            _run([bindir("pg_basebackup"),"-h","127.0.0.1",
                  "-p",str(ports[0]),"-U","postgres",
                  "-D",str(paths[name]),"-Fp","-Xs","-R"],timeout=60)
            start(name)
            _wait(lambda n=name:query(n,"SELECT pg_is_in_recovery()")=="t"
                  and query(n,"SELECT COUNT(*) FROM ocaf40_events")=="1",seconds=20)
        pre_pid={node:query(node,"SELECT pg_backend_pid()") for node in ("a","b","c")}
        query("a","INSERT INTO ocaf40_events VALUES (2,'pre-crash')")
        for n in ("b","c"):
            _wait(lambda n=n:query(n,"SELECT COUNT(*) FROM ocaf40_events")=="2",seconds=20)
        # A crash is real: request PostgreSQL's IMMEDIATE termination; no
        # graceful checkpoint. This is NOT a host power-off experiment.
        pgctl("a","-m","immediate","-w","stop")
        started.discard("a")
        failed_immediate=True
        if query("a","SELECT 1",check=False)=="0":
            raise PostgresHAError("former primary still accepting TCP after crash")
        pgctl("b","-w","promote")
        _wait(lambda:query("b","SELECT pg_is_in_recovery()")=="f")
        query("b","INSERT INTO ocaf40_events VALUES (3,'post-promotion')")
        # Reparent C from stopped primary A to new timeline on promoted B.
        pgctl("c","-m","fast","-w","stop")
        started.discard("c")
        auto=paths["c"]/"postgresql.auto.conf"
        before=auto.read_text()
        cleaned="\n".join(x for x in before.splitlines()
            if not x.lstrip().startswith("primary_conninfo"))+"\n"
        cleaned+=f"primary_conninfo = 'host=127.0.0.1 port={ports[1]} user=postgres application_name=ocaf40_reparent_c'\n"
        auto.write_text(cleaned)
        start("c")
        _wait(lambda:query("c","SELECT pg_is_in_recovery()")=="t"
              and query("c","SELECT COUNT(*) FROM ocaf40_events")=="3",
              seconds=25)
        wal_receiver=query("c","SELECT status FROM pg_stat_wal_receiver LIMIT 1")
        if wal_receiver!="streaming":
            raise PostgresHAError("standby C did not establish new streaming WAL connection to B")
        if (len(set(pre_pid.values()))!=3 or
                query("b","SELECT COUNT(*) FROM ocaf40_events")!="3"):
            raise PostgresHAError("PostgreSQL processes or WAL handover inconsistent")
        result={
            "schema_version":1,
            "status":"POSTGRES_3_PROCESSES_STREAMING_FAILOVER_PASSED",
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "nodes_count":3,
            "postgres_version":query("b","SHOW server_version"),
            "ports":list(ports),
            "distinct_postgres_ports":True,
            "primary_was_writable":True,
            "both_standbys_replayed_pre_crash_wal":True,
            "old_primary_killed_immediate":failed_immediate,
            "old_primary_unreachable_after_crash":True,
            "standby_b_promoted_writable":True,
            "standby_c_followed_promoted_b":True,
            "standby_c_replayed_post_promotion_wal":True,
            "standby_c_wal_receiver_status":wal_receiver,
            "primary_a_backend_pid_before_crash":int(pre_pid["a"]),
            "standby_b_backend_pid_before_crash":int(pre_pid["b"]),
            "standby_c_backend_pid_before_crash":int(pre_pid["c"]),
            "kernel_boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "all_three_same_kernel":True,
            "three_physically_independent_hosts":False,
            "old_primary_external_restart_fencing_tested":False,
            "host_power_loss_tested":False,
            "split_brain_after_old_primary_restart_tested":False,
            "production_approved":False,
            "stage_040_complete":False,
        }
        validate_report(result)
        if report.exists() or report.is_symlink():
            raise PostgresHAError("report destination already exists")
        report.parent.mkdir(parents=True,exist_ok=True)
        with report.open("x") as sink:
            json.dump(result,sink,indent=2,sort_keys=True)
            sink.write("\n")
        return result
    finally:
        for node in ("c","b","a"):
            if node in started:
                _run([bindir("pg_ctl"),"-D",str(paths[node]),"-m","immediate",
                      "-w","stop"],check=False,timeout=15)
        shutil.rmtree(state,ignore_errors=True)


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pg-bin-dir",type=Path,required=True)
    parser.add_argument("--report",type=Path,required=True)
    parser.add_argument("--ack-local-ephemeral-postgres-primary-kill",action="store_true")
    a=parser.parse_args()
    if not a.ack_local_ephemeral_postgres_primary_kill:
        parser.error("explicit local ephemeral PostgreSQL crash acknowledgement required")
    try:
        result=run_lab(pg_bin=a.pg_bin_dir,report=a.report)
    except (OSError,ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
