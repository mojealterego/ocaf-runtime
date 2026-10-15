"""Contract checks for Firecracker launch and runtime proof."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_live_firecracker as live


class LiveContract(unittest.TestCase):
    def test_guest_wasi_child_must_finish_before_firecracker_is_sigkilled(self):
        import tempfile, threading, time
        from stage039_guest_wasi import INIT, OUTPUT, PASS, FAILED
        class Live:
            def poll(self): return None
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/"serial-console.log"
            path.write_bytes(b"[    0.00] Linux version 6.1.0\n"+INIT+
                             b"OCAF039_GUEST_CRNG_READY\n")
            def finish():
                time.sleep(.12)
                with path.open("ab") as f: f.write(OUTPUT+PASS)
            t=threading.Thread(target=finish,daemon=True)
            t.start()
            try:
                evidence=live.await_real_guest_wasi_execution(Live(),path,seconds=2)
                self.assertTrue(evidence["wasi_preview1_fd_write_observed_inside_guest"])
                self.assertTrue(evidence["guest_wasmtime_child_exited_zero"])
            finally:t.join(timeout=1)
            path.write_bytes(b"[    0.00] Linux version 6.1.0\n"+INIT+FAILED)
            with self.assertRaisesRegex(live.LiveFirecrackerError,"failed"):
                live.await_real_guest_wasi_execution(Live(),path,seconds=1)
            path.write_bytes(b"[    0.00] Linux version 6.1.0\n"+INIT)
            with self.assertRaisesRegex(live.LiveFirecrackerError,"deadline"):
                live.await_real_guest_wasi_execution(Live(),path,seconds=1)

    def test_guest_wasi_wait_rejects_dead_vm(self):
        import tempfile
        class Dead:
            def poll(self): return 143
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/"serial"
            path.write_bytes(b"Linux version 6.1\n")
            with self.assertRaisesRegex(live.LiveFirecrackerError,"exited"):
                live.await_real_guest_wasi_execution(Dead(),path,seconds=1)

    def test_uid_gid_must_not_be_root(self):
        with self.assertRaises(live.LiveFirecrackerError):
            live.build_jailer_command(Path("/opt/bin/jailer"), Path("/opt/bin/firecracker"),
                "ocaf-test-a", 0, 1002, Path("/srv/ocaf-jailer"),
                "ocaf-stage039-vm-0123456789abcdef.service")

    def test_jailer_api_socket_path_must_fit_linux_sockaddr_un_limit(self):
        ident = "ocaf-" + "a" * 24
        unsafe = Path("/opt/ocaf039-independent-38720117864-tenant-a/jailer")
        safe = Path("/opt/o39h38720117864-a/jailer")
        with self.assertRaisesRegex(live.LiveFirecrackerError, "sockaddr_un"):
            live.validate_api_socket_path(unsafe, "firecracker", ident)
        result = live.validate_api_socket_path(safe, "firecracker", ident)
        self.assertTrue(str(result).endswith("/root/firecracker.socket"))
        self.assertLessEqual(len(str(result).encode()), 107)

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


    def test_firecracker_api_response_uses_content_length_without_waiting_for_eof(self):
        import json,socket,threading,time,tempfile
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/"api.sock"
            listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
            listener.bind(str(path))
            listener.listen(1)
            response={"id":"ocaf-test-a","state":"Running","vmm_version":"1.17.0"}
            raw=json.dumps(response).encode()
            def serve():
                try:
                    peer,_=listener.accept()
                    with peer:
                        peer.recv(4096)
                        peer.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: "+
                                     str(len(raw)).encode()+b"\r\nContent-Type: application/json\r\n\r\n"+raw)
                        time.sleep(1.2)
                finally:
                    listener.close()
            worker=threading.Thread(target=serve,daemon=True)
            worker.start()
            try:
                start=time.monotonic()
                result=live._api_state(path)
                self.assertEqual(result,response)
                self.assertLess(time.monotonic()-start,1.0)
            finally:
                worker.join(timeout=3)


if __name__ == "__main__":
    unittest.main()
