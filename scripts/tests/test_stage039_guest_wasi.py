"""Fail-closed guest WASIp1 execution serial observations (not host-only Wasmtime)."""
from pathlib import Path
import hashlib, json, sys, tempfile, unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_guest_wasi as g

GOOD=(b"[  0.100] Linux version 6.1.0\n"+
      g.INIT+g.OUTPUT+g.PASS)

class GuestWasiBindings(unittest.TestCase):
    def test_real_import_and_child_exit_required(self):
        r=g.verify_console(GOOD)
        self.assertTrue(r["wasi_preview1_fd_write_observed_inside_guest"])
        self.assertEqual(r["serial_sha256"],hashlib.sha256(GOOD).hexdigest())
        for broken in (GOOD.replace(g.PASS,b""), GOOD.replace(g.OUTPUT,b""),
                GOOD.replace(g.INIT,b""), GOOD.replace(b"Linux version",b"User version"),
                GOOD+g.FAILED, GOOD+g.PASS,
                g.PASS+g.INIT+g.OUTPUT, b"",GOOD*40000):
            with self.subTest(raw=broken[:75]),self.assertRaises(g.GuestWasiError):
                g.verify_console(broken)

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
                        "wasi_executed_inside_guest":False,
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
            self.assertFalse(x["signed_wasi_manifest_verified"])
            with self.assertRaises(g.GuestWasiError):
                g.verify_brokered_pair(report,jail,"b"*64)
            report["tenant_b"]["vm_id"]=report["tenant_a"]["vm_id"]
            with self.assertRaises(g.GuestWasiError):
                g.verify_brokered_pair(report,jail,sha)
