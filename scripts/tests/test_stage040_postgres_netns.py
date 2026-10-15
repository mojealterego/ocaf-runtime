"""0.40.6 real kernel network namespace PostgreSQL TCP replication and partition."""
from pathlib import Path
import sys, unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_postgres_netns as n

def proof():
    return {
        "schema_version":1,
        "status":"POSTGRES_THREE_NETNS_TCP_STREAM_PARTITION_RECOVERY_PASSED",
        "postgresql_real_processes":True,
        "three_distinct_kernel_netns_inodes":True,
        "bridge_veth_real_kernel_network":True,
        "two_distinct_remote_pg_stat_replication_client_ips":True,
        "initial_wal_replay_to_both":True,
        "kernel_route_blackhole_b_to_a_installed":True,
        "partitioned_b_stale_while_c_updated":True,
        "b_recovered_after_network_heal":True,
        "primary_a_immediate_crash":True,
        "standby_b_promoted_writable":True,
        "standby_c_reparented_to_b_streaming":True,
        "standby_c_replayed_after_b_promotion":True,
        "one_underlying_linux_kernel":True,
        "inter_kernel_tcp_streaming_tested":False,
        "physical_host_loss_tested":False,
        "production_approved":False
    }

class TCPReplicationProofTests(unittest.TestCase):
    def test_accepts_real_netns_tcp_not_claims_of_interhost(self):
        x=n.validate_report(proof())
        self.assertTrue(x["bridge_veth_real_kernel_network"])
        self.assertFalse(x["inter_kernel_tcp_streaming_tested"])
    def test_every_observation_required_and_no_false_production_flags(self):
        for k,v in proof().items():
            if v is True:
                with self.subTest(k=k),self.assertRaises(n.NetnsPGError):
                    n.validate_report({**proof(),k:False})
        for k in ("inter_kernel_tcp_streaming_tested","physical_host_loss_tested","production_approved"):
            with self.subTest(k=k),self.assertRaises(n.NetnsPGError):
                n.validate_report({**proof(),k:True})
    def test_unexpected_kernel_client_ips_denied(self):
        n.validate_client_ips(("10.247.40.2","10.247.40.3"))
        for ips in (("10.247.40.2","10.247.40.2"),("127.0.0.1","10.247.40.3"),("10.247.40.1","10.247.40.3")):
            with self.subTest(ips=ips),self.assertRaises(n.NetnsPGError):
                n.validate_client_ips(ips)
