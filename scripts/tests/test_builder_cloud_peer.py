"""Unix bridge peer authentication for Agents Builder Cloud (AB-CLOUD-CODE/NOCODE)."""
from __future__ import annotations

from contextlib import closing

import os
import socket
import struct
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.builder_cloud_peer import BridgePeerError, verify_bridge_peer


@unittest.skipUnless(sys.platform.startswith('linux') and hasattr(socket, 'SO_PEERCRED'),
                     'SO_PEERCRED requires Linux')
class BridgePeerSecurityTests(unittest.TestCase):
    def test_real_unix_socketpair_pins_expected_current_process(self):
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        with closing(a), closing(b):
            self.assertEqual(verify_bridge_peer(a, expected_pid=os.getpid()), os.getpid())
            self.assertEqual(verify_bridge_peer(b, expected_pid=None), os.getpid())

    def test_mismatched_process_is_rejected(self):
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        with closing(a), closing(b):
            with self.assertRaisesRegex(BridgePeerError, 'PID'):
                verify_bridge_peer(a, expected_pid=os.getpid() + 10000)

    def test_expected_pid_requires_real_positive_integer(self):
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        with closing(a), closing(b):
            for value in (0, -1, True, 1.5, '42'):
                with self.subTest(value=value), self.assertRaisesRegex(BridgePeerError, 'expected PID'):
                    verify_bridge_peer(a, expected_pid=value)

    def test_untrusted_peer_uid_is_rejected(self):
        fake = FakeSocket(struct.pack('3i', 43210, os.geteuid() + 1000, 1000))
        with self.assertRaisesRegex(BridgePeerError, 'UID'):
            verify_bridge_peer(fake, expected_pid=None)

    def test_invalid_peer_credentials_fail_closed(self):
        for value in (b'', b'\x00' * 4, struct.pack('3i', 0, os.geteuid(), 100)):
            with self.subTest(value=value), self.assertRaises(BridgePeerError):
                verify_bridge_peer(FakeSocket(value), expected_pid=None)

    def test_peer_credentials_syscall_failure_is_rejected(self):
        fake = FakeSocket(OSError('SO_PEERCRED failure'))
        with self.assertRaisesRegex(BridgePeerError, 'credentials'):
            verify_bridge_peer(fake, expected_pid=os.getpid())

    def test_unsupported_kernel_platform_is_denied_not_skipped(self):
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        with closing(a), closing(b):
            with patch('scripts.builder_cloud_peer.sys.platform', 'darwin'):
                with self.assertRaisesRegex(BridgePeerError, 'unavailable'):
                    verify_bridge_peer(a, expected_pid=os.getpid())

    def test_real_different_process_cannot_impersonate_expected_bridge(self):
        with tempfile.TemporaryDirectory() as tmp:
            sock_path = Path(tmp) / 'peer.sock'
            child = subprocess.Popen([
                sys.executable, '-B', '-c',
                'import socket,sys; s=socket.socket(socket.AF_UNIX);'
                's.bind(sys.argv[1]);s.listen(1);'
                'c,_=s.accept();c.recv(1);c.close();s.close()',
                str(sock_path)], stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            try:
                deadline = time.monotonic() + 3
                while not sock_path.exists() and child.poll() is None and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertIsNone(child.poll())
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
                    peer.settimeout(2)
                    peer.connect(str(sock_path))
                    self.assertEqual(verify_bridge_peer(peer, expected_pid=child.pid), child.pid)
                    with self.assertRaisesRegex(BridgePeerError, 'PID'):
                        verify_bridge_peer(peer, expected_pid=os.getpid())
                    peer.sendall(b'x')
                self.assertEqual(child.wait(timeout=3), 0, child.stderr.read().decode())
            finally:
                if child.poll() is None:
                    child.kill(); child.wait(timeout=3)
                child.stderr.close()


class FakeSocket:
    def __init__(self, value: bytes | OSError):
        self.value = value

    def getsockopt(self, level: int, option: int, size: int) -> bytes:
        if isinstance(self.value, OSError):
            raise self.value
        return self.value


class IntegrationContractTests(unittest.TestCase):
    """Guard the integration sites, not just an unused helper."""
    def test_supervisor_pins_actual_spawned_pid(self):
        root = Path(__file__).resolve().parents[1]
        source = (root / 'builder_cloud_supervisor.py').read_text()
        self.assertIn('bridge_pid=bridge.process.pid', source)

    def test_ingress_checks_credentials_after_connect_before_write(self):
        root = Path(__file__).resolve().parents[1]
        source = (root / 'builder_cloud_ingress.py').read_text()
        connection = source.index('peer.connect(str(self.config.bridge))')
        check = source.index('verify_bridge_peer(peer, expected_pid=self.config.bridge_pid)')
        send = source.index('write_frame(peer,envelope,remaining(deadline))')
        self.assertLess(connection, check)
        self.assertLess(check, send)


    def test_dedicated_cloud_workflow_triggers_on_all_peer_boundary_changes(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/stage052-builder-cloud.yml').read_text()
        for name in ('builder_cloud_ingress.py', 'builder_cloud_supervisor.py',
                     'builder_cloud_peer.py'):
            with self.subTest(name=name):
                self.assertEqual(workflow.count(f'      - "scripts/{name}"'), 2)
        for name in ('test_builder_cloud_ingress.py', 'test_builder_cloud_supervisor.py',
                     'test_builder_cloud_peer.py'):
            with self.subTest(name=name):
                self.assertEqual(workflow.count(f'      - "scripts/tests/{name}"'), 2)
        self.assertIn("-p 'test_builder_cloud_*.py'", workflow)


if __name__ == '__main__':
    unittest.main()
