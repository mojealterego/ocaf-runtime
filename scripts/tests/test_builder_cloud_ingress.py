import base64
from contextlib import closing
import hashlib
import http.client
import json
import os
from pathlib import Path
import socket
import ssl
import struct
import subprocess
import sys
import tempfile
import threading
import unittest

from scripts.builder_cloud_ingress import Ingress, GatewayConfig, MAX_BODY, MAX_FRAME, ASSETS, read_frame, write_frame, load_json


class IngressTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.tmp.name)
        def openssl(*args):
            subprocess.run(['openssl', *args], cwd=cls.root, check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        openssl('req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', 'ca.key', '-out', 'ca.crt', '-subj', '/CN=Test CA', '-days', '1', '-addext', 'basicConstraints=critical,CA:TRUE', '-addext', 'keyUsage=critical,keyCertSign,cRLSign')
        for name in ['server', 'alice', 'bob', 'unknown']:
            openssl('req', '-newkey', 'rsa:2048', '-nodes', '-keyout', f'{name}.key', '-out', f'{name}.csr', '-subj', f'/CN={name}')
            ext = ('basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nauthorityKeyIdentifier=keyid\n' + ('subjectAltName=IP:127.0.0.1\nextendedKeyUsage=serverAuth\n' if name == 'server' else 'extendedKeyUsage=clientAuth\n'))
            (cls.root / 'ext').write_text(ext)
            openssl('x509','-req','-in',f'{name}.csr','-CA','ca.crt','-CAkey','ca.key','-CAcreateserial','-out',f'{name}.crt','-days','1','-extfile','ext')
        cls.registry = {}
        for name, tenant in [('alice','a'),('bob','b')]:
            fp = hashlib.sha256(ssl.PEM_cert_to_DER_cert((cls.root/f'{name}.crt').read_text())).hexdigest()
            cls.registry[fp] = {'tenant':tenant, 'user':name}
        cls.socket_path = cls.root/'bridge.sock'
        cls.listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
        cls.listener.bind(str(cls.socket_path)); os.chmod(cls.socket_path,0o600)
        cls.listener.listen(); cls.listener.settimeout(.2)
        cls.calls=[]; cls.stop=threading.Event()
        def backend():
            while not cls.stop.is_set():
                try: c,_=cls.listener.accept()
                except TimeoutError: continue
                except OSError: break
                with c:
                    try:
                        request=read_frame(c, 5)
                        cls.calls.append(request)
                        body=json.dumps({'subject':request['subject']['tenant'],'path':request['path']}).encode()
                        write_frame(c,{'status':200,'body_b64':base64.b64encode(body).decode()},5)
                    except (OSError,ValueError): pass
        cls.backend=threading.Thread(target=backend,daemon=True);cls.backend.start()
        static=cls.root/'www';static.mkdir()
        for path in ASSETS:(static/path[1:]).write_text('<h1>Builder</h1>' if path=='/index.html' else '')
        cls.ingress=Ingress(('127.0.0.1',0), GatewayConfig(
            origin='https://127.0.0.1', cert=cls.root/'server.crt', key=cls.root/'server.key',
            ca=cls.root/'ca.crt', identities=cls.registry, bridge=cls.socket_path, assets=static))
        cls.thread=threading.Thread(target=cls.ingress.serve_forever,daemon=True);cls.thread.start()
    @classmethod
    def tearDownClass(cls):
        cls.ingress.shutdown();cls.ingress.server_close();cls.thread.join()
        cls.stop.set();cls.listener.close();cls.backend.join(2);cls.tmp.cleanup()
    def context(self,name='alice'):
        c=ssl.create_default_context(cafile=str(self.root/'ca.crt'))
        if name:c.load_cert_chain(self.root/f'{name}.crt',self.root/f'{name}.key')
        return c
    def request(self, method='GET', path='/v1/projects/demo', name='alice', body=None, headers=None):
        with closing(http.client.HTTPSConnection('127.0.0.1',self.ingress.server_address[1],context=self.context(name),timeout=4)) as conn:
            conn.request(method,path,body=body,headers={'Host':'127.0.0.1','Authorization':'Bearer example-token',**(headers or {})})
            res=conn.getresponse();return res.status,res.read(),dict(res.headers)
    def raw(self, request):
        with socket.create_connection(self.ingress.server_address,timeout=4) as s:
            with self.context().wrap_socket(s,server_hostname='127.0.0.1') as t:
                t.sendall(request);return t.recv(4096)
    def test_certificate_identity_is_bound_and_isolated(self):
        for name,tenant in [('alice','a'),('bob','b')]:
            status,body,_=self.request(name=name)
            self.assertEqual(status,200);self.assertEqual(json.loads(body)['subject'],tenant)
        self.assertEqual(self.calls[-1]['subject']['user'],'bob')
    def test_no_cert_and_unknown_cert(self):
        with self.assertRaises((ssl.SSLError,ConnectionError,http.client.HTTPException)):
            self.request(name=None)
        self.assertEqual(self.request(name='unknown')[0],403)
    def test_spoofed_identity_rejected_without_backend(self):
        n=len(self.calls)
        self.assertEqual(self.request(headers={'X-Tenant':'b'})[0],400)
        self.assertEqual(self.request(headers={'X-Forwarded-User':'bob'})[0],400)
        self.assertEqual(len(self.calls),n)
    def test_mutations_require_exact_origin_and_json(self):
        n=len(self.calls);body=b'{"hello":1}'
        self.assertEqual(self.request('PUT',body=body,headers={'Content-Type':'application/json'})[0],403)
        self.assertEqual(self.request('PUT',body=body,headers={'Origin':'https://evil.example','Content-Type':'application/json'})[0],403)
        self.assertEqual(self.request('PUT',body=body,headers={'Origin':'https://127.0.0.1','Content-Type':'text/plain'})[0],415)
        self.assertEqual(len(self.calls),n)
        self.assertEqual(self.request('PUT',body=body,headers={'Origin':'https://127.0.0.1','Content-Type':'application/json'})[0],200)
        self.assertEqual(base64.b64decode(self.calls[-1]['body_b64']),body)
    def test_framing_and_size_limits(self):
        n=len(self.calls)
        base=b'PUT /v1/projects/demo HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer example-token\r\nOrigin: https://127.0.0.1\r\nContent-Type: application/json\r\n'
        for h,status in [(b'Content-Length: 0\r\ncontent-length: 0\r\n',400),(b'Transfer-Encoding: chunked\r\n',400),(b'Content-Length: '+str(MAX_BODY+1).encode()+b'\r\n',413)]:
            self.assertIn(f' {status} '.encode(),self.raw(base+h+b'\r\n'))
        self.assertEqual(len(self.calls),n)
    def test_static_and_route_allowlist(self):
        status,body,h=self.request(path='/')
        self.assertEqual(status,200);self.assertEqual(body,b'<h1>Builder</h1>')
        self.assertEqual(h['X-Content-Type-Options'],'nosniff')
        self.assertEqual(self.request(path='/../server.key')[0],404)
        self.assertEqual(self.request(path='/v1/projects/%2e%2e')[0],404)
        self.assertEqual(self.request(path='/v1/projects/demo?tenant=b')[0],404)
    def test_missing_token_cookie_and_wrong_host(self):
        n=len(self.calls)
        self.assertEqual(self.request(headers={'Authorization':''})[0],403)
        self.assertEqual(self.request(headers={'Cookie':'session=abc'})[0],403)
        self.assertEqual(self.request(headers={'Host':'evil.example'})[0],400)
        self.assertEqual(len(self.calls),n)
    def test_untrusted_socket_permissions_denied(self):
        n=len(self.calls)
        os.chmod(self.socket_path,0o666)
        try:self.assertEqual(self.request()[0],503)
        finally:os.chmod(self.socket_path,0o600)
        self.assertEqual(len(self.calls),n)
    def test_frame_rejects_oversize_truncation_and_ambiguous_json(self):
        for raw in (struct.pack('!I',MAX_FRAME+1),struct.pack('!I',5)+b'{}',struct.pack('!I',len(b'{"a":1,"a":2}'))+b'{"a":1,"a":2}'):
            left,right=socket.socketpair()
            with left,right:
                right.sendall(raw);right.shutdown(socket.SHUT_WR)
                with self.assertRaises(ValueError):read_frame(left,1)
        for raw in (b'1e400',b'NaN',b'{"a":1,"a":2}'):
            with self.assertRaises(ValueError):load_json(raw)
    @unittest.skipUnless(sys.platform.startswith('linux') and hasattr(socket, 'SO_PEERCRED'),
                         'Linux SO_PEERCRED required')
    def test_unix_peer_pid_is_verified_before_forwarding(self):
        original = self.ingress.config.bridge_pid
        count = len(self.calls)
        try:
            self.ingress.config.bridge_pid = os.getpid() + 10000
            self.assertEqual(self.request()[0],503)
            self.assertEqual(len(self.calls),count, 'impostor must not receive bearer token')
            self.ingress.config.bridge_pid = os.getpid()
            self.assertEqual(self.request()[0],200)
            self.assertEqual(len(self.calls),count+1)
        finally:
            self.ingress.config.bridge_pid = original

    def test_private_tls_key_permissions_are_enforced(self):
        config = self.ingress.config
        key = config.key
        old = key.stat().st_mode & 0o777
        try:
            key.chmod(0o644)
            with self.assertRaises(ValueError):
                config.validate()
        finally:
            key.chmod(old)

    def test_standalone_cli_denies_public_identity_registry_before_listening(self):
        with tempfile.TemporaryDirectory() as tmp:
            registry = Path(tmp) / 'registry.json'
            registry.write_text(json.dumps(self.registry))
            registry.chmod(0o666)
            import sys
            cmd = [sys.executable, '-m', 'scripts.builder_cloud_ingress',
                   '--origin', 'https://127.0.0.1', '--port', '0',
                   '--cert', str(self.root/'server.crt'),
                   '--key', str(self.root/'server.key'),
                   '--ca', str(self.root/'ca.crt'),
                   '--registry', str(registry), '--bridge', str(self.socket_path),
                   '--assets', str(self.root/'www')]
            result = subprocess.run(cmd, capture_output=True, timeout=3)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b'private', result.stderr)

    def test_backend_unavailable_is_fail_closed(self):
        old=self.ingress.config.bridge
        self.ingress.config.bridge=self.root/'missing.sock'
        try:self.assertEqual(self.request()[0],503)
        finally:self.ingress.config.bridge=old

if __name__=='__main__':unittest.main()
