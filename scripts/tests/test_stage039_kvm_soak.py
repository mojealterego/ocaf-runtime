"""Receipt gates for the real 100-cycle KVM host runner."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_kvm_soak as soak


def receipt():
    return {
        "tenant_id": "tenant-a", "host_boot_id": "a" * 36,
        "vm_id": "ocaf-" + "a" * 24,
        "status": "JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED",
        "firecracker_api_running": True, "guest_kernel_console_seen": True,
        "running_rootfs_fd": {
            "status": "RUNNING_VMM_ROOTFS_FD_MATCHES_PINNED_INODE",
            "rootfs_sha256": "b" * 64, "matching_fd_count": 1,
        },
        "vm_sigkill_fault": {
            "status": "VM_SIGKILL_CGROUP_SUBTREE_DRAINED",
            "cgroup_subtree_empty": True, "sigkill_exit_code": -9,
        },
        "stage_039_complete": False, "production_approved": False,
    }


class SoakReceipt(unittest.TestCase):
    def test_exact_live_fd_and_crash_receipt(self):
        boot, vm = soak.verify_cycle(receipt(), tenant="tenant-a",
                                     host_boot_id=None, rootfs_sha256="b" * 64)
        self.assertEqual((boot, vm), ("a" * 36, "ocaf-" + "a" * 24))

    def test_missing_fd_crash_or_host_identity_is_denied(self):
        good = receipt()
        mutations = (
            {**good, "running_rootfs_fd": None},
            {**good, "running_rootfs_fd": {
                **good["running_rootfs_fd"], "rootfs_sha256": "c" * 64}},
            {**good, "vm_sigkill_fault": {
                **good["vm_sigkill_fault"], "cgroup_subtree_empty": False}},
            {**good, "production_approved": True},
        )
        for value in mutations:
            with self.subTest(value=value), self.assertRaises(soak.SoakDenied):
                soak.verify_cycle(value, tenant="tenant-a", host_boot_id=None,
                                  rootfs_sha256="b" * 64)
        with self.assertRaises(soak.SoakDenied):
            soak.verify_cycle(good, tenant="tenant-a", host_boot_id="different-boot-id",
                              rootfs_sha256="b" * 64)


if __name__ == "__main__":
    unittest.main()
