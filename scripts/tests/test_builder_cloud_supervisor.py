"""Process-lifecycle regressions; executable fixture is NOT the Rust backend."""
import base64
import json
import hashlib
import http.client
import select
import ssl
import signal
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import threading
import unittest

from scripts.builder_cloud_supervisor import (BridgeProcess, StartupError,
    private_config, read_bridge_config)

FIXTURE = '''#!/usr/bin/env python3
import base64,json,os,signal,socket,struct,sys
from pathlib import Path
cfg=json.loads(Path(sys.argv[1]).read_text()); p=cfg['socket']
mode=cfg['test_mode']
if cfg.get('pid_file'): Path(cfg['pid_file']).write_text(str(os.getpid()))
if mode=='exit': sys.exit(17)
if mode=='stall':
    signal.pause();sys.exit(0)
s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);s.bind(p);os.chmod(p,0o600);s.listen()
def stop(*a):
    s.close();os.unlink(p);sys.exit(0)
signal.signal(signal.SIGTERM,stop)
def read_exact(c,n):
    b=b''
    while len(b)<n:
        d=c.recv(n-len(b))
        if not d: raise EOFError()
        b+=d
    return b
while True:
    c,_=s.accept()
    with c:
        try:
            n=struct.unpack('!I',read_exact(c,4))[0];r=json.loads(read_exact(c,n))
            status=200 if mode=='bad' or r else 400
            body= b'{"error":"http_400"}' if not r else json.dumps({'fixture':True,'tenant':r['subject']['tenant']}).encode()
            raw=json.dumps({'status':status,'body_b64':base64.b64encode(body).decode()}).encode()
            c.sendall(struct.pack('!I',len(raw))+raw)
        except (EOFError,OSError): pass
'''

