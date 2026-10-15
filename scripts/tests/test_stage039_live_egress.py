"""Stage 0.39.7 local-loopback kernel eBPF egress acceptance contract.

These tests do not claim a live eBPF attach without a KVM/BPF host.
"""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_live_egress as harness


class TestLiveEgressHarness(unittest.TestCase):
    def test_rejects_cgroup_outside_own_test_scope(self):
        for path in ("/sys/fs/cgroup", "/sys/fs/cgroup/system.slice/sshd.service",
                     "/tmp/ocaf-stage039-a-test.service",
                     "/sys/fs/cgroup/../etc/ocaf-stage039-a-test.service",
                     "/sys/fs/cgroup/ocaf-stage039-a-../test.service"):
            with self.subTest(path=path):
                self.assertFalse(harness.permitted_probe_cgroup(Path(path)))

    def test_accepts_unique_scoped_probe_only(self):
        path = Path("/sys/fs/cgroup/ocaf-stage039-a-0123456789abcdef.service")
        self.assertTrue(harness.permitted_probe_cgroup(path))
        self.assertTrue(harness.permitted_probe_cgroup(Path(str(path).replace("-a-","-b-"))))

    def test_rejects_binary_symlink_writable_or_wrong_digest(self):
        with tempfile.TemporaryDirectory() as root:
            binary = Path(root) / "aya-loader"
            binary.write_bytes(b"not a real binary")
            binary.chmod(0o700)
            actual = harness.sha256_file(binary)
            self.assertEqual(harness.verified_executable(binary, actual), actual)
            with self.assertRaises(harness.AcceptanceError):
                harness.verified_executable(binary, "f"*64)
            binary.chmod(0o777)
            with self.assertRaises(harness.AcceptanceError):
                harness.verified_executable(binary, actual)
            binary.chmod(0o700)
            link = Path(root) / "symlink"
            link.symlink_to(binary)
            with self.assertRaises(harness.AcceptanceError):
                harness.verified_executable(link, actual)

    def test_rejects_missing_kernel_attachment_even_if_A_denied(self):
        with self.assertRaises(harness.AcceptanceError):
            harness.evaluate_probes(
                before_a=True, before_b=True, attached=False,
                during_a=False, during_b=True, after_a=True)

    def test_requires_B_noninterference_and_A_recovery(self):
        for before_a,before_b,attached,during_a,during_b,after_a in (
            (False, True, True, False, True, True),
            (True, False, True, False, True, True),
            (True, True, True, True, True, True),
            (True, True, True, False, False, True),
            (True, True, True, False, True, False)):
            with self.subTest(data=(before_a,before_b,attached,during_a,during_b,after_a)):
                with self.assertRaises(harness.AcceptanceError):
                    harness.evaluate_probes(
                        before_a=before_a, before_b=before_b,
                        attached=attached, during_a=during_a,
                        during_b=during_b, after_a=after_a)

    def test_accepts_only_controlled_negative_and_recovery(self):
        d = harness.evaluate_probes(before_a=True,before_b=True,attached=True,
                                    during_a=False,during_b=True,after_a=True)
        self.assertEqual(d["status"], "KERNEL_EBPF_EGRESS_ISOLATION_PASSED")
        self.assertTrue(d["tenant_b_not_blocked"])
        self.assertTrue(d["tenant_a_recovered_after_detach"])
        self.assertFalse(d["microvm_booted"])
        self.assertFalse(d["production_approved"])

    def test_rejects_fabricated_or_mismatched_aya_event(self):
        for event in ({}, {"status":"AYA_CGROUP_EGRESS_DETACHED","program_id":2},
                      {"status":"AYA_CGROUP_EGRESS_ATTACHED","program_id":0},
                      {"status":"AYA_CGROUP_EGRESS_ATTACHED","program_id":True}):
            with self.subTest(event=event):
                with self.assertRaises(harness.AcceptanceError):
                    harness.validate_aya_event(event, 23)

    def test_valid_kernel_event_is_pinned_to_cgroup_inode(self):
        e={"status":"AYA_CGROUP_EGRESS_ATTACHED","program_id":18,"cgroup_inode":23,
           "guest_booted":False,"production_approved":False}
        self.assertEqual(harness.validate_aya_event(e,23),18)
        with self.assertRaises(harness.AcceptanceError):
            harness.validate_aya_event(e,27)

    def test_cgroup_join_failure_never_counts_as_policy_block(self):
        with mock.patch.object(harness.subprocess, "run") as run:
            run.return_value = mock.Mock(returncode=1, stdout=json.dumps(
                {"joined":False,"attempted":False,"connected":False}), stderr="")
            with self.assertRaises(harness.AcceptanceError):
                harness.probe_connect(Path("/sys/fs/cgroup/ocaf-stage039-a-0123456789abcdef.service"),
                                      1234)
                
    def test_preflight_blocks_non_root_and_missing_bpf(self):
        with mock.patch.object(harness.os, "geteuid", return_value=1000):
            with self.assertRaises(harness.AcceptanceError):
                harness.preflight()
        with mock.patch.object(harness.os, "geteuid", return_value=0), \
             mock.patch.object(harness.Path, "exists", return_value=False):
            with self.assertRaises(harness.AcceptanceError):
                harness.preflight()


if __name__ == "__main__":
    unittest.main()
