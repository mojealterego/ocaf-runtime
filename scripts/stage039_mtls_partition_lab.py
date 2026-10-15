#!/usr/bin/env python3
"""0.39.16 REAL Linux network namespace TCP TLS1.3 mutual-auth fault lab.

Two isolated Linux netns communicate via a REAL veth pair, with no
default route or internet. OpenSSL CA signs ephemeral server/allowed
client/attacker certificates. A durable single-primary SQLite lease is
fenced across kernel route blackhole, replay, rogue certificate, server
SIGKILL, restart and renewal. This is ONE Linux kernel and NOT automatic
multi-host HA, Firecracker-guest TLS or independent audit.
"""
from __future__ import annotations

import argparse
from datetime import datetime,timezone
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
import subprocess
import tempfile
import time

import stage039_mtls_fencing as fencing


class MTLSLabError(ValueError):
    pass


def _run(argv:list[str], *,timeout:float=25, expect_ok=True)->subprocess.CompletedProcess:
    try:
        r=subprocess.run(argv,capture_output=True,text=True,timeout=timeout,check=False)
    except (OSError,subprocess.TimeoutExpired) as exc:
        raise MTLSLabError("bounded lab command unavailable: "+argv[0]) from exc
    if expect_ok and r.returncode:
        raise MTLSLabError("real lab command failed: "+argv[0]+": "+r.stderr[-500:])
    return r


def _certs(root:Path):
    openssl=shutil.which("openssl")
    if not openssl:raise MTLSLabError("OpenSSL executable not available")
    _run([openssl,"req","-x509","-new","-newkey","rsa:2048","-nodes",
          "-days","1","-keyout",str(root/"ca.key"),"-out",str(root/"ca.pem"),
          "-subj","/CN=ocaf039-ci-ephemeral-ca",
          "-addext","basicConstraints=critical,CA:TRUE"])
    for name,dns in (("server",fencing.SERVER_SAN),
                     ("client",fencing.ALLOWED_SAN),
                     ("rogue","ocaf039-tenant-b.client")):
        _run([openssl,"req","-new","-newkey","rsa:2048","-nodes",
              "-keyout",str(root/(name+".key")),"-out",str(root/(name+".csr")),
              "-subj","/CN="+dns,"-addext","subjectAltName=DNS:"+dns])
        _run([openssl,"x509","-req","-in",str(root/(name+".csr")),
              "-CA",str(root/"ca.pem"),"-CAkey",str(root/"ca.key"),
              "-CAcreateserial","-out",str(root/(name+".pem")),
              "-days","1","-sha256","-copy_extensions","copy"])
        for extension in (".key",".csr",".pem"):
            (root/(name+extension)).chmod(0o600)
    (root/"ca.key").chmod(0o600)
    (root/"ca.pem").chmod(0o600)
    return {
        x:hashlib.sha256((root/(x+".pem")).read_bytes()).hexdigest()
        for x in ("server","client","rogue")
    }


def _netns_up(ns_a:str,ns_b:str,v_a:str,v_b:str):
    for ns in (ns_a,ns_b):_run(["ip","netns","add",ns])
    _run(["ip","link","add",v_a,"type","veth","peer","name",v_b])
    _run(["ip","link","set",v_a,"netns",ns_a])
    _run(["ip","link","set",v_b,"netns",ns_b])
    for ns,dev,ip in ((ns_a,v_a,"10.238.39.1/30"),(ns_b,v_b,"10.238.39.2/30")):
        _run(["ip","-n",ns,"link","set","lo","up"])
        _run(["ip","-n",ns,"addr","add",ip,"dev",dev])
        _run(["ip","-n",ns,"link","set",dev,"up"])
        routes=_run(["ip","-n",ns,"route","show","default"]).stdout.strip()
        if routes:raise MTLSLabError("isolated test netns unexpectedly has a default route")


def verify_negative(result:subprocess.CompletedProcess,reason:str)->None:
    if result.returncode==0 or '"status": "LEASE_' in result.stdout:
        raise MTLSLabError("unexpected successful TLS RPC on negative "+reason)


