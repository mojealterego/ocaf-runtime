"""Negative joint-fault gate contracts; real kernel behavior is CI gated."""
from pathlib import Path
import sys
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_joint_fault_guest_aya as fault


class FaultProofGate(unittest.TestCase):
    def test_cgroup_path_parses_proc_line_terminator(self):
        name = "ocaf-stage039-vm-" + "a" * 16 + ".service"
        peer = mock.Mock(cgroup="0::/" + name + "\n")
        with mock.patch.object(fault.Path, "is_dir", return_value=True), \
             mock.patch.object(fault.Path, "is_symlink", return_value=False), \
             mock.patch.object(fault.Path, "stat", return_value=mock.Mock(st_uid=0)):
            self.assertEqual(fault.vm_cgroup_from_peer(peer), Path("/sys/fs/cgroup") / name)

    def setUp(self):
        self.bridge = fault.AyaFaultBridge(Path("/tmp/nonexistent-ocaf039"))
        self.bridge.guard = mock.Mock()
        self.bridge.receipt = {"kernel_bpf_program_id": 98}
        self.bridge.ready.set()
        self.proc = mock.Mock()
        self.proc.poll.return_value = None
        self.other = (mock.Mock(), "ocaf-" + "a" * 24,
                      Path("/sys/fs/cgroup/ocaf-stage039-vm-" + "a" * 16 + ".service"), 77)
        self.serial = ("OCAF039_FAULT_HOST_AYA_GATE_RELEASED\n"
                       "OCAF039_Q_GUEST_DENIED=aya_ready\n")
        self.result = {}

    def invoke(self):
        with mock.patch.object(fault.live, "_bounded_console_evidence",
                               return_value=self.serial), mock.patch.object(
                                   fault.vm_egress, "require_vm_running", return_value=77):
            self.bridge.prove(self.result, self.proc, "ocaf-" + "b" * 24,
                              Path("/sys/fs/cgroup/x"), Path("/tmp/serial"), self.other)

    def test_valid_denial_requires_live_kernel_link_and_second_vm(self):
        self.invoke()
        self.bridge.guard.require_live.assert_called()
        self.assertTrue(self.bridge.proved)
        self.assertFalse(self.result["joint_guest_aya_fault"]["permit_issued"])
        self.assertFalse(self.result["joint_guest_aya_fault"]["stage_039_complete"])

    def test_guest_denial_alone_cannot_mask_missing_host_aya(self):
        self.bridge.guard.require_live.side_effect = fault.preboot.PrebootAyaError("detached")
        with self.assertRaises(fault.preboot.PrebootAyaError):
            self.invoke()
        self.assertFalse(self.bridge.proved)

    def test_guest_denial_cannot_mask_permit_issuance(self):
        self.bridge.permit_issued = True
        with self.assertRaises(fault.JointFaultDenied):
            self.invoke()
        self.assertFalse(self.bridge.proved)

    def test_guest_denial_cannot_mask_second_vm_failure(self):
        with mock.patch.object(fault.live, "_bounded_console_evidence",
                               return_value=self.serial), mock.patch.object(
                                   fault.vm_egress, "require_vm_running", return_value=78):
            with self.assertRaises(fault.JointFaultDenied):
                self.bridge.prove(self.result, self.proc, "ocaf-" + "b" * 24,
                                  Path("/sys/fs/cgroup/x"), Path("/tmp/serial"), self.other)


if __name__ == "__main__":
    unittest.main()
