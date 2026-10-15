#!/usr/bin/env python3
"""0.39.15 — actual Python TLSv1.3 mutual-certificate network fencing RPC.

This is a bounded, ephemeral LAB security harness, NOT distributed
consensus: the authority is a single durable SQLite lease store. No
majority, remote attestation or production GO is implied. An unreachable
authority MUST stop writes; clients never self-promote on lost quorum.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import socket
import sqlite3
import ssl
import stat
import struct
import time

MAX_BYTES=4096
MAX_TTL=30
TENANT="tenant-a"
ALLOWED_SAN="ocaf039-tenant-a.client"
SERVER_SAN="ocaf039-broker.local"
LEASE=re.compile(r"[a-f0-9]{32}\Z")


class FenceError(ValueError):
    pass


def strict_request(raw:bytes)->dict:
    if not isinstance(raw,bytes) or not 0<len(raw)<=MAX_BYTES:
        raise FenceError("invalid bounded TLS RPC frame")
    def unique(pairs):
        out={}
        for k,v in pairs:
            if k in out: raise FenceError("duplicate RPC JSON attribute")
            out[k]=v
        return out
    try:
        data=json.loads(raw,object_pairs_hook=unique,
                        parse_constant=lambda _:(_ for _ in ()).throw(FenceError("NaN denied")))
    except (UnicodeError,ValueError,TypeError) as exc:
        raise FenceError("malformed TLS RPC JSON") from exc
    if (not isinstance(data,dict) or set(data)!=
            {"version","action","tenant_id","lease_id","epoch","ttl_seconds"}
            or type(data["version"]) is not int or data["version"]!=1
            or data["action"] not in ("claim","renew","release")
            or data["tenant_id"]!=TENANT
            or not isinstance(data["lease_id"],str)
            or LEASE.fullmatch(data["lease_id"]) is None
            or type(data["epoch"]) is not int or not 1<=data["epoch"]<2**63
            or type(data["ttl_seconds"]) is not int
            or not 2<=data["ttl_seconds"]<=MAX_TTL):
        raise FenceError("unauthorized principal, epoch or lease request")
    return data


def peer_authorized(cert:dict)->bool:
    if not isinstance(cert,dict):return False
    # Crucial: no CN fallback or multiple identities allowed.
    return cert.get("subjectAltName")==(("DNS",ALLOWED_SAN),)


def safe_to_write(authenticated_ack:bool,now:int,expires:int)->bool:
    return (authenticated_ack is True and type(now) is int
            and type(expires) is int and now>=0 and expires>now)


class FenceLedger:
    """Single authority only. Atomic SQLite BEGIN IMMEDIATE, durable epochs."""
    def __init__(self,path:Path):
        if not isinstance(path,Path) or not path.is_absolute() or ".." in path.parts:
            raise FenceError("unsafe lease storage path")
        parent=path.parent
        if (parent.is_symlink() or not parent.is_dir()
                or parent.stat().st_mode & 0o077):
            raise FenceError("lease state must live in private directory")
        if path.is_symlink():
            raise FenceError("lease DB symlink refused")
        if path.exists() and (not path.is_file() or path.stat().st_mode & 0o077
                              or path.stat().st_nlink!=1):
            raise FenceError("untrusted lease DB inode")
        if not path.exists():
            fd=os.open(path,os.O_RDWR|os.O_CREAT|os.O_EXCL|
                       getattr(os,"O_NOFOLLOW",0),0o600)
            os.close(fd)
        self.conn=sqlite3.connect(str(path),isolation_level=None,timeout=2)
        self.conn.execute("PRAGMA journal_mode=WAL")
        self.conn.execute("PRAGMA synchronous=FULL")
        self.conn.execute("PRAGMA busy_timeout=2000")
        self.conn.execute("PRAGMA trusted_schema=OFF")
        self.conn.execute("""CREATE TABLE IF NOT EXISTS fencing (
            tenant TEXT PRIMARY KEY, epoch INTEGER NOT NULL,
            token TEXT NOT NULL, expires INTEGER NOT NULL)""")

    def close(self):
        self.conn.close()

    def __enter__(self):
        return self

    def __exit__(self,*exc):
        self.close()

    def apply(self,request:dict,*,now:int)->dict:
        if type(now) is not int or now<0:
            raise FenceError("invalid authoritative monotonic clock")
        r=strict_request(json.dumps(request,separators=(",",":")).encode())
        try:
            self.conn.execute("BEGIN IMMEDIATE")
            old=self.conn.execute(
                "SELECT epoch,token,expires FROM fencing WHERE tenant=?",(TENANT,)).fetchone()
            epoch,token,ttl=r["epoch"],r["lease_id"],r["ttl_seconds"]
            if r["action"]=="claim":
                if old is not None and (now<old[2] or epoch<=old[0]):
                    raise FenceError("second primary or stale epoch denied")
                expiry=now+ttl
                self.conn.execute("""INSERT INTO fencing (tenant,epoch,token,expires)
                        VALUES(?,?,?,?) ON CONFLICT(tenant)
                        DO UPDATE SET epoch=excluded.epoch,token=excluded.token,expires=excluded.expires""",
                        (TENANT,epoch,token,expiry))
                name="LEASE_GRANTED"
            elif r["action"]=="renew":
                if old is None or old[0]!=epoch or old[1]!=token or now>=old[2]:
                    raise FenceError("invalid, fenced or expired lease heartbeat")
                expiry=now+ttl
                self.conn.execute("UPDATE fencing SET expires=? WHERE tenant=?",(expiry,TENANT))
                name="LEASE_RENEWED"
            else:
                if old is None or old[0]!=epoch or old[1]!=token or now>=old[2]:
                    raise FenceError("stale holder cannot release lease")
                expiry=now
                self.conn.execute("UPDATE fencing SET expires=? WHERE tenant=?",(expiry,TENANT))
                name="LEASE_RELEASED"
            self.conn.execute("COMMIT")
        except BaseException:
            if self.conn.in_transaction:self.conn.execute("ROLLBACK")
            raise
        return {"status":name,"tenant_id":TENANT,"epoch":epoch,
                "expires_unix_secs":expiry,"authority":"SINGLE_HOST_SQLITE",
                "distributed_quorum_verified":False,"production_approved":False}


def _recv(sock,n):
    chunks=bytearray()
    while len(chunks)<n:
        b=sock.recv(n-len(chunks))
        if not b:raise FenceError("TLS RPC truncated")
        chunks.extend(b)
    return bytes(chunks)


def _recv_frame(sock):
    n=struct.unpack("!I",_recv(sock,4))[0]
    if not 0<n<=MAX_BYTES:raise FenceError("oversized TLS RPC")
    return _recv(sock,n)


def _send_frame(sock,data):
    if not isinstance(data,bytes) or not 0<len(data)<=MAX_BYTES:
        raise FenceError("invalid TLS RPC response length")
    sock.sendall(struct.pack("!I",len(data))+data)


def _tls_server_context(cert,key,ca):
    ctx=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.minimum_version=ssl.TLSVersion.TLSv1_3
    ctx.maximum_version=ssl.TLSVersion.TLSv1_3
    ctx.verify_mode=ssl.CERT_REQUIRED
    ctx.load_cert_chain(certfile=str(cert),keyfile=str(key))
    ctx.load_verify_locations(cafile=str(ca))
    return ctx


def _tls_client_context(cert,key,ca):
    ctx=ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    ctx.minimum_version=ssl.TLSVersion.TLSv1_3
    ctx.maximum_version=ssl.TLSVersion.TLSv1_3
    ctx.check_hostname=True
    ctx.verify_mode=ssl.CERT_REQUIRED
    ctx.load_cert_chain(certfile=str(cert),keyfile=str(key))
    ctx.load_verify_locations(cafile=str(ca))
    return ctx


def server_forever(*,bind:str,port:int,cert:Path,key:Path,ca:Path,db:Path,ready:Path):
    ctx=_tls_server_context(cert,key,ca)
    ledger=FenceLedger(db)
    try:
        with socket.socket(socket.AF_INET,socket.SOCK_STREAM) as server:
            server.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
            server.bind((bind,port))
            server.listen(10)
            server.settimeout(0.5)
            ready.write_text("ready\n")
            while True:
                try:
                    raw,addr=server.accept()
                except socket.timeout:continue
                with raw:
                    raw.settimeout(2)
                    try:
                        with ctx.wrap_socket(raw,server_side=True) as client:
                            if not peer_authorized(client.getpeercert()):
                                raise FenceError("TLS client DNS SAN not authorized")
                            request=strict_request(_recv_frame(client))
                            outcome=ledger.apply(request,now=int(time.time()))
                            _send_frame(client,json.dumps(outcome,sort_keys=True).encode())
                    except (OSError,ssl.SSLError,FenceError):
                        # Reject unauthorized clients without confirmation.
                        pass
    finally:
        ledger.close()


def client_rpc(*,host:str,port:int,cert:Path,key:Path,ca:Path,request:dict):
    req=strict_request(json.dumps(request,sort_keys=True).encode())
    ctx=_tls_client_context(cert,key,ca)
    with socket.create_connection((host,port),timeout=2) as plain:
        plain.settimeout(2)
        with ctx.wrap_socket(plain,server_hostname=SERVER_SAN) as tls:
            if tls.version()!="TLSv1.3":raise FenceError("TLS1.3 was not negotiated")
            _send_frame(tls,json.dumps(req,sort_keys=True).encode())
            response=json.loads(_recv_frame(tls))
    if (not isinstance(response,dict)
            or response.get("tenant_id")!=TENANT
            or response.get("epoch")!=req["epoch"]
            or response.get("status") not in
               ("LEASE_GRANTED","LEASE_RENEWED","LEASE_RELEASED")
            or response.get("production_approved") is not False):
        raise FenceError("untrusted or mismatched lease response")
    return response


def main()->int:
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("mode",choices=("serve","client"))
    p.add_argument("--host",required=True)
    p.add_argument("--port",type=int,default=24399)
    for key in ("cert","key","ca"):
        p.add_argument("--"+key,type=Path,required=True)
    p.add_argument("--db",type=Path)
    p.add_argument("--ready",type=Path)
    p.add_argument("--action",choices=("claim","renew","release"))
    p.add_argument("--token")
    p.add_argument("--epoch",type=int)
    p.add_argument("--ttl",type=int,default=6)
    x=p.parse_args()
    if not 1024<=x.port<=65535:p.error("bounded port required")
    try:
        if x.mode=="serve":
            if x.db is None or x.ready is None:p.error("server DB and readiness path required")
            server_forever(bind=x.host,port=x.port,cert=x.cert,key=x.key,
                           ca=x.ca,db=x.db,ready=x.ready)
        else:
            if x.action is None or x.token is None or x.epoch is None:
                p.error("complete bounded lease request required")
            response=client_rpc(host=x.host,port=x.port,cert=x.cert,key=x.key,ca=x.ca,
                request={"version":1,"action":x.action,"tenant_id":TENANT,
                         "lease_id":x.token,"epoch":x.epoch,"ttl_seconds":x.ttl})
            print(json.dumps(response,sort_keys=True))
    except (OSError,ValueError,ssl.SSLError,sqlite3.Error) as exc:
        p.error(str(exc))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
