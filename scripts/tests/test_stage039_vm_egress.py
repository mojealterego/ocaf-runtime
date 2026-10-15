"""Fail-closed VM cgroup kernel egress contract tests (no hardware claims)."""
from pathlib import Path
import sys
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_vm_egress as vm

CGROUP = Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service")
VM_ID = "ocaf-0123456789abcdef01234567"
MEMBERSHIP = "0::/system.slice/ocaf-stage039-vm-0123456789abcdef.service"


class VmEgressContract(unittest.TestCase):
    def test_accepts_exact_vm_id_cgroup_and_pid_membership(self):
        self.assertTrue(vm.validate_vm_identity(VM_ID, CGROUP, MEMBERSHIP))
        self.assertTrue(vm.validate_vm_identity(
            VM_ID, CGROUP, "0::/ocaf-stage039-vm-0123456789abcdef.service"))

    def test_rejects_untrusted_vm_scope(self):
        for vm_id, path, membership in (
            ("tenant-a", CGROUP, MEMBERSHIP),
            (VM_ID, Path("/tmp/ocaf-stage039-vm-0123456789abcdef.service"), MEMBERSHIP),
            (VM_ID, Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdeg.service"), MEMBERSHIP),
            (VM_ID, Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service/child"), MEMBERSHIP),
            (VM_ID, CGROUP, "0::/ocaf-stage039-vm-0123456789abcdef.service-suffix"),
            (VM_ID, CGROUP, "0::/ocaf-stage039-vm-fedcba9876543210.service"),
            (VM_ID, CGROUP, "0::/ocaf-stage039-vm-0123456789abcdef.service-old"),
            (VM_ID, CGROUP, "1:name=systemd:/ocaf-stage039-vm-0123456789abcdef.service"),
            (VM_ID, CGROUP, ""),
        ):
            with self.subTest(vm_id=vm_id, path=str(path), membership=membership):
                with self.assertRaises(vm.VmCgroupEgressError):
                    vm.validate_vm_identity(vm_id, path, membership)

    def test_probe_rejects_dead_firecracker_process(self):
        dead = mock.Mock(pid=12345)
        dead.poll.return_value = 9
        with self.assertRaisesRegex(vm.VmCgroupEgressError, "Firecracker process"):
            vm.require_vm_running(dead, VM_ID, CGROUP)

    def test_positive_host_probe_is_not_guest_network_or_production_proof(self):
        result = vm.evaluate_vm_host_probe(
            before_a=True, before_b=True, attached=True,
            during_a=False, during_b=True, after_a=True)
        self.assertEqual(result["status"], "VM_HOST_CGROUP_EBPF_PROBE_PASSED")
        self.assertFalse(result["guest_network_egress_tested"])
        self.assertFalse(result["production_approved"])
        self.assertFalse(result["stage_039_complete"])
        self.assertTrue(result["host_vm_cgroup_egress_enforced"])

    def test_negative_noninterference_recovery_required(self):
        for during_a, during_b, after_a in (
            (True, True, True), (False, False, True), (False, True, False)):
            with self.subTest(during_a=during_a,during_b=during_b,after_a=after_a):
                with self.assertRaises(vm.VmCgroupEgressError):
                    vm.evaluate_vm_host_probe(
                        before_a=True, before_b=True, attached=True,
                        during_a=during_a, during_b=during_b, after_a=after_a)


if __name__ == "__main__":
    unittest.main()
