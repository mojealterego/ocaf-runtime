"""Contract checks for Firecracker launch and runtime proof."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_live_firecracker as live


class LiveContract(unittest.TestCase):
    def test_uid_gid_must_not_be_root(self):
        with self.assertRaises(live.LiveFirecrackerError):
            live.build_jailer_command(Path("/opt/bin/jailer"), Path("/opt/bin/firecracker"),
                "ocaf-test-a", 0, 1002, Path("/srv/ocaf-jailer"),
                "ocaf-stage039-vm-0123456789abcdef.service")

    def test_jailer_command_requires_cgroup2_and_chroot(self):
        cmd = live.build_jailer_command(Path("/opt/bin/jailer"), Path("/opt/bin/firecracker"),
            "ocaf-test-a", 1002, 1002, Path("/srv/ocaf-jailer"),
            "ocaf-stage039-vm-0123456789abcdef.service")
        self.assertIn("--cgroup-version", cmd)
        self.assertIn("--parent-cgroup", cmd)
        self.assertIn("--chroot-base-dir", cmd)
        self.assertIn("--config-file", cmd)
        self.assertIn("/vmconfig.json", cmd)
        self.assertNotIn("--daemonize", cmd)

    def test_running_status_alone_does_not_prove_kernel_boot(self):
        with self.assertRaises(live.LiveFirecrackerError):
            live.validate_running(
                {"state": "Running", "id": "ocaf-test-a"}, "",
                "0::/ocaf-stage039-vm-0123456789abcdef.service",
                "ocaf-test-a", "ocaf-stage039-vm-0123456789abcdef.service")

    def test_must_be_pinned_to_expected_process_cgroup(self):
        with self.assertRaises(live.LiveFirecrackerError):
            live.validate_running(
                {"state": "Running", "id": "ocaf-test-a"}, "Linux version 6",
                "0::/system.slice/other.service",
                "ocaf-test-a", "ocaf-stage039-vm-0123456789abcdef.service")

    def test_kernel_boot_proof_does_not_certify_full_isolation(self):
        status = live.validate_running(
            {"state": "Running", "id": "ocaf-test-a"},
            "Booting Linux\nLinux version 6.6.1",
            "0::/ocaf-stage039-vm-0123456789abcdef.service",
            "ocaf-test-a", "ocaf-stage039-vm-0123456789abcdef.service")
        self.assertEqual(status["status"], "JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED")
        self.assertFalse(status["ebpf_attached"])
        self.assertFalse(status["production_approved"])

    def test_wrong_vm_identity_rejected(self):
        with self.assertRaises(live.LiveFirecrackerError):
            live.validate_running(
                {"state": "Running", "id": "other"}, "Linux version 6",
                "0::/ocaf-stage039-vm-0123456789abcdef.service",
                "ocaf-test-a", "ocaf-stage039-vm-0123456789abcdef.service")


    def test_linux_console_timestamped_kernel_version_is_valid_boot_evidence(self):
        report=live.validate_running(
            {"state":"Running","id":"ocaf-test-a"},
            "[    0.000000] Linux version 6.6.15 (builder@host)\n",
            "0::/ocaf-stage039-vm-0123456789abcdef.service/firecracker/ocaf-test-a",
            "ocaf-test-a","ocaf-stage039-vm-0123456789abcdef.service")
        self.assertTrue(report["guest_kernel_console_seen"])


    def test_guest_staging_is_readable_to_unprivileged_jailer_uid_even_with_private_umask(self):
        import hashlib
        import os
        import tempfile
        import stat
        with tempfile.TemporaryDirectory() as base:
            source=Path(base)/"kernel.src"
            dest=Path(base)/"kernel.dest"
            source.write_bytes(b"test-guest-image-data")
            expected=hashlib.sha256(source.read_bytes()).hexdigest()
            oldmask=os.umask(0o077)
            try:
                live._copy_pinned(source,dest,expected)
            finally:
                os.umask(oldmask)
            self.assertEqual(stat.S_IMODE(dest.stat().st_mode),0o444)


    def test_staging_parent_modes_fail_closed_under_permissive_umask(self):
        import os,stat,tempfile
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)/"firecracker"/"vm-id"/"root"
            before=os.umask(0o002)
            try:
                live._secure_staging_directories(root)
            finally:
                os.umask(before)
            self.assertEqual(stat.S_IMODE(root.parent.parent.stat().st_mode),0o700)
            self.assertEqual(stat.S_IMODE(root.parent.stat().st_mode),0o700)
            self.assertEqual(stat.S_IMODE(root.stat().st_mode),0o755)
            self.assertEqual(stat.S_IMODE((root/"guest").stat().st_mode),0o755)


    def test_large_guest_serial_logs_keep_initial_kernel_boot_line(self):
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/"console.log"
            output.write_text("[    0.000000] Linux version 4.14.55\n" +
                              ("[api get] 200 OK\n"*12000) +
                              "Welcome to Alpine Linux\n")
            bounded=live._bounded_console_evidence(output)
            self.assertIn("Linux version 4.14.55",bounded)
            self.assertIn("Welcome to Alpine Linux",bounded)
            self.assertLess(len(bounded),80000)


if __name__ == "__main__":
    unittest.main()
