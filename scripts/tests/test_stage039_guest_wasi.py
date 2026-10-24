"""Fail-closed guest WASIp1 execution serial observations (not host-only Wasmtime)."""
from pathlib import Path
import hashlib, json, sys, tempfile, unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_guest_wasi as g

GOOD=(b"[  0.100] Linux version 6.1.0\n"+
      g.INIT+b"OCAF039_GUEST_CRNG_READY_AFTER_CPU_RNG\n"+
      b"OCAF039_GUEST_BPF_SOCKET_FILTER_ID=17\n"+g.OUTPUT+g.PASS)

class GuestWasiBindings(unittest.TestCase):
    def test_tenant_nonce_and_guest_module_digest_are_bound_to_serial(self):
        wasi={"task_nonce":"a"*32,"module_sha256":"b"*64}
        raw=(b"Linux version 6.1.0\n"+g.INIT+g.RNG_READY[0]+
             b"OCAF039_GUEST_BPF_SOCKET_FILTER_ID=17\n"+
             b"OCAF039_GUEST_WASI_MODULE_SHA256="+b"b"*64+b"\n"+
             b"OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE:tenant-a:"+b"a"*32+b"\n"+
             b"OCAF039_GUEST_WASI_TENANT=tenant-a\n"+
             b"OCAF039_GUEST_WASI_NONCE="+b"a"*32+b"\n"+g.PASS)
        receipt=g.verify_console(raw,expected_wasi=wasi,tenant_id="tenant-a")
        self.assertTrue(receipt["guest_module_digest_checked"])
        self.assertEqual(receipt["task_nonce"],wasi["task_nonce"])
        for broken in (raw.replace(b"b"*64,b"c"*64),
                       raw.replace(b"tenant-a",b"tenant-b"),
                       raw.replace(b"a"*32,b"d"*32),
                       raw.replace(b"OCAF039_GUEST_WASI_MODULE_SHA256=",b"")):
            with self.subTest(raw=broken[-170:]),self.assertRaises(g.GuestWasiError):
                g.verify_console(broken,expected_wasi=wasi,tenant_id="tenant-a")

    def test_real_import_and_child_exit_required(self):
        r=g.verify_console(GOOD)
        self.assertTrue(r["wasi_preview1_fd_write_observed_inside_guest"])
        self.assertEqual(r["serial_sha256"],hashlib.sha256(GOOD).hexdigest())
        self.assertEqual(r["guest_kernel_bpf_program_id"],17)
        for broken in (GOOD.replace(b"OCAF039_GUEST_CRNG_READY_AFTER_CPU_RNG\n",b""), GOOD.replace(g.PASS,b""), GOOD.replace(g.OUTPUT,b""),
                GOOD.replace(g.INIT,b""), GOOD.replace(b"Linux version",b"User version"),
                GOOD+g.FAILED, GOOD+g.PASS,
                GOOD.replace(b"OCAF039_GUEST_BPF_SOCKET_FILTER_ID=17\n",b""),
                g.PASS+g.INIT+g.OUTPUT, b"",GOOD*40000):
            with self.subTest(raw=broken[:75]),self.assertRaises(g.GuestWasiError):
                g.verify_console(broken)

    def test_actual_tty_crlf_is_accepted_without_changing_raw_serial_hash(self):
        serial=GOOD.replace(b"\n",b"\r\n")
        receipt=g.verify_console(serial)
        self.assertEqual(receipt["serial_sha256"],hashlib.sha256(serial).hexdigest())
        self.assertEqual(receipt["guest_kernel_bpf_program_id"],17)

    def test_reject_forged_linux_marker_after_guest_success(self):
        forged=g.INIT+b"OCAF039_GUEST_CRNG_READY\\n"+g.OUTPUT+g.PASS+b"Linux version 6.1.0\\n"
        with self.assertRaises(g.GuestWasiError):
            g.verify_console(forged)

    def test_reject_ambiguous_linux_boot_markers(self):
        duplicate=b"Linux version 6.1.0\\n"+GOOD
        with self.assertRaises(g.GuestWasiError):
            g.verify_console(duplicate)

    def test_reject_marker_embedded_in_untrusted_text(self):
        raw=(b"[ 0.1] Linux version 6.1\\n"+g.INIT+
             b"OCAF039_GUEST_CRNG_READY\\n"+
             b"attacker-prefix-"+g.OUTPUT+g.PASS)
        with self.assertRaises(g.GuestWasiError):
            g.verify_console(raw)

    def test_reject_embedded_success_marker_with_real_newlines(self):
        raw=(b"Linux version 6.1"+bytes([10])+g.INIT+
             g.RNG_READY[0]+b"prefix:"+g.OUTPUT+g.PASS)
        with self.assertRaises(g.GuestWasiError):
            g.verify_console(raw)

    def test_broker_two_identity_binding_and_digests(self):
        with tempfile.TemporaryDirectory() as root:
            root=Path(root);jail=root/"jailer";jail.mkdir()
            sha="a"*64
            report={"status":"TWO_CONCURRENT_TENANTS_VM_BOOT_AND_FAULT_ISOLATION_PASSED",
                    "production_approved":False,"stage_039_complete":False,
                    "broker_vm_binding":{
                        "status":"BROKER_SO_PEERCRED_TWO_REAL_JAILED_VM_BOOT_BOUND",
                        "real_kernel_so_peercred_authenticated_broker":True,
                        "durable_sqlite_leases_matched":True,
                        "admission_before_vm_launch":True,
                        "durable_broker_leases_released_after_vm_crashes":True,
                        "wasi_executed_inside_guest":True,
                        "signed_wasi_manifest_verified":True,
                        "production_approved":False}}
            for tenant,c in (("a","a"),("b","b")):
                vm="ocaf-"+c*24
                path=jail/"firecracker"/vm
                path.mkdir(parents=True)
                (path/"serial-console.log").write_bytes(GOOD)
                report["tenant_"+tenant]={"tenant_id":"tenant-"+tenant,
                    "vm_id":vm,"guest_rootfs_sha256":sha,
                    "firecracker_api_running":True,"guest_kernel_console_seen":True,
                    "process_in_expected_cgroup":True,"production_approved":False}
            x=g.verify_brokered_pair(report,jail,sha)
            self.assertEqual(x["status"],
                 "TWO_REAL_BROKERED_JAILED_FIRECRACKER_GUESTS_WASIP1_FD_WRITE_EXECUTED")
            self.assertTrue(x["signed_wasi_manifest_verified"])
            with self.assertRaises(g.GuestWasiError):
                g.verify_brokered_pair(report,jail,"b"*64)
            report["tenant_b"]["vm_id"]=report["tenant_a"]["vm_id"]
            with self.assertRaises(g.GuestWasiError):
                g.verify_brokered_pair(report,jail,sha)
            report["tenant_b"]["vm_id"]="ocaf-"+"b"*24
            original=jail/"firecracker"/("ocaf-"+"a"*24)
            outside=root/"outside"
            original.rename(outside)
            original.symlink_to(outside, target_is_directory=True)
            with self.assertRaisesRegex(g.GuestWasiError,"redirected"):
                g.verify_brokered_pair(report,jail,sha)
            original.unlink()
            original.mkdir()
            (original/"serial-console.log").write_bytes(GOOD)
            (jail/"firecracker").rename(jail/"firecracker-old")
            (jail/"firecracker").symlink_to(jail/"firecracker-old", target_is_directory=True)
            with self.assertRaisesRegex(g.GuestWasiError,"redirected"):
                g.verify_brokered_pair(report,jail,sha)
