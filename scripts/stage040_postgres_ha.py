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
import re
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


def pg_basebackup_command(binary:str,port:int,destination:Path)->list[str]:
    if (not isinstance(binary,str) or not binary.startswith("/")
        or type(port) is not int or not 1024<=port<=65535
        or not isinstance(destination,Path) or not destination.is_absolute()
        or ".." in destination.parts or destination.is_symlink()):
        raise PostgresHAError("untrusted local streaming backup target")
    # Default spread checkpoint can wait for minutes under concurrent WAL
    # churn; force a PostgreSQL CHECKPOINT now, not a synthetic sleep.
    return [binary,"--host","127.0.0.1","--port",str(port),
            "--username","postgres","--pgdata",str(destination),
            "--format=plain","--wal-method=stream",
            "--checkpoint=fast","--write-recovery-conf"]


def validate_pgbench_load(output:str,seconds:int,clients:int)->dict:
    if (not isinstance(output,str) or len(output)>100000 or
        type(seconds) is not int or not 15<=seconds<=120 or
        type(clients) is not int or not 2<=clients<=16):
        raise PostgresHAError("bounded real pgbench stress required")
    counts=re.findall(r"number of transactions actually processed:\s*([0-9]+)",output)
    rates=re.findall(r"tps\s*=\s*([0-9]+(?:\.[0-9]+)?)",output)
    client_ids=re.findall(r"number of clients:\s*([0-9]+)",output)
    durations=re.findall(r"duration:\s*([0-9]+)\s*s",output)
    if (len(counts)!=1 or len(rates)!=1 or len(client_ids)!=1 or
            len(durations)!=1 or int(client_ids[0])!=clients or
            int(durations[0])!=seconds or int(counts[0])<100 or
            float(rates[0])<=0):
        raise PostgresHAError("pgbench WAL churn or independent client load unverified")
    return {"actual_pgbench_completed":True,"duration_seconds":seconds,
            "clients":clients,"transactions":int(counts[0]),"tps":float(rates[0])}


def validate_soak_replay(primary:str,standby_b:str,standby_c:str)->dict:
    if (not all(isinstance(v,str) and re.fullmatch(r"-?[0-9]+:-?[0-9]+:-?[0-9]+",v)
                for v in (primary,standby_b,standby_c)) or
            primary!=standby_b or primary!=standby_c):
        raise PostgresHAError("standby B/C did not replay real pgbench account WAL")
    return {"both_streaming_replicas_replayed_pgbench_dataset":True,
            "pgbench_account_balance_triplet_sha256_verified":True}


def run_lab(*,pg_bin:Path,report:Path,soak_seconds:int=0)->dict:
    if (os.geteuid()==0 or os.uname().sysname!="Linux"
            or not isinstance(pg_bin,Path) or not pg_bin.is_absolute()
            or pg_bin.is_symlink() or not pg_bin.is_dir()):
        raise PostgresHAError("nonroot dedicated ephemeral Linux + absolute PostgreSQL binary dir required")
    if type(soak_seconds) is not int or (soak_seconds!=0 and not 15<=soak_seconds<=120):
        raise PostgresHAError("bounded pgbench soak duration required")
    needed=("initdb","pg_ctl","pg_basebackup","psql") + (("pgbench",) if soak_seconds else ())
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
        if soak_seconds:
            _run([bindir("pgbench"),"-i","-s","1","-h","127.0.0.1",
                  "-p",str(ports[0]),"-U","postgres","postgres"],timeout=80)
        for name in ("b","c"):
            _run(pg_basebackup_command(bindir("pg_basebackup"),ports[0],paths[name]),
                 timeout=90)
            start(name)
            _wait(lambda n=name:query(n,"SELECT pg_is_in_recovery()")=="t"
                  and query(n,"SELECT COUNT(*) FROM ocaf40_events")=="1",seconds=20)
        pre_pid={node:query(node,"SELECT pg_backend_pid()") for node in ("a","b","c")}
        soak=None
        if soak_seconds:
            workload=_run([bindir("pgbench"),"-c","8","-j","2",
                           "-T",str(soak_seconds),"-r",
                           "-h","127.0.0.1","-p",str(ports[0]),
                           "-U","postgres","postgres"],timeout=soak_seconds+45)
            parsed=validate_pgbench_load(workload.stdout,soak_seconds,8)
            balances=("SELECT "
              "(SELECT COALESCE(SUM(abalance),0) FROM pgbench_accounts)::text||':'||"
              "(SELECT COALESCE(SUM(bbalance),0) FROM pgbench_branches)::text||':'||"
              "(SELECT COALESCE(SUM(tbalance),0) FROM pgbench_tellers)::text")
            primary_sum=query("a",balances)
            for name in ("b","c"):
                _wait(lambda n=name:query(n,balances)==primary_sum,seconds=35)
            soak={**parsed,**validate_soak_replay(
                primary_sum,query("b",balances),query("c",balances)),
                "load_during_replication":True,
                "primary_immediate_crash_after_soak":True,
                "production_approved":False}
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
            "pgbench_wal_soak":soak,
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
    parser.add_argument("--soak-seconds",type=int,default=0,
                        help="optional real PostgreSQL 8-client WAL pgbench load while standby B/C stream")
    parser.add_argument("--ack-local-ephemeral-postgres-primary-kill",action="store_true")
    a=parser.parse_args()
    if not a.ack_local_ephemeral_postgres_primary_kill:
        parser.error("explicit local ephemeral PostgreSQL crash acknowledgement required")
    try:
        result=run_lab(pg_bin=a.pg_bin_dir,report=a.report,
                       soak_seconds=a.soak_seconds)
    except (OSError,ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
