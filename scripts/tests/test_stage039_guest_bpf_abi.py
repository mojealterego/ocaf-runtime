"""ABI report rejects missing, duplicated and contradictory guest observations."""
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_guest_bpf_abi as abi


GOOD = (b"Linux version 6.1\nOCAF039_GUEST_PID1_BOOTED\n"
        b"OCAF039_GUEST_BPF_SOCKET_FILTER_ID=17\n"
        b"OCAF039_GUEST_BPF_ABI_BTF=1 KPROBE=0 SYSCALL=1\n"
        b"OCAF039_GUEST_OPENAT_EVENT_ID=31 COUNT=3 TGID=42 PID=42\n")


class GuestAbiEvidence(unittest.TestCase):
    def test_real_guest_marker_can_report_partial_kprobe(self):
        observed = abi.parse_guest_abi(GOOD)
        self.assertTrue(observed["btf_file_observed_in_guest"])
        self.assertFalse(observed["kprobe_events_observed_in_guest"])
        self.assertTrue(observed["bpf_syscall_observed_in_guest"])
        self.assertEqual(observed["serial_sha256"], hashlib.sha256(GOOD).hexdigest())

    def test_missing_duplicated_or_forged_abi_line_is_denied(self):
        for damaged in (GOOD.replace(b"OCAF039_GUEST_BPF_ABI_BTF=1 KPROBE=0 SYSCALL=1\n", b""),
                        GOOD + b"OCAF039_GUEST_BPF_ABI_BTF=1 KPROBE=0 SYSCALL=1\n",
                        GOOD.replace(b"KPROBE=0", b"KPROBE=2"),
                        GOOD.replace(b"SYSCALL=1", b"SYSCALL=0"),
                        GOOD.replace(b"OCAF039_GUEST_PID1_BOOTED\n", b""),
                        GOOD.replace(b"OCAF039_GUEST_OPENAT_EVENT_ID=", b"EVENT_ID="),
                        GOOD.replace(b"OCAF039_GUEST_BPF_SOCKET_FILTER_ID=", b"")):
            with self.subTest(damaged=damaged[-100:]), self.assertRaises(abi.GuestAbiDenied):
                abi.parse_guest_abi(damaged)

    def test_pinned_kernel_config_exposes_kprobe_blocker(self):
        raw = (b"CONFIG_BPF_SYSCALL=y\nCONFIG_DEBUG_INFO_BTF=y\n"
               b"# CONFIG_KPROBES is not set\n")
        pin = hashlib.sha256(raw).hexdigest()
        flags = abi.config_flags(raw, pin)
        self.assertEqual(flags, {"DEBUG_INFO_BTF": True, "KPROBES": False,
                                 "KPROBE_EVENTS": False, "BPF_SYSCALL": True})
        with self.assertRaises(abi.GuestAbiDenied):
            abi.config_flags(raw + b"CONFIG_KPROBES=y\n", pin)

    def test_two_real_guest_receipt_hashes_required_for_partial_report(self):
        config = (b"CONFIG_BPF_SYSCALL=y\nCONFIG_DEBUG_INFO_BTF=y\n"
                  b"# CONFIG_KPROBES is not set\n")
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            receipts = {}
            for vm_id, tenant in (("ocaf-" + "a" * 24, "a"),
                                  ("ocaf-" + "b" * 24, "b")):
                dest = base / "firecracker" / vm_id
                dest.mkdir(parents=True)
                (dest / "serial-console.log").write_bytes(GOOD)
                receipts[tenant] = {"vm_id": vm_id,
                                    "serial_sha256": hashlib.sha256(GOOD).hexdigest(),
                                    "guest_openat_bpf_program_id": 31,
                                    "guest_openat_event_count": 3}
            verified = {"tenant_receipts": receipts}
            with mock.patch.object(abi.guest, "verify_brokered_pair", return_value=verified):
                result = abi.build_report({}, base, {"a": "a" * 64, "b": "b" * 64},
                                          config, hashlib.sha256(config).hexdigest())
                self.assertEqual(result["status"],
                                 "GUEST_BPF_ABI_PARTIAL_KPROBE_OR_BTF_UNAVAILABLE")
                self.assertFalse(result["stage_039_22_complete"])
                receipts["b"]["serial_sha256"] = "0" * 64
                with self.assertRaises(abi.GuestAbiDenied):
                    abi.build_report({}, base, {"a": "a" * 64, "b": "b" * 64},
                                     config, hashlib.sha256(config).hexdigest())


if __name__ == "__main__":
    unittest.main()
