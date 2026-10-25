"""Stage 0.39: independent GitHub-runner kernel guest failure evidence."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_independent_hosts as hosts


def entry(tenant,boot,vm,host):
    return {"tenant_id":tenant,"host_boot_id":boot,"hostname":host,
            "soak":{"status":"ONE_HOST_100_REAL_KVM_JAILED_VM_SIGKILL_CYCLES_PASS",
                    "tenant_id":tenant,"host_boot_id":boot,
                    "cycle_count":100,"distinct_vm_ids":100,
                    "all_running_rootfs_fd_inodes_pinned":True,
                    "all_vm_cgroups_drained_after_sigkill":True,
                    "durable_host_identity_proved":False,
                    "cycles":[{"cycle":i,"vm_id":f"vm-{i}",
                               "cgroup_drained_after_sigkill":True}
                              for i in range(1,101)]},
            "guest":{"vm_id":vm,"status":"JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED",
                     "firecracker_api_running":True,"guest_kernel_console_seen":True,
                     "guest_rootfs_sha256":"a"*64,
                     "running_rootfs_fd":{
                         "status":"RUNNING_VMM_ROOTFS_FD_MATCHES_PINNED_INODE",
                         "rootfs_sha256":"a"*64,"rootfs_inode":123,
                         "matching_fd_count":1},
                     "tenant_id":tenant,"vm_sigkill_fault":{
                         "status":"VM_SIGKILL_CGROUP_SUBTREE_DRAINED",
                         "cgroup_subtree_empty":True,
                         "sigkill_exit_code":-9},
                     "production_approved":False}}


class IndependentHostValidator(unittest.TestCase):
    def test_real_separate_kernel_boot_ids_and_host_failure(self):
        a=entry("tenant-a","boot-a","vm-a","host-1")
        b=entry("tenant-b","boot-b","vm-b","host-2")
        z=hosts.verify_hosts(a,b)
        self.assertTrue(z["two_separate_kernel_boot_ids_observed"])
        self.assertFalse(z["interhost_rpc_tested"])
        self.assertFalse(z["production_approved"])

    def test_distinct_kernel_boots_are_valid_even_if_hostnames_are_reused(self):
        # GitHub ephemeral runners may reuse image-supplied hostname strings;
        # /proc/sys/kernel/random/boot_id is the actual kernel-identity signal.
        a=entry("tenant-a","boot-a","vm-a","runner")
        b=entry("tenant-b","boot-b","vm-b","runner")
        report=hosts.verify_hosts(a,b)
        self.assertTrue(report["two_separate_kernel_boot_ids_observed"])
        self.assertFalse(report["hostnames_are_distinct"])
        self.assertFalse(report["durable_production_host_independence_proved"])

    def test_rejects_same_kernel_or_reused_guest_evidence(self):
        a=entry("tenant-a","boot-a","vm-a","host-1")
        b=entry("tenant-b","boot-b","vm-b","host-2")
        for mutated in (
            {**b,"host_boot_id":"boot-a"},
            {**b,"guest":{**b["guest"],"vm_id":"vm-a"}},
            {**b,"tenant_id":"tenant-a"},
            {**b,"guest":{**b["guest"],"tenant_id":"tenant-a"}},
            {**b,"guest":{**b["guest"],"vm_sigkill_fault":{"cgroup_subtree_empty":False}}},
            {**b,"guest":{**b["guest"],"running_rootfs_fd":None}},
            {**b,"guest":{**b["guest"],"running_rootfs_fd":{
                **b["guest"]["running_rootfs_fd"],"rootfs_sha256":"b"*64}}},
            {**b,"soak":{**b["soak"],"cycle_count":99}},
        ):
            with self.subTest(mutated=mutated),self.assertRaises(hosts.IndependentHostError):
                hosts.verify_hosts(a,mutated)
