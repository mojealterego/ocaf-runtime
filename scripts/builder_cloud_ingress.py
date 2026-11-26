#!/usr/bin/env python3
"""OCAF Builder mTLS ingress; fixed editor/API routes and private Unix bridge.

Never derive the authenticated subject from public HTTP, and never log secrets.
The Rust bridge must independently validate the opaque OAuth token and scope.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import http
import json
import math
import ipaddress
import os
from pathlib import Path
import re
import signal
import socket
import socketserver
import ssl
import stat
import struct
import threading
import time
from dataclasses import dataclass
from urllib.parse import urlsplit

from scripts.builder_cloud_peer import verify_bridge_peer

MAX_BODY = 128 * 1024
MAX_HEADERS = 16 * 1024
MAX_FRAME = 256 * 1024
MAX_WORKERS = 32
TIMEOUT = 8.0
IDENTIFIER = re.compile(r'[A-Za-z0-9_.-]{1,64}\Z')
PROJECT = re.compile(r'/v1/projects/([A-Za-z0-9_.-]{1,64})(/history)?\Z')
HEADER = re.compile(r'[A-Za-z0-9-]+\Z')
ASSETS = {
    '/index.html': 'text/html; charset=utf-8', '/style.css': 'text/css; charset=utf-8',
    '/manifest.webmanifest': 'application/manifest+json', '/sw.js': 'text/javascript; charset=utf-8',
    '/agent-catalog.json': 'application/json; charset=utf-8',
    **{'/'+name+'.mjs': 'text/javascript; charset=utf-8' for name in (
        'app','editor','strict-json','cloud','preview-gate','i18n','credentials',
        'models','integrations-ui','runner-client','agent-catalog','agent-catalog-ui')},
}


class Rejected(Exception):
    def __init__(self, status: int):
        self.status = status


def identifier(value):
    return isinstance(value, str) and IDENTIFIER.fullmatch(value) and value not in ('.','..')


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON key')
        result[key] = value
    return result


def invalid_constant(_):
    raise ValueError('invalid JSON number')


def finite_float(value):
    result = float(value)
    if not math.isfinite(result):
        raise ValueError("invalid JSON number")
    return result


def load_json(raw):
    return json.loads(raw, object_pairs_hook=unique_object, parse_constant=invalid_constant, parse_float=finite_float)


def remaining(deadline):
    value = deadline - time.monotonic()
    if value <= 0:
        raise TimeoutError('deadline exceeded')
    return value


def recv_exact(sock, n, deadline):
    data = bytearray()
    while len(data) < n:
        sock.settimeout(remaining(deadline))
        part = sock.recv(min(65536, n-len(data)))
        if not part:
            raise ValueError('truncated frame')
        data.extend(part)
    return bytes(data)


def read_frame(sock, timeout=TIMEOUT):
    deadline = time.monotonic() + timeout
    size = struct.unpack('!I', recv_exact(sock, 4, deadline))[0]
    if not 1 <= size <= MAX_FRAME:
        raise ValueError('invalid frame size')
    return load_json(recv_exact(sock, size, deadline))


def write_frame(sock, value, timeout=TIMEOUT):
    data = json.dumps(value, ensure_ascii=True, allow_nan=False, separators=(',', ':')).encode('ascii')
    if not 1 <= len(data) <= MAX_FRAME:
        raise ValueError('invalid frame size')
    sock.settimeout(timeout)
    sock.sendall(struct.pack('!I', len(data)) + data)


def private_regular_file(path, max_bytes=2*1024*1024):
    parent = path.parent
    if not path.is_absolute() or parent.resolve(strict=True) != parent:
        raise ValueError('private file must have a canonical absolute parent')
    directory, file = parent.stat(), path.lstat()
    if (not stat.S_ISDIR(directory.st_mode) or directory.st_uid != os.geteuid()
            or directory.st_mode & 0o077 or not stat.S_ISREG(file.st_mode)
            or file.st_uid != os.geteuid() or file.st_mode & 0o077
            or file.st_nlink != 1 or file.st_size > max_bytes):
        raise ValueError('invalid private file ownership, permissions or size')


def private_bridge(path):
    # Same-UID trust boundary. Refuse a writable parent and symlink socket.
    parent = path.parent
    if parent.resolve(strict=True) != parent.absolute():
        raise ValueError('bridge parent must be canonical')
    p, s = parent.stat(), path.lstat()
    if (not stat.S_ISDIR(p.st_mode) or p.st_uid != os.geteuid() or p.st_mode & 0o077
            or not stat.S_ISSOCK(s.st_mode) or s.st_uid != os.geteuid() or s.st_mode & 0o077):
        raise ValueError('bridge must be private to the service UID')


@dataclass
class GatewayConfig:
    origin: str
    cert: Path
    key: Path
    ca: Path
    identities: dict
    bridge: Path
    assets: Path
    bridge_pid: int | None = None

    def validate(self):
        parsed = urlsplit(self.origin)
        if (parsed.scheme != 'https' or not parsed.hostname or parsed.username or parsed.password
                or parsed.path or parsed.query or parsed.fragment or parsed.netloc != self.origin[8:]
                or any(ord(ch) < 33 or ord(ch) > 126 for ch in self.origin)
                or parsed.netloc.lower() != parsed.netloc):
            raise ValueError('origin must be exact lowercase HTTPS authority')
        if parsed.port == 0 or parsed.netloc.endswith(':'):
            raise ValueError('invalid port')
        host = parsed.hostname
        if ':' in host:
            if '%' in host:
                raise ValueError('invalid IPv6 host')
            ipaddress.IPv6Address(host)
        elif len(host)>253 or not all(re.fullmatch(r'[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?', p) for p in host.split('.')):
            raise ValueError('invalid DNS host')
        if not isinstance(self.identities, dict) or not 1 <= len(self.identities) <= 10000:
            raise ValueError('invalid certificate registry')
        for digest, record in self.identities.items():
            if (not re.fullmatch('[0-9a-f]{64}', digest) or not isinstance(record, dict)
                    or set(record) != {'tenant','user'} or not all(identifier(v) for v in record.values())):
                raise ValueError('invalid certificate identity')
        if self.bridge_pid is not None and (type(self.bridge_pid) is not int or self.bridge_pid <= 0):
            raise ValueError('invalid bridge PID')
        private_regular_file(self.key)
        private_bridge(self.bridge)
        return parsed.netloc


def parse_request(sock, deadline):
    data = bytearray()
    while b'\r\n\r\n' not in data:
        sock.settimeout(remaining(deadline))
        part = sock.recv(4096)
        if not part:
            raise Rejected(400)
        data.extend(part)
        if len(data) > MAX_HEADERS + 4096:
            raise Rejected(431)
    header, body = bytes(data).split(b'\r\n\r\n', 1)
    if len(header) > MAX_HEADERS:
        raise Rejected(431)
    try:
        lines = header.decode('ascii').split('\r\n')
        method, path, version = lines.pop(0).split(' ')
    except ValueError:
        raise Rejected(400) from None
    if version != 'HTTP/1.1' or method not in ('GET','POST','PUT'):
        raise Rejected(405)
    if len(path) > 2048:
        raise Rejected(414)
    if len(lines) > 32:
        raise Rejected(431)
    headers = {}
    for line in lines:
        key, sep, value = line.partition(':')
        key = key.lower()
        value = value.strip(' ')
        if (not sep or not HEADER.fullmatch(key) or key in headers or len(value) > 4096
                or any(ord(ch) < 32 or ord(ch) > 126 for ch in value)
                or key.startswith(('x-','forwarded','proxy-'))):
            raise Rejected(400)
        headers[key] = value
    if 'transfer-encoding' in headers or 'expect' in headers or 'upgrade' in headers:
        raise Rejected(400)
    if 'cookie' in headers:
        raise Rejected(403)
    size_text = headers.get('content-length', '0')
    if not re.fullmatch('[0-9]{1,10}', size_text):
        raise Rejected(400)
    size = int(size_text)
    if size > MAX_BODY:
        raise Rejected(413)
    if len(body) > size or (method == 'GET' and size):
        raise Rejected(400)
    if method != 'GET' and 'content-length' not in headers:
        raise Rejected(411)
    body += recv_exact(sock, size-len(body), deadline)
    return method, path, headers, body


class Ingress(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = False
    request_queue_size = MAX_WORKERS

    def __init__(self, address, config):
        self.config = config
        self.authority = config.validate()
        self.identities = {k: dict(v) for k,v in config.identities.items()}
        self.context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.context.minimum_version = ssl.TLSVersion.TLSv1_2
        self.context.verify_mode = ssl.CERT_REQUIRED
        self.context.verify_flags |= ssl.VERIFY_X509_STRICT
        self.context.load_verify_locations(cafile=str(config.ca))
        self.context.load_cert_chain(str(config.cert),str(config.key))
        self.context.options |= ssl.OP_NO_COMPRESSION
        self.workers = threading.BoundedSemaphore(MAX_WORKERS)
        self.assets = {}
        for path, mime in ASSETS.items():
            asset = config.assets / path[1:]
            if asset.is_symlink() or not asset.is_file() or asset.stat().st_size > 2*1024*1024:
                raise ValueError('invalid static asset')
            self.assets[path] = (mime, asset.read_bytes())
        if '/index.html' not in self.assets:
            raise ValueError('editor index missing')
        self.assets['/'] = self.assets['/index.html']
        super().__init__(address, _Worker)

    def process_request(self, request, client_address):
        if not self.workers.acquire(blocking=False):
            self.shutdown_request(request)
            return
        try:
            super().process_request(request, client_address)
        except BaseException:
            self.workers.release()
            self.shutdown_request(request)
            raise

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            self.workers.release()

    def handle_error(self, request, client_address):
        # Do not expose tokens, subject names, paths or tracebacks to logs.
        pass

    def dispatch(self, method, path, headers, body, identity):
        if headers.get('host') != self.authority:
            raise Rejected(400)
        if path in self.assets:
            if method != 'GET':
                raise Rejected(405)
            return 200, *self.assets[path]
        match = PROJECT.fullmatch(path)
        if not match or not identifier(match.group(1)):
            raise Rejected(404)
        if path == '/v1/projects/preview':
            if method != 'POST':
                raise Rejected(405)
        elif match.group(2):
            if method != 'GET':
                raise Rejected(405)
        elif method not in ('GET','PUT'):
            raise Rejected(405)
        if headers.get('origin', self.config.origin if method == 'GET' else '') != self.config.origin:
            raise Rejected(403)
        if not re.fullmatch(r'Bearer [!-~]{1,4089}', headers.get('authorization','')):
            raise Rejected(403)
        if method != 'GET' and headers.get('content-type') != 'application/json':
            raise Rejected(415)
        envelope = {
            'subject': {**identity,'authenticated_at':int(time.time()),'assurance_level':1},
            'method':method,'path':path,
            'headers':[[k,v] for k,v in headers.items() if k in ('host','authorization','origin','content-type','content-length')],
            'body_b64':base64.b64encode(body).decode('ascii'),
        }
        try:
            private_bridge(self.config.bridge)
            with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as peer:
                deadline=time.monotonic()+TIMEOUT
                peer.settimeout(remaining(deadline));peer.connect(str(self.config.bridge))
                verify_bridge_peer(peer, expected_pid=self.config.bridge_pid)
                write_frame(peer,envelope,remaining(deadline))
                response=read_frame(peer,remaining(deadline))
            if (not isinstance(response,dict) or set(response) != {'status','body_b64'}
                    or type(response['status']) is not int or response['status'] not in (200,400,403,404,405,409,411,413,414,415,422,429,431,503)):
                raise ValueError('invalid backend response')
            raw=base64.b64decode(response['body_b64'],validate=True)
            if len(raw)>MAX_BODY:
                raise ValueError('response limit')
            load_json(raw)
            return response['status'],'application/json',raw
        except (OSError,ValueError,TypeError,RecursionError):
            raise Rejected(503) from None


class _Worker(socketserver.BaseRequestHandler):
    def handle(self):
        deadline = time.monotonic()+TIMEOUT
        try:
            self.request.settimeout(remaining(deadline))
            with self.server.context.wrap_socket(self.request, server_side=True) as tls:
                try:
                    cert=tls.getpeercert(binary_form=True)
                    identity=self.server.identities.get(hashlib.sha256(cert).hexdigest()) if cert else None
                    request=parse_request(tls,deadline)
                    if identity is None:
                        raise Rejected(403)
                    status,ctype,body=self.server.dispatch(*request,identity)
                except Rejected as exc:
                    status=exc.status;ctype='application/json';body=json.dumps({'error':f'http_{status}'}).encode()
                except (ValueError,TimeoutError):
                    status=400;ctype='application/json';body=b'{"error":"http_400"}'
                reason=http.HTTPStatus(status).phrase
                response=(f'HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {len(body)}\r\n'
                          'Connection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n'
                          'Referrer-Policy: no-referrer\r\nX-Frame-Options: DENY\r\n\r\n').encode('ascii')
                tls.settimeout(2);tls.sendall(response+body)
        except (OSError,ssl.SSLError):
            return


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for key in ('origin','cert','key','ca','registry','bridge','assets'):
        parser.add_argument('--'+key,required=True,type=str if key=='origin' else Path)
    parser.add_argument('--bind',default='127.0.0.1')
    parser.add_argument('--port',type=int,default=8443)
    parser.add_argument('--bridge-pid',type=int,default=None,
                        help='pin Linux Unix peer PID (required for high-assurance standalone deployments)')
    args=parser.parse_args()
    try:
        private_regular_file(args.registry)
        private_regular_file(args.key)
    except (OSError, ValueError):
        parser.error('invalid private registry or TLS key')
    config=GatewayConfig(args.origin,args.cert,args.key,args.ca,load_json(args.registry.read_bytes()),args.bridge,args.assets,
                         bridge_pid=args.bridge_pid)
    with Ingress((args.bind,args.port),config) as server:
        def stop(*_):threading.Thread(target=server.shutdown,daemon=True).start()
        signal.signal(signal.SIGINT,stop);signal.signal(signal.SIGTERM,stop)
        server.serve_forever()


if __name__=='__main__':main()
