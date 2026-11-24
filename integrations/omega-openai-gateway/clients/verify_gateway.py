"""Run native Gateway against deterministic loopback provider fixtures; never call cloud."""
import argparse
import hashlib
import json
import os
import signal
import subprocess
import tempfile
import threading
import time
import unittest
from contextlib import contextmanager
from http.client import HTTPConnection
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

TOKEN = 'verification-token-not-a-real-secret-0001'


class Provider(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def send_data(self, status, content_type, data):
        self.send_response(status)
        self.send_header('Content-Type', content_type)
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        try:
            length = int(self.headers.get('Content-Length', '0'))
            if not 0 < length <= 65536:
                raise ValueError('length')
            value = json.loads(self.rfile.read(length))
            if self.path == '/v1/embeddings' and value['model'] == 'fixture-emb':
                vector = [1.0, 0.0] if 'needle' in value['input'] else [0.0, 1.0]
                output = {'model': 'fixture-emb', 'data': [{'index': 0, 'embedding': vector}]}
            elif self.path == '/v1/chat/completions' and value['model'] == 'fixture-gguf':
                if value.get('stream'):
                    self.send_data(200, 'text/event-stream', b'data: {"choices":[{"delta":{"content":"OK"}}]}\n\ndata: [DONE]\n\n')
                    return
                output = {'model':'fixture-gguf','choices':[{'message':{'content':'OK'},'finish_reason':'stop'}],
                          'usage':{'prompt_tokens':5,'completion_tokens':1}}
            else:
                raise ValueError('unexpected route')
            self.send_data(200, 'application/json', json.dumps(output).encode())
        except (KeyError, TypeError, ValueError):
            self.send_data(400, 'application/json', b'{"error":"fixture_rejected"}')


@contextmanager
def fixture(port):
    server = ThreadingHTTPServer(('127.0.0.1', port), Provider)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield server.server_address[1]
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def request(port, path, payload=None, token=TOKEN):
    conn = HTTPConnection('127.0.0.1', port, timeout=10)
    try:
        headers = {'Content-Type':'application/json'}
        if token:
            headers['Authorization'] = 'Bearer ' + token
        body = None if payload is None else json.dumps(payload, ensure_ascii=False).encode()
        conn.request('GET' if payload is None else 'POST', path, body=body, headers=headers)
        response = conn.getresponse()
        return response.status, response.read(3*1024*1024+1)
    finally:
        conn.close()


def expect(path, payload, status=200, token=TOKEN):
    actual, body = request(8788, path, payload, token)
    if actual != status:
        raise AssertionError(f'{path}: expected {status}, received {actual}')
    return body


@contextmanager
def gateway(binary, state):
    env = {k:v for k,v in os.environ.items() if not k.startswith(('OMEGA_', 'OPENAI_'))}
    env.update(OMEGA_GATEWAY_TOKEN=TOKEN, OMEGA_LOCAL_MODEL='fixture-gguf',
               OMEGA_CLOUD_DAILY_REQUESTS='0', OMEGA_LOCAL_FUNCTIONS='1', OMEGA_RAG_ENABLED='1',
               OMEGA_RAG_DB=str(state/'rag.sqlite'), OMEGA_RAG_EMBEDDING_MODEL='fixture-emb',
               OMEGA_RAG_DIMENSION='2', OMEGA_RAG_LOCAL_EMBEDDINGS='1')
    process = subprocess.Popen([str(binary)], env=env)
    try:
        deadline = time.monotonic()+15
        while time.monotonic()<deadline:
            if process.poll() is not None:
                raise RuntimeError('gateway exited before health check')
            try:
                if request(8788, '/healthz')[0] == 204:
                    break
            except OSError:
                pass
            time.sleep(0.1)
        else:
            raise RuntimeError('gateway health check timed out')
        yield
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGINT)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)


def run_native(binary):
    from omega_ingest import Gateway, ingest
    binary = Path(binary).resolve(strict=True)
    with tempfile.TemporaryDirectory() as directory, fixture(8082), fixture(8083):
        state = Path(directory)
        doc = state/'fixture.txt'
        doc.write_text('needle żółć', encoding='utf-8')
        query = {'query':'needle','embedding':[1.0,0.0],'seeds':1,'limit':2,'body_bytes':100}
        with gateway(binary, state):
            expect('/v1/generate', {'input':'hello'}, 401, token=None)
            expect('/v1/generate', {'input':'hello','model':'astra','allow_cloud':True}, 403)
            expect('/v1/mcp/start', {'input':'hello','model':'astra','profile':'fixture','allow_cloud':True}, 403)
            expect('/v1/audio/webrtc/start', {'sdp':'v=0','allow_cloud':True}, 403)
            payload = {'model':'omega-local','messages':[{'role':'user','content':'hello'}]}
            assert json.loads(expect('/v1/chat/completions', payload))['choices'][0]['message']['content']=='OK'
            assert b'data: [DONE]' in expect('/v1/chat/completions', {**payload,'stream':True})
            assert ingest([doc], 0, Gateway(TOKEN))['revision']==1
            result=json.loads(expect('/v1/rag/retrieve',query))
            assert result['revision']==1 and result['evidence'][0]['body']=='needle żółć'
            assert result['evidence'][0]['sha256']==list(hashlib.sha256(doc.read_bytes()).digest())
            expect('/v1/rag/index', {'expected_revision':0,'sources':[],'edges':[]}, 409)
        with gateway(binary,state):
            assert json.loads(expect('/v1/rag/retrieve',query))['revision']==1
    print('PASS: native local-only HTTP, SSE, cloud-denial, RAG and restart contract')


class FixtureTests(unittest.TestCase):
    def test_embedding(self):
        with fixture(0) as port:
            status, body=request(port,'/v1/embeddings',{'model':'fixture-emb','input':'needle'})
            self.assertEqual(status,200)
            self.assertEqual(json.loads(body)['data'][0]['embedding'],[1,0])

    def test_sse(self):
        with fixture(0) as port:
            status, body=request(port,'/v1/chat/completions',{'model':'fixture-gguf','stream':True})
            self.assertEqual(status,200)
            self.assertTrue(body.endswith(b'data: [DONE]\n\n'))

    def test_reject_unknown_model(self):
        with fixture(0) as port:
            self.assertEqual(request(port,'/v1/chat/completions',{'model':'cloud'})[0],400)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    mode=parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--binary')
    mode.add_argument('--self-test',action='store_true')
    args=parser.parse_args()
    if args.self_test:
        unittest.main(argv=['verify_gateway.py'],verbosity=2)
    else:
        run_native(args.binary)
