"""Tenant WASIp1 challenge modules must be distinct and narrowly generated."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SOURCE = Path(__file__).resolve().parents[2] / "stage039/guest-wasi/build_tenant_module.py"
SPEC = importlib.util.spec_from_file_location("stage039_tenant_module", SOURCE)
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class TenantModules(unittest.TestCase):
    def test_two_challenges_have_different_exact_wasi_output(self):
        nonce_a = "0123456789abcdef0123456789abcdef"
        nonce_b = "fedcba9876543210fedcba9876543210"
        for tenant, nonce in (("tenant-a", nonce_a), ("tenant-b", nonce_b)):
            output = module.expected_output(tenant, nonce)
            wat = module.render_wat(tenant, nonce)
            self.assertIn('"wasi_snapshot_preview1" "fd_write"', wat)
            self.assertIn(output[:-1] + r"\0a", wat)
            self.assertIn(f"(i32.const {len(output)})", wat)
            self.assertEqual(wat.count("(call $fd_write"), 1)
        self.assertNotEqual(module.render_wat("tenant-a", nonce_a),
                            module.render_wat("tenant-b", nonce_b))

    def test_rejects_invalid_tenant_or_nonce_before_wat_construction(self):
        for tenant, nonce in (("tenant-c", "a" * 32),
                              ("tenant-a", "A" * 32),
                              ("tenant-a", "a" * 31),
                              ("tenant-a", "a" * 31 + '"')):
            with self.subTest(tenant=tenant, nonce=nonce):
                with self.assertRaises(ValueError):
                    module.render_wat(tenant, nonce)

    @unittest.skipUnless(shutil.which("wat2wasm") and shutil.which("wasm-objdump"),
                         "wabt unavailable locally")
    def test_generated_modules_compile_and_import_preview1(self):
        with tempfile.TemporaryDirectory() as d:
            for tenant, nonce in (("tenant-a", "a" * 32),
                                  ("tenant-b", "b" * 32)):
                wat = Path(d) / f"{tenant}.wat"
                wasm = Path(d) / f"{tenant}.wasm"
                wat.write_text(module.render_wat(tenant, nonce), encoding="ascii")
                subprocess.run(["wat2wasm", str(wat), "-o", str(wasm)], check=True)
                imports = subprocess.check_output(["wasm-objdump", "-x", str(wasm)], text=True)
                self.assertIn("wasi_snapshot_preview1", imports)
                self.assertIn("fd_write", imports)
                self.assertNotEqual(wasm.read_bytes(), b"")


if __name__ == "__main__":
    unittest.main()
