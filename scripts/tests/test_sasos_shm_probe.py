"""Linux memfd interprocess sharing contract, not a Zenoh SHM E2E claim."""
import json
from pathlib import Path
import os
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from sasos_shm_probe import ShmProbeError, run_probe


@unittest.skipUnless(sys.platform == "linux" and hasattr(os, "memfd_create"),
                     "Linux memfd required")
class ShmContractTests(unittest.TestCase):
    def test_real_two_process_sharing_and_tamper_denial(self):
        report = run_probe()
        self.assertEqual(report["status"], "LINUX_MEMFD_TWO_PROCESS_PROBE")
        self.assertTrue(report["separate_process_verified"])
        self.assertTrue(report["tampered_payload_rejected"])
        self.assertTrue(report["same_host_only"])
        self.assertFalse(report["zenoh_shm_e2e_verified"])
        self.assertFalse(report["cross_host_zero_copy_verified"])

    def test_bogus_fd_is_denied(self):
        with self.assertRaises(ShmProbeError):
            run_probe(payload_size=0)


if __name__ == "__main__":
    unittest.main()
