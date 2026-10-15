"""Stage 0.39.9: fail-closed real VM SIGKILL and cgroup-subtree drain."""
import signal
import sys
from pathlib import Path
from unittest import TestCase, mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_vm_crash as crash


class VmCrashContracts(TestCase):
    def test_cgroup_events_must_show_entire_subtree_empty(self):
        self.assertEqual(
            crash.validate_drained_subtree("populated 0\nfrozen 0\n")["status"],
            "VM_CGROUP_SUBTREE_EMPTY_AFTER_SIGKILL",
        )

    def test_rejects_still_populated_or_fabricated_event_files(self):
        for raw in (
            "", "populated 1\nfrozen 0\n", "frozen 0\n",
            "populated 0\npopulated 0\n",
            "populated 0\ninvalid", "populated -1\n",
            "populated 0\nother bad\n", "populated 2\n",
        ):
            with self.subTest(raw=raw), self.assertRaises(crash.VmCrashError):
                crash.validate_drained_subtree(raw)

    def test_refuses_signal_if_vm_pid_identity_is_invalid(self):
        proc=mock.Mock(pid=19119)
        proc.poll.return_value=None
        cg=Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service")
        with mock.patch.object(crash.vm_egress, "require_vm_running",
                               side_effect=ValueError("untrusted VM")):
            with self.assertRaises(crash.VmCrashError):
                crash.kill_and_verify(proc, "ocaf-0123456789abcdef01234567", cg)
        proc.kill.assert_not_called()

    def test_real_sigkill_result_is_scoped_and_not_release_approval(self):
        proc=mock.Mock(pid=19119)
        proc.poll.return_value=None
        proc.wait.return_value=-signal.SIGKILL
        cg=Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service")
        with mock.patch.object(crash.vm_egress, "require_vm_running", return_value=67123), \
             mock.patch.object(crash, "_cgroup_events", return_value="populated 0\nfrozen 0\n"):
            report=crash.kill_and_verify(
                proc, "ocaf-0123456789abcdef01234567", cg)
        proc.kill.assert_called_once()
        self.assertEqual(report["status"], "VM_SIGKILL_CGROUP_SUBTREE_DRAINED")
        self.assertTrue(report["sigkill_executed"])
        self.assertTrue(report["cgroup_subtree_empty"])
        self.assertFalse(report["production_approved"])
        self.assertFalse(report["stage_039_complete"])

    def test_wrong_signal_exit_cannot_pass(self):
        proc=mock.Mock(pid=19119)
        proc.poll.return_value=None
        proc.wait.return_value=0
        cg=Path("/sys/fs/cgroup/ocaf-stage039-vm-0123456789abcdef.service")
        with mock.patch.object(crash.vm_egress, "require_vm_running", return_value=67123), \
             mock.patch.object(crash, "_cgroup_events", return_value="populated 0\n"):
            with self.assertRaises(crash.VmCrashError):
                crash.kill_and_verify(proc, "ocaf-0123456789abcdef01234567", cg)


if __name__ == "__main__":
    import unittest
    unittest.main()
