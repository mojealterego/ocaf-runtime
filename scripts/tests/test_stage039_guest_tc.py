"""0.39: actual jailed guest TC ingress eBPF isolation proof validator."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_guest_tc as tc

class GuestTcBpfContracts(unittest.TestCase):
    def test_proves_kernel_program_identity_and_packet_recovery(self):
        out=tc.validate_tc_evidence("filter protocol all pref 49152 bpf chain 0\n"
             "filter protocol all pref 49152 bpf chain 0 handle 0x1 ocaf039_drop direct-action not_in_hw id 302 tag 0123",
             tap="oc39a01234", program_id=302)
        self.assertTrue(out["tc_bpf_program_attached"])
        self.assertEqual(out["kernel_bpf_program_id"],302)
        self.assertFalse(out["aya_cgroup_guest_network_tested"])
        self.assertFalse(out["production_approved"])

    def test_rejects_false_tap_or_missing_bpf_id(self):
        good="filter protocol all pref 49152 bpf chain 0 id 302 tag 0123"
        for raw,tap,identifier in (
            (good,"eth0",302),
            ("", "oc39a01234",302),
            (good,"oc39a01234",303),
            (good.replace("bpf","notbpf"),"oc39a01234",302),
            (good,"oc39b01234",302),
        ):
            with self.subTest(raw=raw,tap=tap,identifier=identifier),self.assertRaises(tc.TcGuestError):
                tc.validate_tc_evidence(raw,tap,identifier)

    def test_bpf_drop_evidence_requires_real_attached_negative_and_recovery(self):
        self.assertTrue(tc.validate_guest_tc_phases(before_a=10,before_b=10,
           during_a_received=False,during_b=15,after_a=23,after_b=28)["guest_bpf_deny_recovery_proved"])
        for kw in (
            {"before_a":10,"before_b":10,"during_a_received":True,"during_b":15,"after_a":23,"after_b":28},
            {"before_a":10,"before_b":10,"during_a_received":False,"during_b":10,"after_a":23,"after_b":28},
            {"before_a":10,"before_b":10,"during_a_received":False,"during_b":15,"after_a":10,"after_b":28},
        ):
            with self.subTest(kw=kw),self.assertRaises(tc.TcGuestError):
                tc.validate_guest_tc_phases(**kw)
