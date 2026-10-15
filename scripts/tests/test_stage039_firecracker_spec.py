"""0.39.5 jailer rootfs/kernel contract: offline config, NOT VM boot."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_firecracker_spec as cfg
import stage039_microvm_gate as gate


class FirecrackerSpecTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        self.approved = root / "approved"
        self.approved.mkdir(mode=0o700)
        self.chroot = root / "jail"
        self.chroot.mkdir(mode=0o700)
        (self.chroot / "guest").mkdir(mode=0o700)
        assets = {}
        for name in gate.REQUIRED_ASSETS:
            p = self.approved / (name + ".bin")
            p.write_bytes(("reviewed-" + name).encode())
            p.chmod(0o700 if name == "firecracker" else 0o600)
            assets[name] = {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
        for key, filename in (("guest_kernel", "vmlinux.bin"),
                              ("guest_rootfs", "rootfs.ext4")):
            dst = self.chroot / "guest" / filename
            dst.write_bytes((self.approved / (key + ".bin")).read_bytes())
            dst.chmod(0o600)
        self.manifest = json.dumps({
            "schema_version": 1, "workload_id": "alpha",
            "tenant_id": "tenant-a", "agent_id": "agent-a",
            "isolation": {"hypervisor": "firecracker", "jailer": True,
                          "network": "none", "guest_egress": False,
                          "rootfs_readonly": True, "mmds_enabled": False,
                          "seccomp": "default", "vcpus": 1, "mem_mib": 256},
            "assets": assets
        }).encode()

    def test_valid_reviewed_artifacts_produce_no_network_readonly_spec(self):
        report = cfg.build(self.manifest, self.approved, self.chroot)
        spec = report["firecracker_config"]
        self.assertEqual(spec["boot-source"]["kernel_image_path"], "/guest/vmlinux.bin")
        self.assertEqual(spec["machine-config"]["vcpu_count"], 1)
        self.assertEqual(spec["network-interfaces"], [])
        self.assertTrue(spec["drives"][0]["is_read_only"])
        self.assertNotIn("vsock", spec)
        self.assertNotIn("mmds-config", spec)
        self.assertEqual(report["status"], "JAILED_STATIC_CONFIG_VERIFIED_NOT_BOOTED")
        self.assertFalse(report["firecracker_booted"])
        self.assertFalse(report["aya_attached"])
        self.assertFalse(report["production_approved"])

    def test_reject_modified_staged_kernel_and_rootfs(self):
        for filename in ("vmlinux.bin", "rootfs.ext4"):
            p = self.chroot / "guest" / filename
            before = p.read_bytes()
            p.write_bytes(b"changed")
            with self.assertRaises(ValueError):
                cfg.build(self.manifest, self.approved, self.chroot)
            p.write_bytes(before)

    def test_reject_jailed_symlink_and_unsafe_mutable_dir(self):
        p = self.chroot / "guest" / "vmlinux.bin"
        p.unlink()
        p.symlink_to(self.approved / "guest_kernel.bin")
        with self.assertRaises(ValueError):
            cfg.build(self.manifest, self.approved, self.chroot)
        p.unlink()
        p.write_bytes((self.approved / "guest_kernel.bin").read_bytes())
        p.chmod(0o600)
        (self.chroot / "guest").chmod(0o777)
        with self.assertRaises(ValueError):
            cfg.build(self.manifest, self.approved, self.chroot)

    def test_source_artifact_digest_tamper_denied(self):
        (self.approved / "firecracker.bin").write_bytes(b"altered")
        with self.assertRaises(ValueError):
            cfg.build(self.manifest, self.approved, self.chroot)

    def test_manifest_cannot_enable_network_or_writable_rootfs(self):
        for field, newval in (("network", "tap"), ("guest_egress", True),
                              ("rootfs_readonly", False), ("mmds_enabled", True)):
            m = json.loads(self.manifest)
            m["isolation"][field] = newval
            with self.assertRaises(ValueError):
                cfg.build(json.dumps(m).encode(), self.approved, self.chroot)

    def test_generating_spec_does_not_execute_process(self):
        import subprocess
        from unittest.mock import patch
        with patch.object(subprocess, "run") as run, patch.object(subprocess, "Popen") as spawn:
            cfg.build(self.manifest, self.approved, self.chroot)
        run.assert_not_called()
        spawn.assert_not_called()


if __name__ == "__main__":
    unittest.main()
