"""Explicit local-only UTF-8 GraphRAG ingestion client for Termux/desktop."""
import argparse
import hashlib
import json
import math
import os
import re
import stat
import sys
from http.client import HTTPConnection, HTTPException
from pathlib import Path

MAX_TOTAL = 1024 * 1024
MAX_SNAPSHOT = 3 * 1024 * 1024


class Gateway:
    def __init__(self, token):
        if not isinstance(token, str) or not re.fullmatch(r'[!-~]{32,512}', token):
            raise ValueError('invalid gateway token')
        self._token = token
        self._profile = None

    def _post(self, endpoint, payload):
        if endpoint not in ('/v1/rag/embed', '/v1/rag/index', '/v1/rag/retrieve'):
            raise ValueError('invalid endpoint')
        data = json.dumps(payload, ensure_ascii=False, allow_nan=False, separators=(',', ':')).encode()
        if len(data) > MAX_SNAPSHOT:
            raise ValueError('request size limit')
        conn = HTTPConnection('127.0.0.1', 8788, timeout=120)
        try:
            conn.request('POST', endpoint, body=data, headers={
                'Content-Type': 'application/json', 'Authorization': 'Bearer ' + self._token})
            response = conn.getresponse()
            if response.status != 200:
                raise ValueError(f'gateway rejected request: HTTP {response.status}')
            body = response.read(MAX_SNAPSHOT + 1)
            if len(body) > MAX_SNAPSHOT:
                raise ValueError('response size limit')
            result = json.loads(body)
            if not isinstance(result, dict):
                raise ValueError('invalid gateway response')
            return result
        finally:
            conn.close()

    def embed(self, text):
        result = self._post('/v1/rag/embed', {'input': text})
        model, vector = result.get('model'), result.get('embedding')
        if not isinstance(model, str) or not model or not isinstance(vector, list) or not 1 <= len(vector) <= 256:
            raise ValueError('invalid embedding response')
        if any(type(v) not in (int, float) or abs(v) > 3.4028235e38 or not math.isfinite(v) for v in vector):
            raise ValueError('invalid embedding values')
        if sum(float(v)**2 for v in vector) <= 1e-24:
            raise ValueError('zero embedding')
        profile = (model, len(vector))
        if self._profile is not None and profile != self._profile:
            raise ValueError('embedding profile changed')
        self._profile = profile
        return [float(v) for v in vector]

    def replace(self, snapshot):
        result = self._post('/v1/rag/index', snapshot)
        revision = result.get('revision')
        if type(revision) is not int or revision != snapshot['expected_revision'] + 1:
            raise ValueError('unexpected snapshot revision')
        return revision

    def retrieve(self, text):
        return self._post('/v1/rag/retrieve', {'query': text, 'embedding': self.embed(text),
            'seeds': 2, 'limit': 6, 'body_bytes': 16384})


def chunks(text, limit=4096):
    if not isinstance(text, str) or type(limit) is not int or not 4 <= limit <= 4096:
        raise ValueError('invalid chunk parameters')
    part, size = [], 0
    for char in text:
        n = len(char.encode('utf-8'))
        if size + n > limit:
            yield ''.join(part)
            part, size = [], 0
        part.append(char)
        size += n
    if part:
        yield ''.join(part)


def read_file(path, remaining):
    flags = os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_NONBLOCK', 0)
    fd = os.open(path, flags)
    with os.fdopen(fd, 'rb') as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size > remaining:
            raise ValueError('invalid document or total size limit')
        data = stream.read(remaining + 1)
        after = os.fstat(stream.fileno())
        if len(data) > remaining or (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
            raise ValueError('document changed or total size limit')
    return data.decode('utf-8'), len(data)


def build_snapshot(paths, revision, gateway):
    if type(revision) is not int or not 0 <= revision < 2**63 - 1 or not 1 <= len(paths) <= 32:
        raise ValueError('invalid revision or file count')
    prepared, edges, manifest, remaining = [], [], {}, MAX_TOTAL
    seen = set()
    for i, raw in enumerate(paths):
        path = Path(raw)
        if path.is_symlink() or path.resolve() in seen:
            raise ValueError('symlink or duplicate document')
        seen.add(path.resolve())
        text, size = read_file(path, remaining)
        remaining -= size
        if not text.strip():
            raise ValueError('empty document')
        previous = None
        for j, body in enumerate(chunks(text)):
            if not body.strip():
                continue
            if len(prepared) >= 256:
                raise ValueError('chunk count limit')
            source_id = f'doc-{i:03}-{j:03}'
            prepared.append((source_id, body))
            manifest[source_id] = str(path)
            if previous is not None:
                edges.append({'from': previous, 'to': source_id})
            previous = source_id
    sources = [{'id': sid, 'body': body, 'embedding': gateway.embed(body),
                'sha256': list(hashlib.sha256(body.encode()).digest())} for sid, body in prepared]
    return {'expected_revision': revision, 'sources': sources, 'edges': edges}, manifest


def ingest(paths, revision, gateway):
    snapshot, manifest = build_snapshot(paths, revision, gateway)
    return {'revision': gateway.replace(snapshot), 'sources': manifest}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    write = sub.add_parser('ingest', help='replace entire index; explicitly supply current revision')
    write.add_argument('--revision', type=int, required=True)
    write.add_argument('files', nargs='+', type=Path)
    search = sub.add_parser('query')
    search.add_argument('text')
    args = parser.parse_args()
    try:
        gateway = Gateway(os.environ.get('OMEGA_GATEWAY_TOKEN'))
        result = ingest(args.files, args.revision, gateway) if args.command == 'ingest' else gateway.retrieve(args.text)
        print(json.dumps(result, ensure_ascii=False, indent=2))
    except (ValueError, OSError, HTTPException) as exc:
        # Avoid printing exceptions that may include request bodies or headers.
        print(f'OMEGA ingestion failed ({type(exc).__name__})', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
