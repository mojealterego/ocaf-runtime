"""Minimal production HTTP entrypoint. Never reports unverified E2E readiness."""
import hmac
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.request import Request, urlopen

MAX_BODY = 32768

class Handler(BaseHTTPRequestHandler):
    def send_json(self, status, data):
        raw = json.dumps(data, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Cache-Control', 'no-store')
        self.send_header('Content-Length', str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_GET(self):
        if self.path == '/health':
            self.send_json(200, {'status': 'up', 'service': 'omega-ocaf-apex', 'production_e2e_verified': False})
        elif self.path == '/ready':
            self.send_json(200, {'status': 'ready', 'inference_configured': bool(os.getenv('OPENAI_API_KEY')), 'production_e2e_verified': False})
        else:
            self.send_json(404, {'error': 'not_found'})

    def do_POST(self):
        secret = os.getenv('OMEGA_API_TOKEN', '')
        supplied = self.headers.get('Authorization', '')
        if not secret or not hmac.compare_digest(supplied, 'Bearer ' + secret):
            self.send_json(401, {'error': 'unauthorized'})
            return
        if self.path != '/v1/agents/execute':
            self.send_json(404, {'error': 'not_found'})
            return
        try:
            size = int(self.headers.get('Content-Length', '0'))
            if not 0 < size <= MAX_BODY:
                self.send_json(413, {'error': 'invalid_body_size'})
                return
            payload = json.loads(self.rfile.read(size))
            mission = payload.get('mission')
            if not isinstance(mission, str) or not 0 < len(mission) <= 4000:
                self.send_json(400, {'error': 'invalid_mission'})
                return
            key = os.getenv('OPENAI_API_KEY')
            if not key:
                self.send_json(503, {'error': 'openai_key_not_configured'})
                return
            model = os.getenv('OPENAI_MODEL', 'gpt-5.1')
            req = Request('https://api.openai.com/v1/responses', data=json.dumps({'model': model, 'input': mission, 'max_output_tokens': 400}).encode(), headers={'Authorization': 'Bearer ' + key, 'Content-Type': 'application/json'}, method='POST')
            with urlopen(req, timeout=45) as resp:
                result = json.load(resp)
            self.send_json(200, {'response_id': result.get('id'), 'model': result.get('model'), 'output': result.get('output'), 'live_inference': True, 'production_e2e_verified': False})
        except Exception as e:
            self.send_json(502, {'error': 'upstream_or_request_failed', 'type': type(e).__name__})

if __name__ == '__main__':
    ThreadingHTTPServer(('0.0.0.0', int(os.getenv('PORT', '8080'))), Handler).serve_forever()