class SupervisorTest(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.exe=self.root/'bridge';self.exe.write_text(FIXTURE);self.exe.chmod(0o700)
        self.config=self.root/'config.json';self.sock=self.root/'bridge.sock'
        self.write_config()
    def write_config(self, mode='ok'):
        self.config.write_text(json.dumps({'socket':str(self.sock),'origin':'https://localhost','test_mode':mode}));self.config.chmod(0o600)
    def launch(self, **kw):
        return BridgeProcess(self.exe,self.config,self.sock,**kw)
    def test_start_probes_protocol_and_stop_reaps_child(self):
        with self.launch(startup_timeout=3,shutdown_timeout=2) as bridge:
            pid=bridge.process.pid
            self.assertIsNone(bridge.process.poll());self.assertTrue(self.sock.is_socket())
        self.assertIsNotNone(bridge.process.poll());self.assertFalse(self.sock.exists())
        with self.assertRaises(ProcessLookupError): os.kill(pid,0)
    def test_child_early_exit_never_reports_ready(self):
        self.write_config('exit');b=self.launch(startup_timeout=2,shutdown_timeout=1)
        with self.assertRaises(StartupError): b.start()
        self.assertEqual(b.process.poll(),17)
    def test_deadline_reaps_unready_child(self):
        self.write_config('stall');b=self.launch(startup_timeout=.3,shutdown_timeout=1)
        start=time.monotonic()
        with self.assertRaises(StartupError):b.start()
        self.assertLess(time.monotonic()-start,3);self.assertIsNotNone(b.process.poll())
    def test_startup_cancellation_reaps_child(self):
        self.write_config('stall')
        stop=threading.Event()
        timer=threading.Timer(.2,stop.set)
        b=self.launch(startup_timeout=10,shutdown_timeout=1,cancel=stop)
        timer.start();start=time.monotonic()
        try:
            with self.assertRaises(StartupError):b.start()
            self.assertLess(time.monotonic()-start,3)
            self.assertIsNotNone(b.process.poll())
        finally:timer.cancel();timer.join()
    def test_invalid_protocol_fails_closed(self):
        self.write_config('bad');b=self.launch(startup_timeout=.5,shutdown_timeout=1)
        with self.assertRaises(StartupError):b.start()
        self.assertIsNotNone(b.process.poll());self.assertFalse(self.sock.exists())
    def test_existing_socket_never_removed_or_replaced(self):
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:
            s.bind(str(self.sock));self.sock.chmod(0o600);s.listen()
            inode=self.sock.stat().st_ino
            with self.assertRaises(StartupError):self.launch().start()
            self.assertEqual(self.sock.stat().st_ino,inode)
    def test_private_config_rejects_links_and_public_permissions(self):
        self.assertIsNone(private_config(self.config))
        self.config.chmod(0o644)
        with self.assertRaises(ValueError):private_config(self.config)
        self.config.chmod(0o600);alias=self.root/'alias';alias.symlink_to(self.config)
        with self.assertRaises(ValueError):private_config(alias)
        alias.unlink();os.link(self.config,alias)
        with self.assertRaises(ValueError):private_config(self.config)
    def test_config_origin_and_socket_cannot_be_overridden(self):
        cfg=read_bridge_config(self.config,'https://localhost')
        self.assertEqual(cfg,self.sock)
        with self.assertRaises(ValueError):read_bridge_config(self.config,'https://other.example')
        self.config.write_text('{"origin":"https://localhost","origin":"https://other.example","socket":"/tmp/escape"}')
        with self.assertRaises(ValueError):read_bridge_config(self.config,'https://localhost')

class SupervisedTlsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Reuse the established test PKI; all keys remain in a temporary directory.
        from test_builder_cloud_ingress import IngressTest
        cls.fixture=IngressTest
        cls.fixture.setUpClass()
    @classmethod
    def tearDownClass(cls):cls.fixture.tearDownClass()
    def start_service(self):
        root=self.fixture.root
        tmp=tempfile.TemporaryDirectory();self.addCleanup(tmp.cleanup)
        work=Path(tmp.name);exe=work/'bridge';exe.write_text(FIXTURE);exe.chmod(0o700)
        self.pid_file=work/'pid';self.sock=work/'bridge.sock';config=work/'config.json'
        config.write_text(json.dumps({'socket':str(self.sock),'origin':'https://127.0.0.1','test_mode':'ok','pid_file':str(self.pid_file)}));config.chmod(0o600)
        registry=work/'identities.json';registry.write_text(json.dumps(self.fixture.registry));registry.chmod(0o600)
        key=work/'server.key';key.write_bytes((root/'server.key').read_bytes());key.chmod(0o600)
        command=[sys.executable,'-m','scripts.builder_cloud_supervisor','--bridge-bin',str(exe),'--bridge-config',str(config),'--origin','https://127.0.0.1','--port','0','--cert',str(root/'server.crt'),'--key',str(key),'--ca',str(root/'ca.crt'),'--registry',str(registry),'--assets',str(root/'www')]
        proc=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        def cleanup():
            if proc.poll() is None:
                proc.terminate()
                try:proc.wait(10)
                except subprocess.TimeoutExpired:proc.kill();proc.wait()
            proc.stdout.close();proc.stderr.close()
        self.addCleanup(cleanup)
        if not select.select([proc.stdout],[],[],8)[0]:self.fail('supervisor readiness deadline')
        line=proc.stdout.readline().strip()
        self.assertRegex(line,r'^OCAF Builder HTTPS listener ready .* port=\d+$')
        return proc,int(line.rsplit('=',1)[1])
    def request(self,port):
        root=self.fixture.root
        context=ssl.create_default_context(cafile=str(root/'ca.crt'));context.load_cert_chain(root/'alice.crt',root/'alice.key')
        conn=http.client.HTTPSConnection('127.0.0.1',port,context=context,timeout=3)
        try:
            conn.request('GET','/v1/projects/demo',headers={'Host':'127.0.0.1','Authorization':'Bearer test-only-token'})
            response=conn.getresponse();return response.status,json.loads(response.read())
        finally:conn.close()
    def test_cli_real_tls_request_and_graceful_shutdown(self):
        proc,port=self.start_service();self.assertEqual(self.request(port),(200,{'fixture':True,'tenant':'a'}))
        pid=int(self.pid_file.read_text());proc.terminate();self.assertEqual(proc.wait(10),0)
        with self.assertRaises(ProcessLookupError):os.kill(pid,0)
        self.assertFalse(self.sock.exists())
        with self.assertRaises(OSError):socket.create_connection(('127.0.0.1',port),timeout=1)
    def test_bridge_death_stops_https_and_exits_nonzero(self):
        proc,port=self.start_service();self.assertEqual(self.request(port)[0],200)
        os.kill(int(self.pid_file.read_text()),signal.SIGKILL)
        self.assertEqual(proc.wait(10),1)
        with self.assertRaises(OSError):socket.create_connection(('127.0.0.1',port),timeout=1)
        self.assertTrue(self.sock.is_socket(),'unclean socket must not be automatically deleted')

if __name__=='__main__': unittest.main()
