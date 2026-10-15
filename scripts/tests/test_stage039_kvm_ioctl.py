"""0.39.4: prove /dev/kvm API ioctl, not merely a KVM device path."""
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import stage039_kvm_ioctl as probe
import stage039_microvm_gate as gate


class KVMIOCTLTests(unittest.TestCase):
    def device(self):
        return SimpleNamespace(st_mode=stat.S_IFCHR | 0o660,
                               st_rdev=os.makedev(10, 232))

    def test_kvm_version_and_vm_creation_fd_checked_and_closed(self):
        with mock.patch.object(probe.os, "lstat", return_value=self.device()), \
             mock.patch.object(probe.os, "open", return_value=7) as opened, \
             mock.patch.object(probe.fcntl, "ioctl", side_effect=[12, 18]) as ioctl, \
             mock.patch.object(probe.os, "close") as closed:
            data = probe.try_create_kvm_vm()
        self.assertTrue(data["api_version_verified"])
        self.assertTrue(data["kvm_vm_fd_created"])
        self.assertEqual(data["kvm_api_version"], 12)
        self.assertFalse(data["guest_booted"])
        self.assertFalse(data["firecracker_booted"])
        self.assertFalse(data["ebpf_attached"])
        self.assertEqual(ioctl.call_args_list[0].args[:2], (7, 0xAE00))
        self.assertEqual(ioctl.call_args_list[1].args[:2], (7, 0xAE01))
        closed.assert_any_call(18)
        closed.assert_any_call(7)

    def test_invalid_kvm_api_version_does_not_create_vm(self):
        with mock.patch.object(probe.os, "lstat", return_value=self.device()), \
             mock.patch.object(probe.os, "open", return_value=7), \
             mock.patch.object(probe.fcntl, "ioctl", return_value=11) as ioctl, \
             mock.patch.object(probe.os, "close") as close:
            report = probe.try_create_kvm_vm()
        self.assertFalse(report["kvm_vm_fd_created"])
        self.assertFalse(report["api_version_verified"])
        self.assertEqual(ioctl.call_count, 1)
        close.assert_called_once_with(7)

    def test_device_requires_correct_major_minor_and_no_symlink(self):
        for mode, dev in ((stat.S_IFREG | 0o600, 0),
                          (stat.S_IFLNK | 0o777, os.makedev(10,232)),
                          (stat.S_IFCHR | 0o660, os.makedev(10,231)),
                          (stat.S_IFCHR | 0o666, os.makedev(10,232))):
            with self.subTest(mode=mode, dev=dev), \
                 mock.patch.object(probe.os, "lstat",
                                   return_value=SimpleNamespace(st_mode=mode, st_rdev=dev)), \
                 mock.patch.object(probe.os, "open") as opened:
                result = probe.try_create_kvm_vm()
                self.assertFalse(result["kvm_vm_fd_created"])
                opened.assert_not_called()

    def test_kvm_create_failure_has_no_vm_claim(self):
        with mock.patch.object(probe.os, "lstat", return_value=self.device()), \
             mock.patch.object(probe.os, "open", return_value=7), \
             mock.patch.object(probe.fcntl, "ioctl", side_effect=[12, OSError("EPERM")]), \
             mock.patch.object(probe.os, "close") as close:
            r = probe.try_create_kvm_vm()
        self.assertFalse(r["kvm_vm_fd_created"])
        self.assertTrue(r["api_version_verified"])
        close.assert_called_once_with(7)

    def test_full_host_still_does_not_prove_firecracker_or_aya(self):
        with mock.patch.object(probe.gate, "host_observations",
                               return_value=dict.fromkeys(gate.HOST_CHECKS, True)), \
             mock.patch.object(probe, "try_create_kvm_vm",
                               return_value={"kvm_vm_fd_created": True,
                                             "api_version_verified": True, "kvm_api_version": 12,
                                             "guest_booted": False, "firecracker_booted": False,
                                             "ebpf_attached": False}):
            report = probe.collect()
        self.assertEqual(report["status"], "KVM_IOCTL_VM_CREATED_NOT_BOOTED")
        self.assertIs(report["production_approved"], False)
        self.assertIs(report["microvm_launched"], False)
        self.assertIs(report["ebpf_attached"], False)

    def test_real_host_missing_kvm_reports_no_go_not_success(self):
        observations = dict.fromkeys(gate.HOST_CHECKS, True)
        observations["kvm_device_accessible"] = False
        with mock.patch.object(probe.gate, "host_observations",
                               return_value=observations), \
             mock.patch.object(probe, "try_create_kvm_vm") as ioctl:
            r = probe.collect()
        ioctl.assert_not_called()
        self.assertEqual(r["status"], "NO_GO")
        self.assertFalse(r["kvm_vm_fd_created"])

    def test_empty_or_malformed_host_observations_fail_closed(self):
        with mock.patch.object(probe.gate, "host_observations", return_value={}):
            with self.assertRaises(ValueError):
                probe.collect()

    def test_report_private_create_only(self):
        with tempfile.TemporaryDirectory() as work:
            p = Path(work) / "host.json"
            gate.save_report(p, {"status": "NO_GO", "production_approved": False})
            self.assertEqual(p.stat().st_mode & 0o777, 0o600)
            with self.assertRaises(FileExistsError):
                gate.save_report(p, {"status": "NO_GO"})


if __name__ == "__main__":
    unittest.main()
