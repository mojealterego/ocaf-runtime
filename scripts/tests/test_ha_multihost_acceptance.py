"""Offline regression of multi-host test harness input and result validation."""
import importlib.util
import json
from pathlib import Path
import sys
import unittest

FILE = Path(__file__).resolve().parents[1] / "ha_multihost_acceptance.py"
spec = importlib.util.spec_from_file_location("ha_multihost_acceptance", FILE)
probe = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = probe
spec.loader.exec_module(probe)
CLUSTER = "ocaf33_" + "a" * 32
HOSTS = ["ocaf@node-a.example.net", "ocaf@node-b.example.net", "ocaf@node-c.example.net"]


def observation(outcome, epoch=None):
    return {"schema_version": 1, "cluster_id": CLUSTER,
            "outcome": outcome, "epoch": epoch}


class HaMultiHostAcceptanceTests(unittest.TestCase):
    def test_validates_three_distinct_destinations(self):
        probe.validate_args(HOSTS, CLUSTER, "/usr/local/bin/ocaf-ha-probe", "/etc/ocaf/pg-dsn")
        with self.assertRaises(probe.AcceptanceError):
            probe.validate_args([HOSTS[0], HOSTS[0], HOSTS[2]], CLUSTER,
                                "/usr/local/bin/ocaf-ha-probe", "/etc/ocaf/pg-dsn")

    def test_prevents_ssh_option_and_remote_shell_injection(self):
        for host in ("-oProxyCommand=evil", "bad;rm -rf /", "", "../evil"):
            with self.assertRaises(probe.AcceptanceError):
                probe.validate_args([host, *HOSTS[1:]], CLUSTER,
                                    "/usr/local/bin/ocaf-ha-probe", "/etc/ocaf/pg-dsn")
        command = probe.ssh_command(HOSTS[0], CLUSTER, "/usr/local/bin/ocaf-ha-probe",
                                    "/etc/ocaf/pg-dsn", "--mode", "attempt", "--expect", "denied")
        self.assertIn("StrictHostKeyChecking=yes", command)
        self.assertIn("BatchMode=yes", command)
        self.assertIn("OCAF_ALLOW_INSECURE_LOOPBACK=0", command[-1])

    def test_rejects_non_test_cluster_and_unsafe_paths(self):
        for namespace in ("production", "ocaf33_short", "ocaf33_" + "z" * 32):
            with self.assertRaises(probe.AcceptanceError):
                probe.validate_args(HOSTS, namespace, "/usr/local/bin/ocaf-ha-probe", "/etc/ocaf/pg-dsn")
        with self.assertRaises(probe.AcceptanceError):
            probe.validate_args(HOSTS, CLUSTER, "/usr/../bin/probe", "/etc/ocaf/pg-dsn")

    def test_rejects_wrong_epoch_order(self):
        with self.assertRaises(probe.AcceptanceError):
            probe.check_sequence(CLUSTER, observation("acquired", 5),
                                 observation("denied"), observation("denied"),
                                 observation("released", 5), observation("acquired", 5),
                                 observation("acquired", 7))

    def test_accepts_monotonic_three_host_sequence(self):
        self.assertEqual(probe.check_sequence(
            CLUSTER, observation("acquired", 7), observation("denied"),
            observation("denied"), observation("released", 7),
            observation("acquired", 8), observation("acquired", 9)), [7, 8, 9])

    def test_refuses_forged_probe_outcome_or_cluster(self):
        bad = observation("denied")
        bad["epoch"] = 12
        with self.assertRaises(probe.AcceptanceError):
            probe.parse_record(json.dumps(bad), CLUSTER, "denied")
        bad = observation("acquired", 1)
        bad["cluster_id"] = "another"
        with self.assertRaises(probe.AcceptanceError):
            probe.parse_record(json.dumps(bad), CLUSTER, "acquired")

    def test_refuses_boolean_epoch(self):
        with self.assertRaises(probe.AcceptanceError):
            probe.parse_record(json.dumps(observation("acquired", True)), CLUSTER, "acquired")


if __name__ == "__main__":
    unittest.main()
