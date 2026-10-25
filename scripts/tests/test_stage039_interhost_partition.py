import sys
from pathlib import Path
import unittest
from unittest.mock import patch
import subprocess
import json

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage039_interhost_partition import PartitionDenied, check_response, drill, drop_counter


class InterhostPartitionTests(unittest.TestCase):
    def test_exact_fencing_response_required(self):
        baseline = {"status": "LEASE_RENEWED", "tenant_id": "tenant-a",
                    "epoch": 4, "production_approved": False}
        check_response(baseline, "LEASE_RENEWED", 4)
        for key, value in (("status", "LEASE_GRANTED"), ("tenant_id", "tenant-b"),
                           ("epoch", 3), ("production_approved", True)):
            changed = dict(baseline)
            changed[key] = value
            with self.subTest(key=key), self.assertRaises(PartitionDenied):
                check_response(changed, "LEASE_RENEWED", 4)

    @patch("stage039_interhost_partition.os.geteuid", return_value=0)
    @patch("stage039_interhost_partition.shutil.which", return_value="/usr/sbin/nft")
    def test_loopback_and_invalid_port_rejected_before_network(self, *_):
        for address, port in (("127.0.0.1", 24399), ("::1", 24399),
                              ("192.0.2.4", 21), ("bad-ip", 24399)):
            with self.subTest(address=address, port=port), self.assertRaises(ValueError):
                drill(address, port, Path("c"), Path("k"), Path("ca"))

    @patch("stage039_interhost_partition.os.geteuid", return_value=0)
    @patch("stage039_interhost_partition.shutil.which", return_value="/usr/sbin/nft")
    def test_kernel_drop_denies_renew_and_cleanup_precedes_recovery(self, *_):
        calls = []
        list_count = 0
        def command(argv):
            calls.append(argv)
        def subprocess_call(argv, **_kwargs):
            nonlocal list_count
            if argv[:3] == ["ip", "-j", "route"]:
                return subprocess.CompletedProcess(argv, 0, '[{"dev":"eth0"}]', '')
            list_count += 1
            return subprocess.CompletedProcess(argv, 0,
                self.nft_json(0 if list_count == 1 else 1), '')
        rpc_count = 0
        def rpc(**_kwargs):
            nonlocal rpc_count
            rpc_count += 1
            if rpc_count == 2:
                raise OSError("kernel packet drop")
            return {"status": {1: "LEASE_GRANTED", 3: "LEASE_RENEWED",
                               4: "LEASE_RELEASED"}[rpc_count],
                    "tenant_id": "tenant-a", "epoch": 123,
                    "expires_unix_secs": 9999999999,
                    "production_approved": False}
        with (patch("stage039_interhost_partition.run_command", side_effect=command),
              patch("stage039_interhost_partition.subprocess.run", side_effect=subprocess_call),
              patch("stage039_interhost_partition.fence.client_rpc", side_effect=rpc),
              patch("stage039_interhost_partition.time.time_ns", return_value=123)):
            result = drill("192.0.2.4", 24399, Path("c"), Path("k"), Path("ca"))
        self.assertEqual(rpc_count, 4)
        self.assertEqual(calls[-1][:4], ["nft", "delete", "table", "ip"])
        self.assertTrue(result["renewal_during_packet_partition_denied"])
        self.assertEqual(result["nft_drop_packets_after_blocked_renewal"], 1)
        self.assertFalse(result["multi_host_failover_tested"])

    @staticmethod
    def nft_json(packets):
        return json.dumps({"nftables": [{"rule": {"expr": [
            {"match": {"op": "==", "left": {"payload": {
                "protocol": "ip", "field": "daddr"}}, "right": "192.0.2.4"}},
            {"match": {"op": "==", "left": {"payload": {
                "protocol": "tcp", "field": "dport"}}, "right": 24399}},
            {"counter": {"packets": packets, "bytes": packets * 40}},
            {"drop": None}]}}]})

    def test_timeout_without_packet_on_exact_drop_rule_is_denied(self):
        self.assertEqual(drop_counter(self.nft_json(0), "192.0.2.4", 24399), 0)
        with self.assertRaises(PartitionDenied):
            drop_counter(self.nft_json(5), "192.0.2.5", 24399)
        with self.assertRaises(PartitionDenied):
            drop_counter(self.nft_json(5), "192.0.2.4", 24400)

    @patch("stage039_interhost_partition.os.geteuid", return_value=0)
    @patch("stage039_interhost_partition.shutil.which", return_value="/usr/sbin/nft")
    def test_renewal_timeout_without_counter_increment_fails_and_removes_rule(self, *_):
        commands = []
        def command(argv): commands.append(argv)
        def subprocess_call(argv, **_kwargs):
            if argv[:3] == ["ip", "-j", "route"]:
                return subprocess.CompletedProcess(argv, 0, '[{"dev":"eth0"}]', '')
            return subprocess.CompletedProcess(argv, 0, self.nft_json(0), '')
        rpc_count = 0
        def rpc(**_kwargs):
            nonlocal rpc_count
            rpc_count += 1
            if rpc_count == 2:
                raise OSError("unrelated timeout")
            return {"status": "LEASE_GRANTED", "tenant_id": "tenant-a",
                    "epoch": 123, "expires_unix_secs": 9999999999,
                    "production_approved": False}
        with (patch("stage039_interhost_partition.run_command", side_effect=command),
              patch("stage039_interhost_partition.subprocess.run", side_effect=subprocess_call),
              patch("stage039_interhost_partition.fence.client_rpc", side_effect=rpc),
              patch("stage039_interhost_partition.time.time_ns", return_value=123)):
            with self.assertRaisesRegex(PartitionDenied, "did not hit scoped"):
                drill("192.0.2.4", 24399, Path("c"), Path("k"), Path("ca"))
        self.assertEqual(commands[-1][:4], ["nft", "delete", "table", "ip"])


if __name__ == "__main__":
    unittest.main()
