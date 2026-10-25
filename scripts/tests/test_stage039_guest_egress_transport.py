"""Kernel peer binding plus real local TLS across the proposed vsock bridge."""
import base64
import http.server
import ipaddress
import json
import os
from pathlib import Path
import shutil
import socket
import ssl
import struct
import subprocess
import sys
import tempfile
import threading
import time
import unittest

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage039_egress_broker import EgressBroker
from stage039_guest_egress_transport import VmPeer, handle_one, listen_once, listen_many, MAX_PERMIT, _trusted_parent_owner
from stage039_signed_manifest import canonical_payload


class _Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        data = b"egress-over-kernel-bound-bridge"
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *_args):
        pass


class GuestEgressTransportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not hasattr(socket, "SO_PEERCRED") or shutil.which("openssl") is None:
            raise unittest.SkipTest("Linux SO_PEERCRED and openssl required")
        cls.temp = tempfile.TemporaryDirectory(dir="/dev/shm")
        cert = Path(cls.temp.name)/"cert.pem"
        key = Path(cls.temp.name)/"key.pem"
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048",
                        "-keyout", str(key), "-out", str(cert), "-days", "1",
                        "-nodes", "-subj", "/CN=localhost",
                        "-addext", "subjectAltName=DNS:localhost"],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        cls.cert = cert
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(cert, key)
        cls.server.socket = ctx.wrap_socket(cls.server.socket, server_side=True)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=2)
        cls.temp.cleanup()

    def setUp(self):
        self.peer = VmPeer.capture(tenant_id="tenant-a", uid=os.geteuid(), pid=os.getpid())
        self.key = Ed25519PrivateKey.generate()
        public = self.key.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        context = ssl.create_default_context(cafile=str(self.cert))
        port = self.server.server_address[1]
        self.broker = EgressBroker(
            trusted_keys={"lab": public}, ssl_context=context,
            allowed_ports=frozenset({port}),
            allowed_networks=(ipaddress.ip_network("127.0.0.1/32"),),
            resolver=lambda _host, port, **kwargs:
                socket.getaddrinfo("127.0.0.1", port, **kwargs))

    def permit(self, *, tenant="tenant-a", nonce="a"*32, url=None):
        payload={"schema_version":1,"tenant_id":tenant,"nonce":nonce,
                 "url":url or f"https://localhost:{self.server.server_address[1]}/ok",
                 "expires_unix":int(time.time())+60,"max_bytes":128}
        public=self.key.public_key().public_bytes(
            serialization.Encoding.Raw,serialization.PublicFormat.Raw)
        return json.dumps({"key_id":"lab","public_key":base64.b64encode(public).decode(),
                           "signature":base64.b64encode(self.key.sign(canonical_payload(payload))).decode(),
                           "payload":payload}).encode()

    def exchange(self, payload:bytes, *,peer=None,broker=None):
        left,right=socket.socketpair(socket.AF_UNIX,socket.SOCK_STREAM)
        with left,right:
            right.sendall(struct.pack("!I",len(payload))+payload)
            decision=handle_one(left,peer=peer or self.peer,broker=broker or self.broker)
            header=right.recv(5)
            self.assertEqual(len(header),5)
            status,length=struct.unpack("!BI",header)
            response=b""
            while len(response)<length:
                response+=right.recv(length-len(response))
            return decision,status,response

    def test_jailer_chowned_parent_is_trusted_only_for_bound_vm_uid(self):
        jailed = VmPeer(self.peer.tenant_id, os.geteuid() + 2,
                        self.peer.pid, self.peer.proc_start_ticks,
                        self.peer.cgroup, self.peer.host_boot_id)
        self.assertTrue(_trusted_parent_owner(jailed.uid, jailed))
        self.assertFalse(_trusted_parent_owner(jailed.uid + 1, jailed))

    def test_signed_tenant_uses_kernel_peer_to_fetch_real_local_tls(self):
        decision,status,response=self.exchange(self.permit())
        self.assertEqual(decision["status"],"GRANTED")
        self.assertEqual(decision["tenant_id"],"tenant-a")
        self.assertEqual(status,1)
        self.assertEqual(response,b"egress-over-kernel-bound-bridge")
        replay,status,response=self.exchange(self.permit())
        self.assertEqual((replay["status"],status,response),("DENIED",0,b""))

    def test_cross_tenant_metadata_and_broker_outage_fail_closed(self):
        for payload in (
            self.permit(tenant="tenant-b",nonce="b"*32),
            self.permit(nonce="c"*32,url="https://169.254.169.254/latest/meta-data/"),
            b"x"*(MAX_PERMIT+1),
        ):
            with self.subTest(length=len(payload)):
                decision,status,response=self.exchange(payload)
                self.assertEqual((decision["status"],status,response),("DENIED",0,b""))
        class Down:
            def fetch(self, **_kwargs):
                raise RuntimeError("egress authority unavailable")
        decision,status,response=self.exchange(self.permit(nonce="d"*32),broker=Down())
        self.assertEqual((decision["status"],status,response),("DENIED",0,b""))
        port=self.server.server_address[1]
        self.broker.resolver=lambda _host,_port,**kwargs: [
            *socket.getaddrinfo("127.0.0.1",port,**kwargs),
            *socket.getaddrinfo("169.254.169.254",port,**kwargs)]
        decision,status,response=self.exchange(self.permit(nonce="1"*32))
        self.assertEqual((decision["status"],status,response),("DENIED",0,b""))

    def test_swapped_or_restarted_vm_pid_is_rejected_before_broker_call(self):
        class Spy:
            called=False
            def fetch(self, **_kwargs):
                self.called=True
                return b"should-not-be-fetched"
        spy=Spy()
        wrong=VmPeer(self.peer.tenant_id,self.peer.uid,self.peer.pid+1,
                     self.peer.proc_start_ticks,self.peer.cgroup,self.peer.host_boot_id)
        decision,status,response=self.exchange(self.permit(nonce="e"*32),peer=wrong,broker=spy)
        self.assertEqual((decision["status"],status,response),("DENIED",0,b""))
        self.assertFalse(spy.called)
        stale=VmPeer(self.peer.tenant_id,self.peer.uid,self.peer.pid,
                     self.peer.proc_start_ticks+1,self.peer.cgroup,self.peer.host_boot_id)
        decision,status,response=self.exchange(self.permit(nonce="f"*32),peer=stale,broker=spy)
        self.assertEqual((decision["status"],status,response),("DENIED",0,b""))
        self.assertFalse(spy.called)

    def test_private_vsock_backend_path_is_created_and_removed(self):
        path=Path(self.temp.name)/"vm-a-vsock_5000"
        results=[]
        thread=threading.Thread(target=lambda: results.append(
            listen_once(path,peer=self.peer,broker=self.broker)),daemon=True)
        thread.start()
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as conn:
            for _ in range(100):
                try:
                    conn.connect(str(path))
                    break
                except (FileNotFoundError, ConnectionRefusedError):
                    time.sleep(.01)
            else:
                self.fail("bounded private vsock UDS listener did not start")
            permit=self.permit(nonce="2"*32)
            conn.sendall(struct.pack("!I",len(permit))+permit)
            header=conn.recv(5)
            self.assertEqual(header[0],1)
            length=struct.unpack("!I",header[1:])[0]
            self.assertEqual(conn.recv(length),b"egress-over-kernel-bound-bridge")
        thread.join(timeout=2)
        self.assertFalse(thread.is_alive())
        self.assertFalse(path.exists())
        self.assertEqual(results[0]["status"],"GRANTED")

    def test_two_real_kernel_peer_connections_grant_then_replay_denial(self):
        path=Path(self.temp.name)/"vm-replay_5000"
        results=[]
        thread=threading.Thread(target=lambda: results.extend(
            listen_many(path,peer=self.peer,broker=self.broker,requests=2)),daemon=True)
        thread.start()
        permit=self.permit(nonce="3"*32)
        for expected in (1,0):
            with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as conn:
                for _ in range(100):
                    try:
                        conn.connect(str(path))
                        break
                    except (FileNotFoundError,ConnectionRefusedError):
                        time.sleep(.01)
                else:
                    self.fail("real private UDS listener did not start")
                conn.sendall(struct.pack("!I",len(permit))+permit)
                status,length=struct.unpack("!BI",conn.recv(5))
                self.assertEqual(status,expected)
                if expected:
                    self.assertEqual(conn.recv(length),b"egress-over-kernel-bound-bridge")
                else:
                    self.assertEqual(length,0)
        thread.join(timeout=2)
        self.assertFalse(thread.is_alive())
        self.assertFalse(path.exists())
        self.assertEqual([r["status"] for r in results],["GRANTED","DENIED"])
        self.assertEqual(results[0]["guest_response_bytes"],len(b"egress-over-kernel-bound-bridge"))
        self.assertIn("replayed destination permit", results[1]["deny_reason"])


if __name__=="__main__":
    unittest.main()
