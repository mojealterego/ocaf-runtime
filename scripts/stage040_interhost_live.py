#!/usr/bin/env python3
"""OCAF 0.40.3: LIVE PostgreSQL TCP WAL streaming on 3 independent SSH hosts.

Runs ONLY after the operator has provided three truly separate, reachable
Linux hosts with PostgreSQL primary A + replicas B/C already provisioned.
Requires pinned SSH host public keys, a dedicated *disposable*
ocaf_stage040_lab database and peer permissions for local PostgreSQL.
Issues a fresh SQL nonce on A and waits for B/C to replicate that exact
row through LIVE PG WAL, not an artifact handoff or namespaces.
It does not promote or fence and therefore cannot certify HA/release.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import shlex
import stat
import subprocess
import time

class InterhostPgError(ValueError):
    pass

BOOT=re.compile(r"[a-f0-9]{8}-[a-f0-9]{4}-[1-8][a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}\Z")
NONCE=re.compile(r"[a-f0-9]{32}\Z")
LSN=re.compile(r"[0-9A-F]{1,8}/[0-9A-F]{1,8}\Z")
SSH_USER=re.compile(r"[a-z_][a-z0-9_-]{1,31}\Z")
ROLES=("a","b","c")

def safe_nonce(value:str)->str:
    if not isinstance(value,str) or NONCE.fullmatch(value) is None:
        raise InterhostPgError("unsafe or reused ephemeral SQL challenge nonce")
    return value

def lsn_position(value:str)->int:
    """Parse the two unsigned 32-bit halves of a PostgreSQL WAL position."""
    if not isinstance(value,str) or LSN.fullmatch(value) is None:
        raise InterhostPgError("invalid bounded PostgreSQL WAL LSN")
    high,low=value.split("/")
    return (int(high,16)<<32)|int(low,16)

def _ipv4(value:str)->str:
    if not isinstance(value,str):
        raise InterhostPgError("literal IPv4 peer required")
    try:
        addr=ipaddress.IPv4Address(value)
    except ipaddress.AddressValueError as exc:
        raise InterhostPgError("invalid peer address") from exc
    if str(addr)!=value or addr.is_loopback or addr.is_link_local or addr.is_multicast or addr.is_unspecified:
        raise InterhostPgError("non-forwardable host IPv4 address")
    return value

def validate_host(node:dict)->dict:
    if not isinstance(node,dict) or set(node)!={"role","host","ssh_user","ssh_port","replication_address"}:
        raise InterhostPgError("strict remote PostgreSQL SSH inventory row required")
    if (node.get("role") not in ROLES
        or not isinstance(node.get("ssh_user"),str)
        or SSH_USER.fullmatch(node["ssh_user"]) is None
        or type(node.get("ssh_port")) is not int
        or not 1<=node["ssh_port"]<=65535):
        raise InterhostPgError("illegal SSH user/port/role")
    _ipv4(node["host"])
    _ipv4(node["replication_address"])
    return dict(node)

def validate_inventory(config:dict,known_hosts:Path)->dict:
    if (not isinstance(config,dict)
        or set(config)!={"schema_version","nodes","database","postgres_port","postgres_socket_dir"}
        or type(config["schema_version"]) is not int or config["schema_version"]!=1
        or config["database"]!="ocaf_stage040_lab"
        or type(config["postgres_port"]) is not int or not 1024<=config["postgres_port"]<=65535
        or not isinstance(config["postgres_socket_dir"],str)
        or config["postgres_socket_dir"] not in ("/var/run/postgresql","/tmp")
        or not isinstance(config["nodes"],dict) or set(config["nodes"])!=set(ROLES)):
        raise InterhostPgError("disposable three-host PostgreSQL lab configuration required")
    hosts={k:validate_host(v) for k,v in config["nodes"].items()}
    if any(hosts[r]["role"]!=r for r in ROLES):
        raise InterhostPgError("host role mismatch")
    for attribute in ("host","replication_address"):
        if len({hosts[r][attribute] for r in ROLES})!=3:
            raise InterhostPgError("three distinct real network endpoints required")
    if not isinstance(known_hosts,Path) or not known_hosts.is_absolute() or known_hosts.is_symlink():
        raise InterhostPgError("separate SSH known_hosts pin required")
    try:
        s=known_hosts.stat()
        if (not stat.S_ISREG(s.st_mode) or s.st_uid!=os.geteuid()
            or s.st_mode & 0o077 or not 60<s.st_size<=131072):
            raise InterhostPgError("private nonempty pinned SSH known_hosts file required")
        pinned=known_hosts.read_text(encoding="utf-8")
    except (OSError,UnicodeError) as exc:
        raise InterhostPgError("SSH key pin file not readable") from exc
    # Check that all three destinations have an explicit static pin.
    for node in hosts.values():
        host=node["host"]
        label=host if node["ssh_port"]==22 else f"[{host}]:{node['ssh_port']}"
        if not any(row.strip().startswith(label+" ") for row in pinned.splitlines()):
            raise InterhostPgError("SSH host public-key pin missing: "+label)
    return hosts

def validate_receipt(report:dict)->dict:
    if (not isinstance(report,dict)
        or type(report.get("schema_version")) is not int or report["schema_version"]!=1
        or report.get("status")!="REAL_DISTINCT_LINUX_KERNELS_POSTGRESQL_LIVE_TCP_STREAMING_VERIFIED"
        or report.get("topology")!="three_truly_distinct_linux_kernels"
        or not isinstance(report.get("boot_ids"),dict)
        or set(report["boot_ids"])!=set(ROLES)
        or any(not isinstance(x,str) or BOOT.fullmatch(x) is None for x in report["boot_ids"].values())
        or len(set(report["boot_ids"].values()))!=3
        or not isinstance(report.get("standby_addresses"),dict)
        or set(report["standby_addresses"])!=set(("b","c"))
        or not isinstance(report.get("client_addresses"),list)
        or report["client_addresses"] != sorted(report["standby_addresses"].values())
        or len(set(report["client_addresses"]))!=2
        or any(_ipv4(x) != x for x in report["client_addresses"])
        or report.get("source_writable") is not True
        or report.get("b_in_recovery") is not True or report.get("c_in_recovery") is not True
        or report.get("replication_b_status")!="streaming"
        or report.get("replication_c_status")!="streaming"
        or report.get("used_ssh_host_key_pinning") is not True
        or report.get("actual_direct_network_replication") is not True
        or report.get("physical_host_power_off_tested") is not False
        or report.get("consensus_fencing_verified") is not False
        or report.get("production_approved") is not False
        or report.get("stage_040_complete") is not False
        or not isinstance(report.get("source_wal_lsn"),str)
        or LSN.fullmatch(report["source_wal_lsn"]) is None):
        raise InterhostPgError("missing factual three-independent-kernel PG streaming report")
    for role in ("source_nonce","standby_b_nonce","standby_c_nonce"):
        safe_nonce(report.get(role))
    if not (report["source_nonce"]==report["standby_b_nonce"]==report["standby_c_nonce"]):
        raise InterhostPgError("PG standby SQL challenge was not actually replayed")
    for key in ("standby_b_wal_replay_at_or_above_source_lsn",
                "standby_c_wal_replay_at_or_above_source_lsn"):
        if report.get(key) is not True:
            raise InterhostPgError("WAL replay LSN did not catch up to primary")
    source_position=lsn_position(report["source_wal_lsn"])
    if source_position==0:
        raise InterhostPgError("source WAL position cannot be zero after challenge commit")
    for role in ("b","c"):
        replay_key=f"standby_{role}_wal_replay_lsn"
        if replay_key in report and lsn_position(report[replay_key])<source_position:
            raise InterhostPgError("observed standby WAL LSN precedes source challenge")
    if "boot_ids_after" in report and report["boot_ids_after"]!=report["boot_ids"]:
        raise InterhostPgError("host kernel changed during observation")
    if "client_addresses_after" in report and report["client_addresses_after"]!=report["client_addresses"]:
        raise InterhostPgError("streaming client topology changed during observation")
    if "receiver_snapshots" in report:
        snapshots=report["receiver_snapshots"]
        if (not isinstance(snapshots,dict) or set(snapshots)!={"before","after"}
            or type(report.get("postgres_port")) is not int
            or not 1024<=report["postgres_port"]<=65535):
            raise InterhostPgError("invalid observed receiver snapshots")
        primary=_ipv4(report.get("source_replication_address"))
        for phase in ("before","after"):
            if not isinstance(snapshots[phase],dict) or set(snapshots[phase])!={"b","c"}:
                raise InterhostPgError("missing standby receiver observation")
            for receiver in snapshots[phase].values():
                validate_receiver(receiver,primary,report["postgres_port"])
    return report

def validate_receiver(receiver:dict,primary:str,port:int)->dict:
    """Bind a real receiver to the configured primary, preserving raw LSNs."""
    if (not isinstance(receiver,dict) or receiver.get("status")!="streaming"
        or receiver.get("sender_host")!=primary
        or type(receiver.get("sender_port")) is not int
        or receiver["sender_port"]!=port):
        raise InterhostPgError("standby receiver is not streaming from configured primary")
    for key in ("written_lsn","flushed_lsn","received_lsn","latest_end_lsn"):
        if receiver.get(key) is not None:
            lsn_position(receiver[key])
    # Avoid persisting conninfo or other unrelated potentially sensitive columns.
    return {key:receiver[key] for key in (
        "status","sender_host","sender_port","written_lsn","flushed_lsn",
        "received_lsn","latest_end_lsn") if key in receiver}

def _ssh(node:dict,known_hosts:Path,argv:list[str],*,timeout:int=15)->str:
    if not isinstance(argv,list) or not argv or any(not isinstance(x,str) for x in argv):
        raise InterhostPgError("invalid SSH remote command")
    cmd=["ssh","-T","-o","BatchMode=yes","-o","StrictHostKeyChecking=yes",
         "-o","IdentitiesOnly=yes","-o",f"UserKnownHostsFile={known_hosts}",
         "-o","ConnectTimeout=7","-p",str(node["ssh_port"]),
         "--",node["ssh_user"]+"@"+node["host"],shlex.join(argv)]
    try:
        result=subprocess.run(cmd,text=True,capture_output=True,timeout=timeout,check=False)
    except (OSError,subprocess.TimeoutExpired) as exc:
        raise InterhostPgError("real independent SSH host unavailable") from exc
    if result.returncode or len(result.stdout)>8192:
        raise InterhostPgError("remote kernel/psql probe failed on "+node["role"]+
                               " (code "+str(result.returncode)+")")
    return result.stdout.strip()

def run_lab(config:dict,known_hosts:Path,report:Path,*,ssh=None)->dict:
    hosts=validate_inventory(config,known_hosts)
    if not isinstance(report,Path) or not report.is_absolute() or report.exists() or report.is_symlink() or not report.parent.is_dir():
        raise InterhostPgError("private new evidence destination required")
    probe=_ssh if ssh is None else ssh
    def boot_ids()->dict:
        return {r:probe(hosts[r],known_hosts,["cat","/proc/sys/kernel/random/boot_id"])
                for r in ROLES}
    boot=boot_ids()
    if len(set(boot.values()))!=3 or any(BOOT.fullmatch(x) is None for x in boot.values()):
        raise InterhostPgError("actual three different Linux host kernels NOT observed")
    port=config["postgres_port"]
    sock=config["postgres_socket_dir"]
    def sql(role:str,expression:str)->str:
        return probe(hosts[role],known_hosts,[
            "sudo","-n","-u","postgres","--","psql",
            "-X","-A","-t","-q","-v","ON_ERROR_STOP=1",
            "-h",sock,"-p",str(port),"-U","postgres",
            "-d","ocaf_stage040_lab","-c",expression],timeout=20)
    def check_roles()->None:
        if sql("a","SELECT pg_is_in_recovery()")!="f":
            raise InterhostPgError("expected actual remote primary A is read-only")
        for role in ("b","c"):
            if sql(role,"SELECT pg_is_in_recovery()")!="t":
                raise InterhostPgError(role+" is not a remote WAL-replay standby")
    addresses={role:hosts[role]["replication_address"] for role in ("b","c")}
    def clients_snapshot()->list[str]:
        clients=sorted(sql("a","SELECT client_addr::text FROM pg_stat_replication "+
                              "WHERE state='streaming' ORDER BY client_addr").splitlines())
        if clients!=sorted(addresses.values()):
            raise InterhostPgError("actual source does not see two remote streaming TCP peers")
        return clients
    def receivers_snapshot()->dict:
        receivers={}
        for role in ("b","c"):
            try:
                # row_to_json also captures version-specific observed LSN columns.
                receiver=json.loads(sql(role,
                    "SELECT row_to_json(receiver)::text FROM pg_stat_wal_receiver AS receiver"))
            except (TypeError,ValueError) as exc:
                raise InterhostPgError("expected exactly one observed WAL receiver") from exc
            receivers[role]=validate_receiver(receiver,hosts["a"]["replication_address"],port)
        return receivers
    check_roles()
    clients=clients_snapshot()
    receivers_before=receivers_snapshot()
    sql("a","CREATE TABLE IF NOT EXISTS ocaf40_live_probe("
            "nonce TEXT PRIMARY KEY,origin_boot_id TEXT NOT NULL,"
            "inserted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp())")
    nonce=secrets.token_hex(16)
    safe_nonce(nonce)
    sql("a",f"INSERT INTO ocaf40_live_probe(nonce,origin_boot_id) VALUES ('{nonce}','{boot['a']}')")
    lsn=sql("a","SELECT pg_current_wal_lsn()")
    source_position=lsn_position(lsn)
    if not source_position:
        raise InterhostPgError("real primary did not return source WAL LSN")
    start=time.monotonic()
    copied={}
    replay_lsn={}
    for role in ("b","c"):
        deadline=time.monotonic()+45
        while time.monotonic()<deadline:
            raw_lsn=sql(role,"SELECT pg_last_wal_replay_lsn()")
            if raw_lsn:
                replay_lsn[role]=raw_lsn
                # Wait for the DDL/INSERT commit before referring to the new table.
                # A missing table here is a real error, never an ignored SSH failure.
                if lsn_position(raw_lsn)>=source_position:
                    copied[role]=sql(role,
                        "SELECT nonce || ':' || origin_boot_id FROM ocaf40_live_probe "+
                        f"WHERE nonce='{nonce}'")
                    if copied[role]==nonce+":"+boot["a"]:
                        break
            time.sleep(.5)
        else:
            raise InterhostPgError("real "+role+" WAL did not replay new primary nonce and LSN")
    # No success receipt if a receiver stopped, a role changed or any host rebooted
    # while the challenge was being observed.
    check_roles()
    clients_after=clients_snapshot()
    receivers_after=receivers_snapshot()
    boot_after=boot_ids()
    if boot_after!=boot:
        raise InterhostPgError("host kernel changed during SQL challenge observation")
    output={
        "schema_version":1,
        "status":"REAL_DISTINCT_LINUX_KERNELS_POSTGRESQL_LIVE_TCP_STREAMING_VERIFIED",
        "observed_at_utc":datetime.now(timezone.utc).isoformat(),
        "topology":"three_truly_distinct_linux_kernels",
        "boot_ids":boot,"boot_ids_after":boot_after,
        "client_addresses":clients,"client_addresses_after":clients_after,
        "standby_addresses":addresses,
        "source_replication_address":hosts["a"]["replication_address"],
        "postgres_port":port,
        "receiver_snapshots":{"before":receivers_before,"after":receivers_after},
        "source_writable":True,"b_in_recovery":True,"c_in_recovery":True,
        "replication_b_status":receivers_after["b"]["status"],
        "replication_c_status":receivers_after["c"]["status"],
        "source_nonce":nonce,
        "standby_b_nonce":copied["b"].split(":",1)[0],
        "standby_c_nonce":copied["c"].split(":",1)[0],
        "source_wal_lsn":lsn,
        "standby_b_wal_replay_lsn":replay_lsn["b"],
        "standby_c_wal_replay_lsn":replay_lsn["c"],
        "standby_b_wal_replay_at_or_above_source_lsn":True,
        "standby_c_wal_replay_at_or_above_source_lsn":True,
        "observation_time_ms":int((time.monotonic()-start)*1000),
        "used_ssh_host_key_pinning":True,
        "actual_direct_network_replication":True,
        "physical_host_power_off_tested":False,
        "consensus_fencing_verified":False,
        "production_approved":False,
        "stage_040_complete":False,
    }
    validate_receipt(output)
    fd=os.open(report,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
    with os.fdopen(fd,"w",encoding="utf-8") as fp:
        json.dump(output,fp,sort_keys=True,indent=2)
        fp.write("\n")
    return output

def main()->int:
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--inventory",type=Path,required=True)
    p.add_argument("--known-hosts",type=Path,required=True)
    p.add_argument("--report",type=Path,required=True)
    p.add_argument("--ack-mutating-disposable-postgresql-interhost-lab",action="store_true")
    args=p.parse_args()
    if not args.ack_mutating_disposable_postgresql_interhost_lab:
        p.error("explicit acknowledgment of dedicated disposable 3-host SQL-write lab required")
    try:
        config=json.loads(args.inventory.read_text())
        result=run_lab(config,args.known_hosts,args.report)
    except (OSError,ValueError) as exc:
        p.error(str(exc))
    print(json.dumps({"status":result["status"],"production_approved":False}))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
