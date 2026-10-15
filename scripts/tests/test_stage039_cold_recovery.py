"""0.39: independent-host artifact-mediated cold restart evidence validation."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_cold_recovery as cold


def proof(boot_id,nonce,when):
    return {
      "status":"TWO_JAILED_GUEST_ORIGINATED_UDP_TENANTS_ISOLATED_AND_FAULTED",
      "host_boot_id":boot_id,"checkpoint_nonce":nonce,
      "observed_at_utc":when,
      "jailer_used_for_both_guests":True,
      "guest_originated_network_tested":True,
      "tenant_a_denied_by_nftables_ingress":True,
      "tenant_a_recovered_after_firewall_removal":True,
      "tenant_b_survived_a_vm_sigkill":True,
      "tenant_a_vm_sigkill":{"sigkill_exit_code":-9,"cgroup_subtree_empty":True},
      "tenant_b_vm_sigkill":{"sigkill_exit_code":-9,"cgroup_subtree_empty":True},
      "production_approved":False
    }


class TestColdHandoff(unittest.TestCase):
    def test_valid_cold_handoff_with_real_guest_packets_and_distinct_kernels(self):
        a=proof("boot-a","0123456789abcdef","2026-10-15T12:00:00+00:00")
        b=proof("boot-b","0123456789abcdef","2026-10-15T12:03:00+00:00")
        x=cold.validate_handoff(a,b)
        self.assertTrue(x["separate_kernel_boot_ids"])
        self.assertTrue(x["guest_nonce_delivered_on_recovery_host"])
        self.assertFalse(x["live_interhost_rpc_failover_tested"])

    def test_fail_closed_for_replay_missing_crash_or_forged_network(self):
        a=proof("boot-a","0123456789abcdef","2026-10-15T12:00:00+00:00")
        b=proof("boot-b","0123456789abcdef","2026-10-15T12:03:00+00:00")
        for modified in (
            {"checkpoint_nonce":"fedcba9876543210"},
            {"host_boot_id":"boot-a"},
            {"observed_at_utc":"2026-10-15T11:00:00+00:00"},
            {"jailer_used_for_both_guests":False},
            {"guest_originated_network_tested":False},
            {"tenant_a_vm_sigkill":{"sigkill_exit_code":0,"cgroup_subtree_empty":True}},
            {"production_approved":True},
        ):
            with self.subTest(modified=modified),self.assertRaises(cold.ColdHandoffError):
                cold.validate_handoff(a,{**b,**modified})
