"""0.39: two independently attributed LIVE concurrent jailed KVM microVMs."""
import json
from pathlib import Path
import sys
import unittest
from unittest import mock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_dual_tenant as dual

class TestDualTenant(unittest.TestCase):
    def test_two_manifests_must_have_separate_tenants_agents_and_workloads(self):
        a={"tenant_id":"tenant-a","agent_id":"agent-a","workload_id":"job-a"}
        b={"tenant_id":"tenant-b","agent_id":"agent-b","workload_id":"job-b"}
        self.assertEqual(dual.validate_distinct_tenants(a,b),("tenant-a","tenant-b"))
        for values in (
            dict(tenant_id="tenant-a"),
            dict(agent_id="agent-a"),
            dict(workload_id="job-a"),
            dict(tenant_id="../malicious"),
        ):
            with self.subTest(values=values),self.assertRaises(dual.DualTenantError):
                dual.validate_distinct_tenants(a,{**b,**values})

    def test_rejects_fabricated_concurrency_and_same_kernel_cgroup(self):
        a={"tenant_id":"tenant-a","vm_id":"ocaf-"+"1"*24,
           "cgroup_inode":100,"host_boot_id":"boot01","status":"JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED",
           "firecracker_api_running":True,"guest_kernel_console_seen":True,"production_approved":False}
        b={**a,"tenant_id":"tenant-b","vm_id":"ocaf-"+"2"*24,"cgroup_inode":200}
        self.assertTrue(dual.validate_dual_evidence(a,b,True)["two_guests_simultaneously_running"])
        for bad in ({**b,"vm_id":a["vm_id"]},
                    {**b,"cgroup_inode":100},
                    {**b,"host_boot_id":"different"},
                    {**b,"guest_kernel_console_seen":False}):
            with self.subTest(bad=bad),self.assertRaises(dual.DualTenantError):
                dual.validate_dual_evidence(a,bad,True)
        with self.assertRaises(dual.DualTenantError):
            dual.validate_dual_evidence(a,b,False)

    def test_rejects_tenant_b_death_during_a_crash(self):
        with self.assertRaises(dual.DualTenantError):
            dual.validate_peer_survived_a_crash(
                a={"vm_sigkill_fault":{"status":"VM_SIGKILL_CGROUP_SUBTREE_DRAINED",
                                       "cgroup_subtree_empty":True}},
                b_still_running=False)

    def test_no_real_vm_test_is_pretended_by_mock(self):
        with self.assertRaises(dual.DualTenantError):
            dual.validate_distinct_tenants(
                {"tenant_id":"tenant-a","agent_id":"a","workload_id":"a"},
                {"tenant_id":"tenant-b","agent_id":"a","workload_id":"b"})
