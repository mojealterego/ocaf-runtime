"""0.39.11: guest-originated UDP TAP isolation, not host-cgroup surrogate."""
import ipaddress
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_guest_network as net

class GuestNetworkContract(unittest.TestCase):
    def test_requires_guest_packet_nonce_tenant_and_unspoofed_source(self):
        nonce="0123456789abcdef"
        self.assertEqual(net.verify_guest_datagram(b"OCAF039:A:0123456789abcdef:7",
                         "198.19.38.2","A",nonce),7)
        for payload,src,tenant in (
            (b"OCAF039:A:other:7","198.19.38.2","A"),
            (b"OCAF039:A:0123456789abcdef:7","198.19.38.6","A"),
            (b"OCAF039:B:0123456789abcdef:7","198.19.38.2","A"),
            (b"OCAF039:A:0123456789abcdef:-1","198.19.38.2","A"),
            (b"OCAF039:A:0123456789abcdef:7extra","198.19.38.2","A"),
            (b"", "198.19.38.2","A"),
        ):
            with self.subTest(payload=payload,src=src), self.assertRaises(net.GuestNetworkError):
                net.verify_guest_datagram(payload,src,tenant,nonce)

    def test_config_nic_is_distinct_and_no_external_bridge(self):
        a=net.build_lab_config("A","oc39a01234","/tmp/a.img","/tmp/k.bin")
        b=net.build_lab_config("B","oc39b01234","/tmp/b.img","/tmp/k.bin")
        self.assertEqual(a["network-interfaces"][0]["host_dev_name"],"oc39a01234")
        self.assertEqual(b["network-interfaces"][0]["host_dev_name"],"oc39b01234")
        self.assertNotEqual(a["network-interfaces"][0]["guest_mac"],b["network-interfaces"][0]["guest_mac"])
        self.assertTrue(a["drives"][0]["is_read_only"])
        for bad in ("eth0","../tap","longer-than-15-characters"):
            with self.subTest(name=bad),self.assertRaises(net.GuestNetworkError):
                net.build_lab_config("A",bad,"/tmp/a.img","/tmp/k.bin")

    def test_fail_closed_negative_and_recovery_require_sequential_new_packets(self):
        self.assertTrue(net.validate_phases(1,3,7,9,11,12)["guest_originated_network_tested"])
        for samples in ((-1,3,7,9,11,12),(1,3,3,9,11,12),
                        (1,3,7,1,11,12),(1,3,7,9,7,12),(1,3,7,9,11,11)):
            with self.subTest(samples=samples), self.assertRaises(net.GuestNetworkError):
                net.validate_phases(*samples)
