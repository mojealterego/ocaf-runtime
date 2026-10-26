import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_guest_kprobe_pair as pair


class GuestKprobePairMarkerTest(unittest.TestCase):
    def setUp(self):
        self.nonce = "a" * 32
        self.serial = (
            b"OCAF039_GUEST_BPF_ABI_BTF=1 KPROBE=1 SYSCALL=1\r\n"
            + b"OCAF039_GUEST_KPROBE_INTEGRATED_ID=7 COUNT=64 NONCE="
            + self.nonce.encode() + b"\r\n"
            + b"OCAF039_GUEST_OPENAT_EVENT_ID=8 COUNT=2 TGID=11 PID=11\r\n"
        )

    def test_positive_marker_binds_tenant_nonce(self):
        result = pair.parse_marker(self.serial, self.nonce)
        self.assertEqual(result["guest_kprobe_program_id"], 7)
        self.assertEqual(result["guest_kprobe_event_count"], 64)

    def test_wrong_tenant_or_duplicate_marker_is_rejected(self):
        with self.assertRaises(pair.KprobePairDenied):
            pair.parse_marker(self.serial, "b" * 32)
        with self.assertRaises(pair.KprobePairDenied):
            pair.parse_marker(self.serial + self.serial, self.nonce)

    def test_missing_openat_binding_is_rejected(self):
        with self.assertRaises(pair.KprobePairDenied):
            pair.parse_marker(self.serial.split(b"OCAF039_GUEST_OPENAT_EVENT_ID=")[0],
                              self.nonce)


if __name__ == "__main__":
    unittest.main()
