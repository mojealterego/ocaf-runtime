import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage039_lifecycle import Lifecycle, LifecycleError, State


class LifecycleTests(unittest.TestCase):
    def test_requires_enforcement_before_start(self):
        lifecycle = Lifecycle("w1")
        with self.assertRaises(LifecycleError):
            lifecycle.vm_started(vm_id="vm", cgroup_inode=1)
        lifecycle.jailer_ready(immutable_assets=True)
        with self.assertRaises(LifecycleError):
            lifecycle.enforce_egress(ebpf_program_id=0)

    def test_full_receipt_bound_release(self):
        lifecycle = Lifecycle("w1")
        lifecycle.jailer_ready(immutable_assets=True)
        lifecycle.enforce_egress(ebpf_program_id=7)
        lifecycle.vm_started(vm_id="vm-1", cgroup_inode=42)
        lifecycle.bind_receipt(receipt_digest="a" * 64, exit_code=0)
        lifecycle.release()
        self.assertEqual(lifecycle.state, State.RELEASED)

    def test_failure_is_terminal(self):
        lifecycle = Lifecycle("w1")
        lifecycle.fail_closed()
        with self.assertRaises(LifecycleError):
            lifecycle.jailer_ready(immutable_assets=True)

    def test_release_requires_guest_receipt(self):
        lifecycle = Lifecycle("w1")
        lifecycle.jailer_ready(immutable_assets=True)
        lifecycle.enforce_egress(ebpf_program_id=1)
        lifecycle.vm_started(vm_id="vm-1", cgroup_inode=1)
        with self.assertRaises(LifecycleError):
            lifecycle.release()


if __name__ == "__main__":
    unittest.main()
