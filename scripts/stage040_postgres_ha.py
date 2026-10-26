#!/usr/bin/env python3
"""Stage 0.40 — real PostgreSQL 16 1-primary/2-standby WAL failover LAB.

This operates on THREE independent PostgreSQL POSTMASTER PROCESSES running
on ONE ephemeral Linux kernel, NOT three physical hosts or a production
consensus cluster. It tests actual streaming WAL replay, an IMMEDIATE
primary server crash, standby promotion and standby reparent/replay.
The old primary remains STOPPED in the default lab. The optional witness lab
rewinds it with PostgreSQL pg_rewind and a physical replication slot after a
second failover, then verifies streamed catch-up from the new primary C.
Neither mode provides external/server-side fencing from future raw writes.
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
    witness=r.get("witness_lab")
    if witness is not None and (not isinstance(witness,dict)
            or witness.get("authority")!="single_local_sqlite_witness"
            or witness.get("majority_consensus_tested") is not False
            or witness.get("former_a_rejoined_by_pg_rewind") is not True
            or witness.get("physical_replication_slot_active_on_c") is not True
            or witness.get("former_a_wal_receiver_status")!="streaming"
            or witness.get("final_committed_event_ids")!="1,2,3,4,5"
            or witness.get("database_server_side_fencing_tested") is not False):
        raise PostgresHAError("incomplete local rewind or falsely advertised witness scope")
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
    if (not all(isinstance(v,str) and re.fullmatch(r"[a-f0-9]{64}",v)
                for v in (primary,standby_b,standby_c)) or
            primary!=standby_b or primary!=standby_c):
        raise PostgresHAError("standby B/C did not replay real pgbench account WAL")
    return {"both_streaming_replicas_replayed_pgbench_dataset":True,
            "pgbench_full_ordered_dataset_sha256_verified":True,
            "pgbench_dataset_sha256":primary}


def pgbench_dataset_digest_sql()->str:
    """Hash all ordered rows, not balance sums that can hide divergent rows.

    PostgreSQL computes each table digest within the same statement snapshot.
    Include history: matching account totals alone cannot show every committed
    pgbench transaction was replicated.
    """
    tables=(
        ("pgbench_accounts", "aid"),
        ("pgbench_branches", "bid"),
        ("pgbench_tellers", "tid"),
        ("pgbench_history", "tid,bid,aid,delta,mtime,filler"),
    )
    digests=[
        "(SELECT encode(sha256(convert_to(COALESCE("
        f"string_agg(row_to_json(t)::text,E'\\n' ORDER BY {order}),''),"
        f"'UTF8')),'hex') FROM {table} t)"
        for table,order in tables
    ]
    return "SELECT encode(sha256(convert_to("+"||':'||".join(digests)+",'UTF8')),'hex')"


def stop_postmasters(pg_ctl:str,paths:dict,started:set)->None:
    """Attempt every dedicated postmaster even if one stop command fails.

    PostgreSQL pg_ctl status returns 3 only when the data directory has no
    running postmaster. Other exit codes, including unreadable PID state, do
    not constitute cleanup proof.
    """
    failed=[]
    for node in ("c","b","a"):
        if node not in started:
            continue
        try:
            _run([pg_ctl,"-D",str(paths[node]),"-m","immediate","-w","stop"],
                 check=False,timeout=15)
            status=_run([pg_ctl,"-D",str(paths[node]),"status"],check=False,timeout=5)
            if status.returncode!=3:
                failed.append(node)
        except PostgresHAError:
            failed.append(node)
    if failed:
        raise PostgresHAError("postmaster cleanup unverified: "+",".join(failed))


def run_lab(*,pg_bin:Path,report:Path,soak_seconds:int=0,
            exercise_witness:bool=False)->dict:
    if (os.geteuid()==0 or os.uname().sysname!="Linux"
            or not isinstance(pg_bin,Path) or not pg_bin.is_absolute()
            or pg_bin.is_symlink() or not pg_bin.is_dir()):
        raise PostgresHAError("nonroot dedicated ephemeral Linux + absolute PostgreSQL binary dir required")
    if type(soak_seconds) is not int or (soak_seconds!=0 and not 15<=soak_seconds<=120):
        raise PostgresHAError("bounded pgbench soak duration required")
    if type(exercise_witness) is not bool:
        raise PostgresHAError("explicit witness exercise selection required")
    if (not isinstance(report,Path) or not report.is_absolute()
            or ".." in report.parts or report.exists() or report.is_symlink()):
        raise PostgresHAError("new absolute evidence destination required")
    needed=("initdb","pg_ctl","pg_basebackup","psql") + (("pgbench",) if soak_seconds else ()) + (("pg_rewind",) if exercise_witness else ())
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
    witness=None
    leases={}
    witness_observation=None
    try:
        if exercise_witness:
            from stage040_quorum import QuorumGate, QuorumError
            witness=QuorumGate(state/"witness.sqlite")
            leases["a"]=witness.grant("node-a",ttl_seconds=300)
            witness.authorize_promotion("node-a",leases["a"].epoch)
        def pgctl(node:str,*args,timeout=25):
            return _run([bindir("pg_ctl"),"-D",str(paths[node]),*args],timeout=timeout)
        def start(node:str):
            index={"a":0,"b":1,"c":2}[node]
            # A pg_ctl timeout may occur after the postmaster was spawned.
            # Always attempt cleanup of this dedicated data directory.
            started.add(node)
            pgctl(node,"-o",f"-p {ports[index]} -h 127.0.0.1 -k {state}",
                  "-l",str(state/(node+".log")),"-w","start",timeout=25)
        def query(node:str,sql:str,check=True)->str:
            port=ports[{"a":0,"b":1,"c":2}[node]]
            res=_run([bindir("psql"),"-X","-q","-A","-t",
                 "-v","ON_ERROR_STOP=1","-h","127.0.0.1","-p",str(port),
                 "-U","postgres","-d","postgres","-c",sql],
                check=check,timeout=5)
            if not check:return str(res.returncode)
            return res.stdout.strip()
        def write(node:str,sql:str)->str:
            if witness is not None:
                witness.authorize_effect("node-"+node,leases[node].epoch)
            return query(node,sql)
        def renew_source_lease()->None:
            if witness is not None:
                leases["a"]=witness.renew("node-a",leases["a"].epoch,ttl_seconds=300)
        _run([bindir("initdb"),"-D",str(paths["a"]),
              "--auth-local=trust","--auth-host=trust","--data-checksums",
              "--no-instructions"],timeout=45)
        conf=paths["a"]/"postgresql.conf"
        conf.write_text(conf.read_text()+"\nwal_level = replica\nmax_wal_senders = 8\n"
                        "max_replication_slots = 8\nhot_standby = on\n"
                        "synchronous_commit = on\nfsync = on\n")
        start("a")
        if query("a","SELECT pg_is_in_recovery()")!="f":
            raise PostgresHAError("original primary reports recovery mode")
        write("a","CREATE TABLE ocaf40_events (id integer PRIMARY KEY, payload text NOT NULL)")
        write("a","INSERT INTO ocaf40_events VALUES (1,'pre-standby')")
        if soak_seconds:
            renew_source_lease()
            _run([bindir("pgbench"),"-i","-s","1","-h","127.0.0.1",
                  "-p",str(ports[0]),"-U","postgres","postgres"],timeout=80)
        for name in ("b","c"):
            renew_source_lease()
            _run(pg_basebackup_command(bindir("pg_basebackup"),ports[0],paths[name]),
                 timeout=90)
            auto=paths[name]/"postgresql.auto.conf"
            auto.write_text("\n".join(line for line in auto.read_text().splitlines()
                if not line.lstrip().startswith("primary_conninfo"))+"\n"+
                f"primary_conninfo = 'host=127.0.0.1 port={ports[0]} user=postgres application_name=ocaf40_{name}'\n")
            start(name)
            _wait(lambda n=name:query(n,"SELECT pg_is_in_recovery()")=="t"
                  and query(n,"SELECT COUNT(*) FROM ocaf40_events")=="1",seconds=20)
        pre_pid={node:query(node,"SELECT pg_backend_pid()") for node in ("a","b","c")}
        soak=None
        if soak_seconds:
            renew_source_lease()
            workload=_run([bindir("pgbench"),"-c","8","-j","2",
                           "-T",str(soak_seconds),"-r",
                           "-h","127.0.0.1","-p",str(ports[0]),
                           "-U","postgres","postgres"],timeout=soak_seconds+45)
            parsed=validate_pgbench_load(workload.stdout,soak_seconds,8)
            balances=pgbench_dataset_digest_sql()
            primary_sum=query("a",balances)
            for name in ("b","c"):
                _wait(lambda n=name:query(n,balances)==primary_sum,seconds=35)
            soak={**parsed,**validate_soak_replay(
                primary_sum,query("b",balances),query("c",balances)),
                "load_during_replication":True,
                "primary_immediate_crash_after_soak":True,
                "production_approved":False}
            # A pair of offsetting corruptions leaves SUM(abalance) unchanged.
            # Exercise that real SQL case inside a transaction, then rollback.
            account_sum=query("a","SELECT SUM(abalance) FROM pgbench_accounts")
            changed=query("a","BEGIN; UPDATE pgbench_accounts SET abalance=abalance+"
                          "CASE aid WHEN 1 THEN 1 ELSE -1 END WHERE aid IN (1,2); "
                          "SELECT SUM(abalance) FROM pgbench_accounts; "+balances+"; ROLLBACK")
            samples=changed.splitlines()
            if len(samples)!=2 or samples[0]!=account_sum or samples[1]==primary_sum:
                raise PostgresHAError("dataset digest failed to detect equal-sum row corruption")
            if query("a",balances)!=primary_sum:
                raise PostgresHAError("negative digest test did not rollback exactly")
            soak["equal_sum_row_corruption_detected_and_rolled_back"]=True
        sync_replication=None
        if exercise_witness:
            query("a","ALTER SYSTEM SET synchronous_standby_names = 'ANY 1 (ocaf40_b,ocaf40_c)'")
            query("a","SELECT pg_reload_conf()")
            _wait(lambda:query("a","SELECT COUNT(*) FROM pg_stat_replication WHERE sync_state='quorum'")=="2")
            write("a","SET synchronous_commit='remote_apply'; INSERT INTO ocaf40_events VALUES (2,'pre-crash')")
            sync_replication={
                "source_write_synchronous_commit":"remote_apply",
                "required_standbys":"ANY 1 (ocaf40_b,ocaf40_c)",
                "observed_quorum_standbys":query("a","SELECT string_agg(application_name,',' ORDER BY application_name) FROM pg_stat_replication WHERE sync_state='quorum'"),
                "scope":"one_pre_crash_transaction_on_primary_a",
                "synchronous_mode_during_b_c_failover":False,
            }
        else:
            write("a","INSERT INTO ocaf40_events VALUES (2,'pre-crash')")
        source_lsn=query("a","SELECT pg_current_wal_lsn()")
        for n in ("b","c"):
            _wait(lambda n=n:query(n,"SELECT COUNT(*) FROM ocaf40_events")=="2"
                  and query(n,f"SELECT pg_last_wal_replay_lsn() >= '{source_lsn}'::pg_lsn")=="t",
                  seconds=20)
        pre_crash_replay={n:query(n,"SELECT pg_last_wal_replay_lsn()") for n in ("b","c")}
        # A crash is real: request PostgreSQL's IMMEDIATE termination; no
        # graceful checkpoint. This is NOT a host power-off experiment.
        pgctl("a","-m","immediate","-w","stop")
        started.discard("a")
        failed_immediate=True
        if query("a","SELECT 1",check=False)=="0":
            raise PostgresHAError("former primary still accepting TCP after crash")
        if witness is not None:
            witness.release("node-a",leases["a"].epoch)
            leases["b"]=witness.grant("node-b",ttl_seconds=300)
            witness.authorize_promotion("node-b",leases["b"].epoch)
            try:
                write("a","INSERT INTO ocaf40_events VALUES (999,'stale')")
            except QuorumError:
                pass
            else:
                raise PostgresHAError("witness accepted old primary write")
        pgctl("b","-w","promote")
        _wait(lambda:query("b","SELECT pg_is_in_recovery()")=="f")
        write("b","INSERT INTO ocaf40_events VALUES (3,'post-promotion')")
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
        version=query("b","SHOW server_version")
        if witness is not None:
            # Show a second REAL PostgreSQL crash, then rewind former A to C's
            # promoted timeline. A slot retains C's WAL while A rejoins.
            try:
                witness.grant("node-c",ttl_seconds=300)
            except QuorumError:
                pass
            else:
                raise PostgresHAError("witness granted overlapping primary leases")
            pgctl("b","-m","immediate","-w","stop")
            started.discard("b")
            if query("b","SELECT 1",check=False)=="0":
                raise PostgresHAError("second crashed primary still accepting TCP")
            witness.release("node-b",leases["b"].epoch)
            leases["c"]=witness.grant("node-c",ttl_seconds=300)
            witness.close()
            witness=QuorumGate(state/"witness.sqlite")
            witness.authorize_promotion("node-c",leases["c"].epoch)
            pgctl("c","-w","promote")
            _wait(lambda:query("c","SELECT pg_is_in_recovery()")=="f")
            write("c","INSERT INTO ocaf40_events VALUES (4,'second-promotion')")
            try:
                write("b","INSERT INTO ocaf40_events VALUES (998,'stale-b')")
            except QuorumError:
                pass
            else:
                raise PostgresHAError("witness accepted second old-primary write")
            slot="ocaf40_rejoin_a"
            if write("c",f"SELECT slot_name FROM pg_create_physical_replication_slot('{slot}', true)")!=slot:
                raise PostgresHAError("physical WAL retention slot not created on promoted C")
            write("c","INSERT INTO ocaf40_events VALUES (5,'before-rewind-rejoin')")
            source_connection=f"host=127.0.0.1 port={ports[2]} user=postgres dbname=postgres"
            _run([bindir("pg_rewind"),"--target-pgdata",str(paths["a"]),
                  "--source-server",source_connection,"--write-recovery-conf"],timeout=90)
            auto=paths["a"]/"postgresql.auto.conf"
            auto.write_text("\n".join(line for line in auto.read_text().splitlines()
                                  if not line.lstrip().startswith("primary_slot_name"))+
                            f"\nprimary_slot_name = '{slot}'\n")
            start("a")
            _wait(lambda:query("a","SELECT pg_is_in_recovery()")=="t"
                  and query("a","SELECT COUNT(*) FROM ocaf40_events")=="5",seconds=35)
            if query("a","SELECT status FROM pg_stat_wal_receiver LIMIT 1")!="streaming":
                raise PostgresHAError("former A did not rejoin actual WAL streaming from C")
            if query("c",f"SELECT active FROM pg_replication_slots WHERE slot_name='{slot}'")!="t":
                raise PostgresHAError("former A did not consume its physical WAL slot")
            witness_observation={
                "authority":"single_local_sqlite_witness",
                "majority_consensus_tested":False,
                "witness_connection_reopened_with_live_lease":True,
                "epochs":[leases[n].epoch for n in ("a","b","c")],
                "overlapping_primary_grant_denied":True,
                "old_a_and_b_harness_writes_denied":True,
                "postgres_b_immediate_crash_then_c_promoted":True,
                "former_a_rejoined_by_pg_rewind":True,
                "physical_replication_slot_active_on_c":True,
                "former_a_wal_receiver_status":"streaming",
                "final_committed_event_ids":query("c","SELECT string_agg(id::text,',' ORDER BY id) FROM ocaf40_events"),
                "harness_write_guard_only":True,
                "database_server_side_fencing_tested":False,
                "physical_host_fault_tested":False,
            }
            if witness_observation["final_committed_event_ids"]!="1,2,3,4,5":
                raise PostgresHAError("second failover lost or duplicated committed events")
        result={
            "schema_version":1,
            "status":"POSTGRES_3_PROCESSES_STREAMING_FAILOVER_PASSED",
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "nodes_count":3,
            "pgbench_wal_soak":soak,
            "postgres_version":version,
            "witness_lab":witness_observation,
            "synchronous_replication":sync_replication,
            "pre_crash_source_wal_lsn":source_lsn,
            "pre_crash_standby_replay_lsns":pre_crash_replay,
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
    finally:
        try:
            stop_postmasters(bindir("pg_ctl"),paths,started)
        finally:
            if witness is not None:
                witness.close()
        shutil.rmtree(state,ignore_errors=True)
    # A successful receipt is published only AFTER cleanup was observed.
    result["cleanup"]={"all_started_postmasters_observed_stopped":True,
                       "private_lab_state_removed":not state.exists()}
    if state.exists():
        raise PostgresHAError("private lab state cleanup failed")
    report.parent.mkdir(parents=True,exist_ok=True)
    fd=os.open(report,os.O_WRONLY|os.O_CREAT|os.O_EXCL|getattr(os,"O_NOFOLLOW",0),0o600)
    with os.fdopen(fd,"w") as sink:
        json.dump(result,sink,indent=2,sort_keys=True)
        sink.write("\n")
        sink.flush()
        os.fsync(sink.fileno())
    return result


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pg-bin-dir",type=Path,required=True)
    parser.add_argument("--report",type=Path,required=True)
    parser.add_argument("--soak-seconds",type=int,default=0,
                        help="optional real PostgreSQL 8-client WAL pgbench load while standby B/C stream")
    parser.add_argument("--exercise-witness",action="store_true",
                        help="exercise durable SINGLE witness, B-to-C failover and former A rejoin; one host only")
    parser.add_argument("--ack-local-ephemeral-postgres-primary-kill",action="store_true")
    a=parser.parse_args()
    if not a.ack_local_ephemeral_postgres_primary_kill:
        parser.error("explicit local ephemeral PostgreSQL crash acknowledgement required")
    try:
        result=run_lab(pg_bin=a.pg_bin_dir,report=a.report,
                       soak_seconds=a.soak_seconds,exercise_witness=a.exercise_witness)
    except (OSError,ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
