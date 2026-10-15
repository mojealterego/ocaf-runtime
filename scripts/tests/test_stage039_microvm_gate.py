"""OCAF 0.39 fail-closed static microVM admission tests, never a launch claim."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / "stage039_microvm_gate.py"
SPEC = importlib.util.spec_from_file_location("stage039_microvm_gate", SCRIPT)
assert SPEC and SPEC.loader
stage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(stage)


class Stage039MicrovmTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name) / "approved"
        self.root.mkdir(mode=0o700)
        assets = {}
        for name in stage.REQUIRED_ASSETS:
            p = self.root / (name + ".bin")
            p.write_bytes(("static-test-" + name).encode())
            p.chmod(0o700 if name == "firecracker" else 0o600)
            assets[name] = {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
        self.manifest = {"schema_version": 1, "workload_id": "workload-a",
                         "tenant_id": "tenant-a", "agent_id": "agent-a",
                         "isolation": {"hypervisor": "firecracker", "jailer": True,
                                       "network": "none", "guest_egress": False,
                                       "rootfs_readonly": True, "mmds_enabled": False,
                                       "seccomp": "default", "vcpus": 1, "mem_mib": 256},
                         "assets": assets}

    def check(self, m=None):
        return stage.inspect_manifest(json.dumps(self.manifest if m is None else m).encode(), self.root)

    def test_positive_static_verification_never_claims_guest_or_ebpf(self):
        with mock.patch("subprocess.run") as run, mock.patch("subprocess.Popen") as popen:
            out = self.check()
        run.assert_not_called()
        popen.assert_not_called()
        self.assertEqual(out["verified_assets"], 4)
        self.assertEqual(out["status"], "STATIC_ARTIFACTS_VERIFIED_ONLY")
        for field in ("microvm_launched", "ebpf_attached", "production_approved", "stage_039_complete"):
            self.assertIs(out[field], False)

    def test_rejects_unsafe_network_mmds_seccomp_jailer_and_rw_filesystem(self):
        for key, value in (("network", "tap"), ("guest_egress", True),
                           ("mmds_enabled", True), ("rootfs_readonly", False),
                           ("jailer", False), ("seccomp", "none")):
            with self.subTest(key=key):
                data = json.loads(json.dumps(self.manifest))
                data["isolation"][key] = value
                with self.assertRaises(ValueError):
                    self.check(data)

    def test_resource_bounds_fail_closed(self):
        for field, value in (("vcpus", True), ("mem_mib", True), ("vcpus", 0),
                             ("vcpus", 5), ("mem_mib", 0), ("mem_mib", 4096)):
            with self.subTest(field=field, value=value):
                data = json.loads(json.dumps(self.manifest))
                data["isolation"][field] = value
                with self.assertRaises(ValueError):
                    self.check(data)

    def test_pinned_digest_tamper_rejected(self):
        (self.root / "guest_rootfs.bin").write_bytes(b"tampered")
        with self.assertRaises(ValueError):
            self.check()

    def test_symlink_and_hardlink_rejected(self):
        item = self.root / "guest_rootfs.bin"
        (self.root / "alias.bin").symlink_to(item)
        bad = json.loads(json.dumps(self.manifest))
        bad["assets"]["guest_rootfs"]["path"] = str(self.root / "alias.bin")
        with self.assertRaises(ValueError):
            self.check(bad)
        os.link(item, self.root / "extra-hardlink")
        with self.assertRaises(ValueError):
            self.check()

    def test_untrusted_path_writable_file_and_duplicate_paths_rejected(self):
        bad = json.loads(json.dumps(self.manifest))
        bad["assets"]["guest_kernel"]["path"] = "/etc/passwd"
        with self.assertRaises(ValueError):
            self.check(bad)
        bad = json.loads(json.dumps(self.manifest))
        bad["assets"]["guest_kernel"]["path"] = bad["assets"]["guest_rootfs"]["path"]
        with self.assertRaises(ValueError):
            self.check(bad)
        (self.root / "aya_cgroup_device.bin").chmod(0o666)
        with self.assertRaises(ValueError):
            self.check()

    def test_mutable_nested_directory_and_symlinked_parent_rejected(self):
        directory = self.root / "dir"
        directory.mkdir(mode=0o700)
        src = self.root / "guest_kernel.bin"
        nested = directory / src.name
        nested.write_bytes(src.read_bytes())
        bad = json.loads(json.dumps(self.manifest))
        bad["assets"]["guest_kernel"]["path"] = str(nested)
        self.assertEqual(self.check(bad)["verified_assets"], 4)
        directory.chmod(0o777)
        with self.assertRaises(ValueError):
            self.check(bad)
        directory.chmod(0o700)
        (self.root / "alias").symlink_to(directory, target_is_directory=True)
        bad["assets"]["guest_kernel"]["path"] = str(self.root / "alias" / src.name)
        with self.assertRaises(ValueError):
            self.check(bad)

    def test_duplicate_json_unknown_field_nan_and_oversized_denied(self):
        raw = json.dumps(self.manifest)
        for example in (raw.replace('"schema_version": 1,', '"schema_version": 1, "schema_version": 1,'),
                        raw.replace('"vcpus": 1', '"vcpus": NaN'),
                        raw.replace('"tenant_id": "tenant-a",', '"tenant_id": "tenant-a", "extra": 1,'),
                        "x" * (stage.MAX_MANIFEST_BYTES + 1)):
            with self.subTest(case=example[:40]):
                with self.assertRaises(ValueError):
                    stage.inspect_manifest(example.encode(), self.root)

    def test_no_secret_or_approved_root_path_leaks_into_report(self):
        with mock.patch.dict(os.environ, {"OCAF_SIGNING_SECRET": "DONT_REPORT_THIS"}):
            out = self.check()
        self.assertNotIn("DONT_REPORT_THIS", json.dumps(out))
        self.assertNotIn(str(self.root), json.dumps(out))

    def test_fails_host_gate_without_actual_kvm(self):
        observations = dict.fromkeys(stage.HOST_CHECKS, True)
        observations["kvm_device_character"] = False
        observations["kvm_device_accessible"] = False
        with mock.patch.object(stage, "host_observations", return_value=observations):
            report = stage.preflight()
        self.assertEqual(report["status"], "NO_GO")
        self.assertFalse(report["stage_039_complete"])
        self.assertFalse(report["production_approved"])

    def test_even_full_host_prerequisites_are_not_production_attestation(self):
        with mock.patch.object(stage, "host_observations",
                               return_value=dict.fromkeys(stage.HOST_CHECKS, True)):
            report = stage.preflight()
        self.assertEqual(report["status"], "HOST_PREREQUISITES_ONLY")
        self.assertFalse(report["microvm_launched"])
        self.assertFalse(report["ebpf_attached"])

    def test_evidence_create_once_and_private(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "report.json"
            stage.save_report(p, stage.preflight())
            self.assertEqual(p.stat().st_mode & 0o777, 0o600)
            with self.assertRaises(FileExistsError):
                stage.save_report(p, stage.preflight())


if __name__ == "__main__":
    unittest.main()
