"""Stage 0.39: kernel-enforced resource envelope acceptance contracts.

These unit tests assert policy parsing and fail-closed telemetry. They cannot
attest that a production cgroup is enabled; live CI exercises that separately.
"""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_vm_resources as vm


def healthy_snapshot():
    return {
        "memory.max": "805306368",
        "memory.swap.max": "0",
        "memory.oom.group": "1",
        "memory.current": "134217728",
        "memory.peak": "268435456",
        "memory.events": "low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\noom_group_kill 0\n",
        "pids.max": "96",
        "pids.current": "8",
        "pids.peak": "13",
        "pids.events": "max 0\n",
        "cpu.max": "200000 100000",
        "cpu.stat": "usage_usec 3600\nuser_usec 2800\nsystem_usec 800\n",
    }


class VmCgroupResourcesContracts(unittest.TestCase):
    def test_fixed_limits_are_derived_from_guest_memory_without_unlimited(self):
        self.assertEqual(vm.desired_limits(256), {
            "memory.max": "805306368",
            "memory.swap.max": "0",
            "memory.oom.group": "1",
            "pids.max": "96",
            "cpu.max": "200000 100000",
        })
        for value in (-1, 0, 63, 2049, True, "256"):
            with self.subTest(value=value), self.assertRaises(vm.VmResourceError):
                vm.desired_limits(value)

    def test_kernel_snapshot_is_observational_and_nonproduction(self):
        result = vm.verify_resource_snapshot(healthy_snapshot(), guest_mem_mib=256)
        self.assertEqual(result["status"], "KERNEL_VM_CGROUP_LIMITS_ENFORCED")
        self.assertEqual(result["observed_memory_peak_bytes"], 268435456)
        self.assertEqual(result["observed_pids_peak"], 13)
        self.assertFalse(result["production_approved"])
        self.assertFalse(result["stage_039_complete"])

    def test_fail_closed_on_missing_or_mutated_critical_cgroup_limits(self):
        for key, bad in [
            ("memory.max", "max"),
            ("memory.max", "805306369"),
            ("memory.swap.max", "max"),
            ("memory.oom.group", "0"),
            ("pids.max", "max"),
            ("pids.max", "97"),
            ("cpu.max", "max 100000"),
            ("cpu.max", "250000 100000"),
        ]:
            data = healthy_snapshot()
            data[key] = bad
            with self.subTest(key=key,bad=bad), self.assertRaises(vm.VmResourceError):
                vm.verify_resource_snapshot(data, guest_mem_mib=256)

    def test_kernel_cpu_stats_may_include_namespaced_dotted_counters(self):
        # Linux exposes core_sched.force_idle_usec on some kernels.
        snap = healthy_snapshot()
        snap["cpu.stat"] += "core_sched.force_idle_usec 0\\n"
        result = vm.verify_resource_snapshot(snap, guest_mem_mib=256)
        self.assertEqual(result["status"], "KERNEL_VM_CGROUP_LIMITS_ENFORCED")

    def test_fail_closed_if_oom_or_pids_max_is_recorded(self):
        for key, value in [
            ("memory.events", "oom 1\noom_kill 0\n"),
            ("memory.events", "oom 0\noom_kill 1\n"),
            ("memory.events", "oom 0\noom_kill 0\noom_group_kill 1\n"),
            ("pids.events", "max 1\n"),
        ]:
            data = healthy_snapshot()
            data[key] = value
            with self.subTest(key=key,value=value), self.assertRaises(vm.VmResourceError):
                vm.verify_resource_snapshot(data, guest_mem_mib=256)

    def test_peak_cannot_exceed_memory_or_pid_quota(self):
        for key, value in [
            ("memory.current", "805306369"),
            ("memory.peak", "805306369"),
            ("pids.current", "97"),
            ("pids.peak", "97"),
        ]:
            data = healthy_snapshot()
            data[key] = value
            with self.subTest(key=key), self.assertRaises(vm.VmResourceError):
                vm.verify_resource_snapshot(data, guest_mem_mib=256)

    def test_refuse_malformed_or_ambiguous_kernel_telemetry(self):
        samples = [
            ("memory.events", "oom 0\noom 0\noom_kill 0\n"),
            ("memory.events", "oom notanumber\noom_kill 0\n"),
            ("pids.events", "other 0\n"),
            ("memory.peak", "-1"),
            ("memory.current", "max"),
            ("cpu.stat", "usage_usec notanumber\n"),
            ("cpu.stat", ""),
        ]
        for key, value in samples:
            data = healthy_snapshot()
            data[key] = value
            with self.subTest(key=key), self.assertRaises(vm.VmResourceError):
                vm.verify_resource_snapshot(data, guest_mem_mib=256)

    def test_only_dedicated_root_level_cgroup_is_in_scope(self):
        good = Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service")
        self.assertTrue(vm.valid_target(good))
        for path in [
            Path("/tmp/ocaf-stage039-vm-0123456789abcdef.service"),
            Path("/sys/fs/cgroup/ssh.service"),
            Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdeg.service"),
            Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service/child"),
            Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service/../ssh.service"),
        ]:
            with self.subTest(path=path):
                self.assertFalse(vm.valid_target(path))


if __name__ == "__main__":
    unittest.main()