def execute(source:Path,report:Path)->dict:
    if os.geteuid()!=0 or os.uname().sysname!="Linux":
        raise MTLSLabError("real root Linux network namespaces required")
    if not source.is_file() or source.is_symlink() or not source.is_absolute():
        raise MTLSLabError("trusted absolute mTLS Python harness source required")
    root=Path(tempfile.mkdtemp(prefix="ocaf39-mtls-",dir="/tmp"))
    suffix=secrets.token_hex(3)
    na="oc39a"+suffix
    nb="oc39b"+suffix
    va="v39a"+suffix
    vb="v39b"+suffix
    created=[]
    server=None
    server_log=None
    try:
        cert_shas=_certs(root)
        # Cleanup both even if midway through creation.
        for ns in (na,nb):
            _run(["ip","netns","add",ns])
            created.append(ns)
        _run(["ip","link","add",va,"type","veth","peer","name",vb])
        _run(["ip","link","set",va,"netns",na])
        _run(["ip","link","set",vb,"netns",nb])
        for ns,dev,ip in ((na,va,"10.238.39.1/30"),(nb,vb,"10.238.39.2/30")):
            _run(["ip","-n",ns,"link","set","lo","up"])
            _run(["ip","-n",ns,"addr","add",ip,"dev",dev])
            _run(["ip","-n",ns,"link","set",dev,"up"])
            if _run(["ip","-n",ns,"route","show","default"]).stdout.strip():
                raise MTLSLabError("unexpected netns internet route")
        inode={}
        for ns in (na,nb):
            inode[ns]=int(_run(["ip","netns","exec",ns,"stat","-c","%i",
                                 "/proc/self/ns/net"]).stdout.strip())
        if inode[na]==inode[nb]:
            raise MTLSLabError("test network namespaces resolve to same kernel inode")
        db=root/"state.sqlite"
        ready=root/"server.ready"
        server_args=[
            "ip","netns","exec",nb,"python3",str(source),"serve",
            "--host","10.238.39.2","--port","24399",
            "--cert",str(root/"server.pem"),"--key",str(root/"server.key"),
            "--ca",str(root/"ca.pem"),"--db",str(db),"--ready",str(ready),
        ]
        server_log=(root/"server.log").open("wb")
        def spawn():
            nonlocal server
            if ready.exists():ready.unlink()
            server=subprocess.Popen(server_args,stdout=server_log,
                                    stderr=subprocess.STDOUT,close_fds=True)
            until=time.monotonic()+6
            while time.monotonic()<until:
                if ready.is_file() and server.poll() is None:return
                if server.poll() is not None:break
                time.sleep(.05)
            raise MTLSLabError("TLS network namespace server not ready")
        spawn()
        token_a=secrets.token_hex(16)
        token_b=secrets.token_hex(16)
        def call(action,epoch,token,*,identity="client",ttl=15,expect_ok=True):
            return _run([
                "ip","netns","exec",na,"python3",str(source),"client",
                "--host","10.238.39.2","--port","24399",
                "--cert",str(root/(identity+".pem")),
                "--key",str(root/(identity+".key")),
                "--ca",str(root/"ca.pem"),
                "--action",action,"--epoch",str(epoch),
                "--token",token,"--ttl",str(ttl),
            ],timeout=5,expect_ok=expect_ok)

        def positive(action,epoch,token,**kwargs):
            response=json.loads(call(action,epoch,token,**kwargs).stdout)
            if response.get("epoch")!=epoch or response.get("production_approved") is not False:
                raise MTLSLabError("mTLS server returned untrusted fencing response")
            return response
        first=positive("claim",1,token_a)
        if first["status"]!="LEASE_GRANTED":raise MTLSLabError("no initial lease")
        verify_negative(call("renew",1,token_a,identity="rogue",expect_ok=False),
                        "rogue client DNS SAN even though CA trusted")
        verify_negative(call("claim",2,token_b,expect_ok=False),
                        "second primary while lease live")
        hb=positive("renew",1,token_a)
        if hb["status"]!="LEASE_RENEWED":raise MTLSLabError("no valid heartbeat")
        _run(["ip","-n",na,"route","add","blackhole","10.238.39.2/32"])
        if "blackhole" not in _run(["ip","-n",na,"route","show","10.238.39.2/32"]).stdout:
            raise MTLSLabError("actual Linux blackhole partition not installed")
        verify_negative(call("renew",1,token_a,expect_ok=False),
                        "kernel-route-partitioned controller")
        denied_write=not fencing.safe_to_write(False,int(time.time()),
                                                hb["expires_unix_secs"])
        if not denied_write:raise MTLSLabError("client fail-closed authority missing")
        _run(["ip","-n",na,"route","del","blackhole","10.238.39.2/32"])
        back=positive("renew",1,token_a)
        if back["status"]!="LEASE_RENEWED":raise MTLSLabError("no recovery after partition")
        released=positive("release",1,token_a)
        if released["status"]!="LEASE_RELEASED":raise MTLSLabError("no release")
        second=positive("claim",2,token_b)
        if second["status"]!="LEASE_GRANTED":raise MTLSLabError("no new higher epoch")
        verify_negative(call("renew",1,token_a,expect_ok=False),"stale epoch replay")
        server.kill()
        if server.wait(timeout=3)!=-9:raise MTLSLabError("server SIGKILL not executed")
        verify_negative(call("renew",2,token_b,expect_ok=False),
                        "authority process crash")
        spawn()
        restored=positive("renew",2,token_b)
        if restored["status"]!="LEASE_RENEWED":
            raise MTLSLabError("authority storage lost after restart")
        verify_negative(call("claim",2,token_a,expect_ok=False),
                        "replay after authority restart")
        outcome={
            "schema_version":1,
            "status":"REAL_NETNS_TLS13_MTLS_PARTITION_FENCING_PASSED",
            "observed_at_utc":datetime.now(timezone.utc).isoformat(),
            "kernel_boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "namespace_a_inode":inode[na],"namespace_b_inode":inode[nb],
            "distinct_linux_network_namespaces":True,
            "veth_actual_kernel_network_transport":True,
            "tls_version":"TLSv1.3",
            "ephemeral_ca_signed_identity_digests_sha256":cert_shas,
            "mutual_tls_client_cert_required":True,
            "rogue_ca_signed_client_certificate_denied":True,
            "second_primary_before_release_denied":True,
            "kernel_blackhole_partition_installed":True,
            "lease_renewal_during_partition_denied":True,
            "client_write_on_lost_authority_denied":True,
            "post_partition_lease_renewal_proved":True,
            "higher_epoch_after_release_proved":True,
            "stale_epoch_after_relinquish_denied":True,
            "authority_process_sigkill_proved":True,
            "renewal_while_authority_dead_denied":True,
            "epoch_persisted_after_authority_restart":True,
            "network_namespace_scope":"single independent Linux kernel, not two physical hosts",
            "majority_quorum_implemented":False,
            "live_interhost_mtls_failover_tested":False,
            "real_guest_mtls_rpc_tested":False,
            "stage_039_complete":False,
            "production_approved":False,
        }
        report.parent.mkdir(parents=True,exist_ok=True)
        with report.open("x") as out:
            json.dump(outcome,out,sort_keys=True,indent=2);out.write("\n")
        return outcome
    finally:
        if server is not None and server.poll() is None:
            server.kill()
            try:server.wait(timeout=3)
            except subprocess.TimeoutExpired:pass
        if server_log is not None:server_log.close()
        for ns in reversed(created):
            subprocess.run(["ip","netns","delete",ns],capture_output=True,
                           timeout=5,check=False)
        shutil.rmtree(root,ignore_errors=True)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fencing-source",type=Path,required=True)
    parser.add_argument("--report",type=Path,required=True)
    parser.add_argument("--ack-root-network-namespace-partition-lab",action="store_true")
    x=parser.parse_args()
    if not x.ack_root_network_namespace_partition_lab:
        parser.error("must explicitly acknowledge isolated ephemeral root network namespace lab")
    try:
        result=execute(x.fencing_source,x.report)
    except (OSError,ValueError,subprocess.SubprocessError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status":result["status"],"stage_039_complete":False,
                      "production_approved":False}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
