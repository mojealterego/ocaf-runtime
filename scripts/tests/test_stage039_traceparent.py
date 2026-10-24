"""Authenticated broker lease identity binds distinct guest traceparents."""
import sys
from pathlib import Path
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_traceparent as traceparent


class TraceParentTests(unittest.TestCase):
    def test_lease_manifest_and_tenant_are_all_bound(self):
        base=traceparent.derive("a"*32,"b"*64,"tenant-a")
        self.assertRegex(base,traceparent.TRACEPARENT)
        for request_id,digest,tenant in (("c"*32,"b"*64,"tenant-a"),
                                         ("a"*32,"d"*64,"tenant-a"),
                                         ("a"*32,"b"*64,"tenant-b")):
            self.assertNotEqual(base,traceparent.derive(request_id,digest,tenant))
        for request_id,digest,tenant in (("a"*31,"b"*64,"tenant-a"),
                                         ("a"*32,"B"*64,"tenant-a"),
                                         ("a"*32,"b"*64,"tenant-c")):
            with self.assertRaises(ValueError):
                traceparent.derive(request_id,digest,tenant)


if __name__=="__main__":
    unittest.main()
