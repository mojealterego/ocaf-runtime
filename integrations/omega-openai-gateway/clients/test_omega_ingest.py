import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock

from omega_ingest import Gateway, build_snapshot, chunks, ingest


class FakeGateway:
    def __init__(self):
        self.calls = []
        self.snapshot = None

    def embed(self, text):
        self.calls.append(text)
        return [1.0, 0.0]

    def replace(self, snapshot):
        self.snapshot = snapshot
        return snapshot['expected_revision'] + 1


class IngestTests(unittest.TestCase):
    def test_unicode_chunks_preserve_text(self):
        text = 'żółć🙂' * 1000
        parts = list(chunks(text, 4096))
        self.assertEqual(''.join(parts), text)
        self.assertTrue(all(len(p.encode()) <= 4096 for p in parts))

    def test_snapshot_has_provenance_and_edges(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'tekst.txt'
            path.write_text('żółć' * 1200, encoding='utf-8')
            gateway = FakeGateway()
            snapshot, manifest = build_snapshot([path], 7, gateway)
            self.assertEqual(snapshot['expected_revision'], 7)
            self.assertGreater(len(snapshot['sources']), 1)
            self.assertEqual(len(snapshot['edges']), len(snapshot['sources']) - 1)
            for item in snapshot['sources']:
                self.assertEqual(bytes(item['sha256']), hashlib.sha256(item['body'].encode()).digest())
                self.assertEqual(manifest[item['id']], str(path))
            self.assertEqual(gateway.calls, [x['body'] for x in snapshot['sources']])
            self.assertIsNone(gateway.snapshot)
            self.assertEqual(ingest([path], 7, gateway)['revision'], 8)

    def test_invalid_inputs_never_publish(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'invalid.txt'
            path.write_bytes(b'\xff')
            g = FakeGateway()
            with self.assertRaises(ValueError):
                ingest([path], 0, g)
            self.assertIsNone(g.snapshot)
            self.assertEqual(g.calls, [])
            with self.assertRaises(ValueError):
                ingest([], 0, g)
            with self.assertRaises(ValueError):
                ingest([path], -1, g)

    def test_http_is_fixed_loopback_and_closes_connection(self):
        g = Gateway('x' * 32)
        with patch('omega_ingest.HTTPConnection') as conn:
            response = MagicMock(status=200)
            response.read.return_value = json.dumps({'model':'local-emb','embedding':[1.0,0.0]}).encode()
            conn.return_value.getresponse.return_value = response
            self.assertEqual(g.embed('żółć'), [1.0, 0.0])
            conn.assert_called_once_with('127.0.0.1', 8788, timeout=120)
            args, kwargs = conn.return_value.request.call_args
            self.assertEqual(args, ('POST', '/v1/rag/embed'))
            self.assertEqual(json.loads(kwargs['body']), {'input':'żółć'})
            conn.return_value.close.assert_called_once()

    def test_nonfinite_and_dimension_changes_fail_closed(self):
        g = Gateway('x' * 32)
        with patch.object(g, '_post', side_effect=[{'model':'emb','embedding':[1,0]}, {'model':'emb','embedding':[1,0,0]}]):
            g.embed('a')
            with self.assertRaises(ValueError):
                g.embed('b')
        with patch.object(g, '_post', return_value={'model':'emb','embedding':[float('nan'),0]}):
            with self.assertRaises(ValueError):
                g.embed('c')


if __name__ == '__main__':
    unittest.main()
