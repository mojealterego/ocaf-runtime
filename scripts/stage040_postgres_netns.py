#!/usr/bin/env python3
"""OCAF 0.40.6: actual Linux veth bridge TCP streaming PG WAL, partition, promotion.

THREE REAL PostgreSQL postmasters in THREE distinct Linux network
namespaces joined only by an isolated kernel veth bridge, no default
routes. B and C open actual PostgreSQL TCP to A, replay WAL. A Linux
blackhole route PARTITIONS B while C continues, then healing catches B
up. Original A is shut down IMMEDIATE; B promotes and C reconnects and
replays new timeline. This is one kernel and NOT inter-host HA.
"""
from __future__ import annotations

import argparse
from datetime import datetime,timezone
import json
import os
from pathlib import Path
import pwd
import re
import secrets
import shutil
import subprocess
import tempfile
import time

class NetnsPGError(ValueError):pass

TRUE_OBSERVATIONS=(
 "postgresql_real_processes","three_distinct_kernel_netns_inodes",
 "bridge_veth_real_kernel_network","two_distinct_remote_pg_stat_replication_client_ips",
 "initial_wal_replay_to_both","kernel_route_blackhole_b_to_a_installed",
 "partitioned_b_stale_while_c_updated","b_recovered_after_network_heal",
 "primary_a_immediate_crash","standby_b_promoted_writable",
 "standby_c_reparented_to_b_streaming","standby_c_replayed_after_b_promotion",
)
EXPECTED_IPS=("10.247.40.2","10.247.40.3")

def validate_client_ips(ips:tuple[str,str])->None:
    if (not isinstance(ips,tuple) or len(ips)!=2
            or ips!=EXPECTED_IPS or len(set(ips))!=2):
        raise NetnsPGError("WAL streaming must originate from real distinct private netns guest IPs")

def validate_report(r:dict)->dict:
    if (not isinstance(r,dict) or
        type(r.get("schema_version")) is not int or r["schema_version"]!=1
        or r.get("status")!="POSTGRES_THREE_NETNS_TCP_STREAM_PARTITION_RECOVERY_PASSED"
        or any(r.get(k) is not True for k in TRUE_OBSERVATIONS)
        or r.get("one_underlying_linux_kernel") is not True
        or r.get("inter_kernel_tcp_streaming_tested") is not False
        or r.get("physical_host_loss_tested") is not False
        or r.get("production_approved") is not False):
        raise NetnsPGError("missing kernel TCP streaming evidence or falsely advertised inter-host HA")
    return r

def run(cmd:list[str],*,timeout:int=35,check:bool=True)->subprocess.CompletedProcess:
    try:
        r=subprocess.run(cmd,capture_output=True,text=True,check=False,timeout=timeout)
    except (OSError,subprocess.TimeoutExpired) as exc:
        raise NetnsPGError("bounded real PG netns command unavailable: "+cmd[0]) from exc
    if check and r.returncode:
        raise NetnsPGError("real PostgreSQL netns command failed: "+" ".join(cmd[:4])+": "+r.stderr[-950:])
    return r

def wait_until(callable,*,seconds:int=25,why:str)->None:
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        try:
            if callable():return
        except (NetnsPGError,ValueError):
            pass
        time.sleep(.2)
    raise NetnsPGError("real network-streamed PostgreSQL timeout: "+why)

