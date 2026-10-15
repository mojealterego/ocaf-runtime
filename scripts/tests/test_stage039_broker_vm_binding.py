"""OCAF 0.39: admission from real SO_PEERCRED broker must match actual jailed VM evidence."""
import hashlib
import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_broker_vm_binding as binding


def manifest(tenant):
    return {"schema_version":1,"tenant_id":"tenant-"+tenant,
            "agent_id":"agent-"+tenant,"workload_id":"workload-"+tenant,
            "isolation":{"network":"none","guest_egress":False,"jailer":True},
            "assets":{"guest_kernel":{"sha256":"a"*64},
                      "guest_rootfs":{"sha256":"b"*64},
                      "firecracker":{"sha256":"c"*64}}}


def admission(tenant,raw):
    return {"status":"RESERVED_NOT_EXECUTED","tenant_id":"tenant-"+tenant,
            "agent_id":"agent-"+tenant,"workload_id":"workload-"+tenant,
            "request_id":("a" if tenant=="a" else "b")*32,
            "leader_epoch":1,"manifest_sha256":hashlib.sha256(raw).hexdigest(),
            "microvm_launched":False,"ebpf_attached":False,"production_approved":False,
            "stage_039_complete":False}


def vm(tenant,cg,vmid):
    return {"tenant_id":"tenant-"+tenant,"agent_id":"agent-"+tenant,
            "workload_id":"workload-"+tenant,"vm_id":vmid,
            "cgroup_inode":cg,
            "status":"JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED",
            "firecracker_api_running":True,"guest_kernel_console_seen":True,
            "process_in_expected_cgroup":True,
            "host_boot_id":"39a9d1b0-757b-48b6-a255-f157ffe3e5ec",
            "guest_kernel_sha256":"a"*64,"guest_rootfs_sha256":"b"*64,
            "firecracker_sha256":"c"*64,"production_approved":False,
            "vm_sigkill_fault":{"status":"VM_SIGKILL_CGROUP_SUBTREE_DRAINED",
                                "cgroup_subtree_empty":True,
                                "sigkill_executed":True,
                                "sigkill_exit_code":-9,
                                "vm_cgroup_inode":cg}}


def evidence():
    return {"status":"TWO_CONCURRENT_TENANTS_VM_BOOT_AND_FAULT_ISOLATION_PASSED",
            "host_boot_id":"39a9d1b0-757b-48b6-a255-f157ffe3e5ec",
            "tenant_a":vm("a",1321,"ocaf-"+"a"*24),
            "tenant_b":vm("b",1322,"ocaf-"+"b"*24),
            "simultaneous_guest_boot":{"two_guests_simultaneously_running":True},
            "fault_isolation":{"tenant_b_survived_tenant_a_sigkill":True},
            "two_independent_hosts":False,"guest_network_egress_tested":False,
            "production_approved":False,"stage_039_complete":False}


class BrokerVmAcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.raw={x:json.dumps(manifest(x),sort_keys=True).encode() for x in ("a","b")}
        self.reports={x:admission(x,self.raw[x]) for x in ("a","b")}
        self.actual=evidence()

    def test_auth_broker_bound_to_actual_two_kernel_guests(self):
        p=binding.verify_binding(self.reports,self.raw,self.actual)
        self.assertEqual(p["status"],"BROKER_SO_PEERCRED_TWO_REAL_JAILED_VM_BOOT_BOUND")
        self.assertTrue(p["admission_before_vm_launch"])
        self.assertTrue(p["both_guest_kernel_boots_observed"])
        self.assertTrue(p["both_vm_cgroups_drained_after_sigkill"])
        self.assertFalse(p["wasi_executed_inside_guest"])
        self.assertFalse(p["production_approved"])

    def test_cross_tenant_replay_and_tampering_denied(self):
        for key,value in (("workload_id","workload-b"),
                          ("agent_id","agent-b"),
                          ("cgroup_inode",1322),
                          ("guest_rootfs_sha256","0"*64),
                          ("firecracker_api_running",False),
                          ("guest_kernel_console_seen",False)):
            with self.subTest(key=key),self.assertRaises(binding.BindingError):
                binding.verify_binding(self.reports,self.raw,{
                    **self.actual,"tenant_a":{**self.actual["tenant_a"],key:value}})

    def test_rejects_forged_approval_wrong_admission_hash_and_epoch(self):
        for change in ({"manifest_sha256":"0"*64},
                       {"production_approved":True},
                       {"status":"RELEASED_NOT_EXECUTED"},
                       {"microvm_launched":True},
                       {"request_id":"123"}):
            with self.subTest(change=change),self.assertRaises(binding.BindingError):
                binding.verify_binding({**self.reports,"a":{**self.reports["a"],**change}},
                                       self.raw,self.actual)
        with self.assertRaises(binding.BindingError):
            binding.verify_binding(self.reports,self.raw,{**self.actual,"production_approved":True})

    def test_rejects_faults_or_unobserved_parallel_guests(self):
        cases=(
            {"simultaneous_guest_boot":{"two_guests_simultaneously_running":False}},
            {"fault_isolation":{"tenant_b_survived_tenant_a_sigkill":False}},
            {"two_independent_hosts":True},
            {"guest_network_egress_tested":True},
            {"tenant_b":{**self.actual["tenant_b"],
                "vm_sigkill_fault":{"sigkill_exit_code":0}}},
        )
        for case in cases:
            with self.subTest(case=case),self.assertRaises(binding.BindingError):
                binding.verify_binding(self.reports,self.raw,{**self.actual,**case})

    def test_nonmatching_broker_ledger_lease_is_denied(self):
        rows={"a":("tenant-a","agent-a","workload-a",1,"a"*32,"reserved"),
              "b":("tenant-b","agent-b","workload-b",1,"b"*32,"reserved")}
        binding.verify_ledger_rows(rows,self.reports)
        with self.assertRaises(binding.BindingError):
            binding.verify_ledger_rows({**rows,"a":(*rows["a"][:5],"released")},self.reports)
        with self.assertRaises(binding.BindingError):
            binding.verify_ledger_rows({**rows,"b":("tenant-a",*rows["b"][1:])},self.reports)
