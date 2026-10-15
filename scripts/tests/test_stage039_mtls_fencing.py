"""0.39: mTLS peer identity, durable fencing and fail-closed RPC contracts."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_mtls_fencing as fencing

class TestFencing(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.db=Path(self.tmp.name)/"fences.sqlite"
        self.store=fencing.FenceLedger(self.db)
        self.addCleanup(self.store.close)
    def request(self,action="claim",epoch=1,token="a"*32,ttl=6):
        return {"version":1,"action":action,"tenant_id":"tenant-a",
                "lease_id":token,"epoch":epoch,"ttl_seconds":ttl}

    def test_unknown_duplicate_and_cross_tenant_json_denied(self):
        self.assertEqual(fencing.strict_request(json.dumps(self.request()).encode()),self.request())
        for raw in (b'{"version":1,"version":1}',b'{"version":1}',b'bad',
                    json.dumps({**self.request(),"extra":"value"}).encode(),
                    json.dumps({**self.request(),"tenant_id":"tenant-b"}).encode(),
                    json.dumps({**self.request(),"epoch":True}).encode()):
            with self.subTest(raw=raw),self.assertRaises(fencing.FenceError):
                fencing.strict_request(raw)

    def test_fencing_persists_after_close_and_reopen(self):
        a=self.store.apply(self.request(),now=100)
        self.assertEqual(a["status"],"LEASE_GRANTED")
        self.store.close()
        self.store=fencing.FenceLedger(self.db)
        with self.assertRaises(fencing.FenceError):
            self.store.apply(self.request(),now=107)
        self.assertEqual(self.store.apply(self.request(epoch=2,token="b"*32),now=107)["epoch"],2)
        with self.assertRaises(fencing.FenceError):
            self.store.apply(self.request(epoch=1),now=200)

    def test_partition_is_not_lease_authorization(self):
        self.store.apply(self.request(),now=100)
        self.assertFalse(fencing.safe_to_write(False,100,110))
        self.assertFalse(fencing.safe_to_write(True,111,110))
        self.assertTrue(fencing.safe_to_write(True,109,110))
        self.assertFalse(fencing.safe_to_write(True,99,110))

    def test_no_double_primary_or_stale_heartbeat(self):
        self.store.apply(self.request(),now=100)
        with self.assertRaises(fencing.FenceError):
            self.store.apply(self.request(epoch=2,token="b"*32),now=102)
        with self.assertRaises(fencing.FenceError):
            self.store.apply(self.request(action="renew",epoch=1,token="b"*32),now=101)
        self.assertEqual(self.store.apply(self.request(action="renew"),now=102)["status"],"LEASE_RENEWED")
        with self.assertRaises(fencing.FenceError):
            self.store.apply(self.request(action="renew"),now=200)
        self.assertEqual(self.store.apply(self.request(epoch=2,token="b"*32),now=200)["status"],"LEASE_GRANTED")
        with self.assertRaises(fencing.FenceError):
            self.store.apply(self.request(action="renew"),now=201)

    def test_authenticated_peer_dns_san_only(self):
        cert={"subjectAltName":(("DNS","ocaf039-tenant-a.client"),)}
        self.assertTrue(fencing.peer_authorized(cert))
        for bad in ({},{"subject":((("commonName","ocaf039-tenant-a.client"),),)},
                    {"subjectAltName":(("DNS","ocaf039-tenant-b.client"),)},
                    {"subjectAltName":(("DNS","ocaf039-tenant-a.client"),
                                      ("DNS","ocaf039-tenant-b.client"))}):
            with self.subTest(cert=bad), self.assertFalse(fencing.peer_authorized(bad)):
                pass