def run_lab(*,pg_bin:Path,report:Path)->dict:
    if (os.geteuid()!=0 or os.uname().sysname!="Linux"
        or not isinstance(pg_bin,Path) or not pg_bin.is_absolute()
        or pg_bin.is_symlink() or not pg_bin.is_dir()
        or not report.is_absolute() or report.exists() or report.is_symlink()
        or not report.parent.is_dir()):
        raise NetnsPGError("privileged disposable Linux network namespace experiment only")
    for name in ("initdb","pg_ctl","pg_basebackup","psql"):
        if not (pg_bin/name).is_file():
            raise NetnsPGError("real PostgreSQL 16 server package unavailable: "+name)
    user=pwd.getpwnam("postgres")
    uid,gid=user.pw_uid,user.pw_gid
    base=Path(tempfile.mkdtemp(prefix="ocaf40-pg-veth-",dir="/tmp"))
    os.chown(base,uid,gid)
    tag=secrets.token_hex(3)
    bridge="br40"+tag
    ns={k:"oc40"+k+tag for k in "abc"}
    root={k:base/k for k in "abc"}
    ip={k:f"10.247.40.{i}" for i,k in enumerate("abc",start=1)}
    port=25340
    namespaces=[]
    bridge_exists=False
    running=set()
    blackhole=False
    try:
        for name in "abc":
            root[name].mkdir(mode=0o700)
            os.chown(root[name],uid,gid)
            run(["ip","netns","add",ns[name]])
            namespaces.append(name)
        run(["ip","link","add",bridge,"type","bridge"])
        bridge_exists=True
        run(["ip","link","set",bridge,"up"])
        for name in "abc":
            host_dev="h40"+name+tag
            guest_dev="g40"+name+tag
            run(["ip","link","add",host_dev,"type","veth",
                 "peer","name",guest_dev])
            run(["ip","link","set",guest_dev,"netns",ns[name]])
            run(["ip","link","set",host_dev,"master",bridge])
            run(["ip","link","set",host_dev,"up"])
            run(["ip","-n",ns[name],"link","set","lo","up"])
            run(["ip","-n",ns[name],"addr","add",
                 ip[name]+"/24","dev",guest_dev])
            run(["ip","-n",ns[name],"link","set",guest_dev,"up"])
            if run(["ip","-n",ns[name],"route","show","default"]).stdout.strip():
                raise NetnsPGError("netns internet default route unexpectedly present")
        inode={name:int(run(["ip","netns","exec",ns[name],
                    "stat","-Lc","%i","/proc/self/ns/net"]).stdout.strip()) for name in "abc"}
        if len(set(inode.values()))!=3:
            raise NetnsPGError("three actual distinct Linux netns inodes not observed")
        def pgcmd(node,args,timeout=35,check=True):
            return run(["ip","netns","exec",ns[node],"runuser",
                       "-u","postgres","--",*args],timeout=timeout,check=check)
        exe=lambda x:str(pg_bin/x)
        def ctl(node,*args,timeout=35,check=True):
            return pgcmd(node,[exe("pg_ctl"),"-D",str(root[node]),*args],
                         timeout=timeout,check=check)
        def query(node,sql,*,check=True):
            cmd=[exe("psql"),"-X","-q","-A","-t",
                "-v","ON_ERROR_STOP=1","-h",ip[node],"-p",str(port),
                "-U","postgres","-d","postgres","-c",sql]
            r=pgcmd(node,cmd,timeout=6,check=check)
            return r.stdout.strip() if check else r.returncode
        def start(node):
            ctl(node,"-o",f"-p {port} -h {ip[node]}",
                "-l",str(base/(node+".log")),"-w","start",timeout=35)
            running.add(node)
        pgcmd("a",[exe("initdb"),"-D",str(root["a"]),
                  "--auth-local=trust","--auth-host=trust",
                  "--no-instructions"],timeout=60)
        conf=root["a"]/"postgresql.conf"
        conf.write_text(conf.read_text()+"\nwal_level = replica\n"
                 "max_wal_senders = 8\nhot_standby = on\n"
                 "wal_receiver_timeout = '3s'\n"
                 "fsync = on\nsynchronous_commit = on\n")
        hba=root["a"]/"pg_hba.conf"
        hba.write_text(hba.read_text()+
                 "\nhost replication postgres 10.247.40.0/24 trust\n"
                 "host all postgres 10.247.40.0/24 trust\n")
        start("a")
        if query("a","SELECT pg_is_in_recovery()")!="f":
            raise NetnsPGError("primary A is not writable")
        query("a","CREATE TABLE pg_netns_receipts(id int PRIMARY KEY,label text NOT NULL)")
        query("a","INSERT INTO pg_netns_receipts VALUES (1,'before-standbys')")
        for node in ("b","c"):
            pgcmd(node,[exe("pg_basebackup"),"--host",ip["a"],
                      "--port",str(port),"--username","postgres",
                      "--pgdata",str(root[node]),
                      "--format=plain","--wal-method=stream",
                      "--checkpoint=fast","--write-recovery-conf"],timeout=75)
            start(node)
            wait_until(lambda name=node:
                query(name,"SELECT pg_is_in_recovery()")=="t"
                and query(name,"SELECT count(*) FROM pg_netns_receipts")=="1",
                why="standby "+node+" did not replay first WAL")
        query("a","INSERT INTO pg_netns_receipts VALUES (2,'pre-partition')")
        for node in ("b","c"):
            wait_until(lambda name=node:query(name,
                "SELECT count(*) FROM pg_netns_receipts")=="2",
                why="initial WAL replay missing on "+node)
        clients=query("a","SELECT client_addr FROM pg_stat_replication ORDER BY client_addr")
        clients_tuple=tuple(clients.splitlines())
        validate_client_ips(clients_tuple)
        b_status=query("b","SELECT status FROM pg_stat_wal_receiver")
        c_status=query("c","SELECT status FROM pg_stat_wal_receiver")
        if b_status!="streaming" or c_status!="streaming":
            raise NetnsPGError("standby peers not actively streaming over kernel TCP")
        run(["ip","-n",ns["b"],"route","add","blackhole",ip["a"]+"/32"])
        blackhole=True
        if "blackhole" not in run(["ip","-n",ns["b"],"route","show",
                                   ip["a"]+"/32"]).stdout:
            raise NetnsPGError("actual kernel blackhole route did not install")
        query("a","INSERT INTO pg_netns_receipts VALUES (3,'during-b-partition')")
        wait_until(lambda:query("c","SELECT count(*) FROM pg_netns_receipts")=="3",
                   why="healthy C failed to replay during B partition")
        if query("b","SELECT count(*) FROM pg_netns_receipts")!="2":
            raise NetnsPGError("isolated B somehow received A's new WAL before heal")
        run(["ip","-n",ns["b"],"route","del","blackhole",ip["a"]+"/32"])
        blackhole=False
        wait_until(lambda:query("b","SELECT count(*) FROM pg_netns_receipts")=="3",
                   seconds=40,why="standby B failed to heal its real WAL TCP stream")
        ctl("a","-m","immediate","-w","stop")
        running.remove("a")
        if query("a","SELECT 1",check=False)==0:
            raise NetnsPGError("old primary still accepts SQL after IMMEDIATE stop")
        ctl("b","-w","promote")
        wait_until(lambda:query("b","SELECT pg_is_in_recovery()")=="f",
                   why="B was not promoted to primary")
        query("b","INSERT INTO pg_netns_receipts VALUES (4,'on-new-primary-b')")
        ctl("c","-m","fast","-w","stop")
        running.remove("c")
        auto=root["c"]/"postgresql.auto.conf"
        old=auto.read_text()
        clean="\n".join(s for s in old.splitlines()
                           if not s.lstrip().startswith("primary_conninfo"))+"\n"
        clean+=f"primary_conninfo = 'host={ip['b']} port={port} user=postgres application_name=ocaf40_netns_c'\n"
        auto.write_text(clean)
        start("c")
        wait_until(lambda:query("c","SELECT pg_is_in_recovery()")=="t"
              and query("c","SELECT count(*) FROM pg_netns_receipts")=="4",
              seconds=30,why="C failed to follow new B WAL timeline")
        b_clients=query("b","SELECT client_addr FROM pg_stat_replication")
        if b_clients!=ip["c"] or query("c","SELECT status FROM pg_stat_wal_receiver")!="streaming":
            raise NetnsPGError("C did not establish NEW real streaming PostgreSQL TCP to promoted B")
        # all observations are derived after real SQL and kernel netns network failures.
        evidence={
           "schema_version":1,
           "status":"POSTGRES_THREE_NETNS_TCP_STREAM_PARTITION_RECOVERY_PASSED",
           "observed_at_utc":datetime.now(timezone.utc).isoformat(),
           "kernel_boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
           "netns_inodes":inode,"pg_primary_ip":ip["a"],
           "standby_tcp_client_ips":list(clients_tuple),
           "standby_after_promotion_tcp_ip":b_clients,
           "pg_port_per_netns":port,
           "postgres_version":query("b","SHOW server_version"),
           "postgresql_real_processes":True,
           "three_distinct_kernel_netns_inodes":True,
           "bridge_veth_real_kernel_network":True,
           "two_distinct_remote_pg_stat_replication_client_ips":True,
           "initial_wal_replay_to_both":True,
           "kernel_route_blackhole_b_to_a_installed":True,
           "partitioned_b_stale_while_c_updated":True,
           "b_recovered_after_network_heal":True,
           "primary_a_immediate_crash":True,
           "standby_b_promoted_writable":True,
           "standby_c_reparented_to_b_streaming":True,
           "standby_c_replayed_after_b_promotion":True,
           "one_underlying_linux_kernel":True,
           "inter_kernel_tcp_streaming_tested":False,
           "physical_host_loss_tested":False,
           "production_approved":False,
        }
        validate_report(evidence)
        with report.open("x") as out:
            json.dump(evidence,out,sort_keys=True,indent=2)
            out.write("\n")
        return evidence
    finally:
        if blackhole:
            run(["ip","-n",ns["b"],"route","del","blackhole",ip["a"]+"/32"],check=False)
        for name in ("c","b","a"):
            if name in running:
                try:ctl(name,"-m","immediate","-w","stop",timeout=15,check=False)
                except (OSError,NetnsPGError):pass
        for name in reversed(namespaces):
            run(["ip","netns","delete",ns[name]],timeout=7,check=False)
        if bridge_exists:
            run(["ip","link","del",bridge],timeout=7,check=False)
        shutil.rmtree(base,ignore_errors=True)

def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pg-bin-dir",type=Path,required=True)
    parser.add_argument("--report",type=Path,required=True)
    parser.add_argument("--ack-privileged-isolated-netns-postgres-fault-lab",action="store_true")
    args=parser.parse_args()
    if not args.ack_privileged_isolated_netns_postgres_fault_lab:
        parser.error("explicit isolated privileged real kernel PG network partition experiment required")
    try:
        result=run_lab(pg_bin=args.pg_bin_dir,report=args.report)
    except (OSError,ValueError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
