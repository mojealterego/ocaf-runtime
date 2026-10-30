"""0.56 — real Python AST and Rust CST negative security acceptance tests."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omnidev_semantic_critic import python_security_review, rust_security_review


class SemanticCriticTests(unittest.TestCase):
    def test_python_alias_to_subprocess_cannot_bypass_line_heuristic(self):
        code = "import subprocess as sp\ndef run():\n    return sp.run(['id'])\n"
        self.assertIn("DANGEROUS_CALL",
                      [x["rule"] for x in python_security_review(code)])

    def test_python_indirect_and_unsafe_calls_are_denied(self):
        code = "from os import system as call\ncall('id')\n"
        self.assertTrue(python_security_review(code))
        self.assertTrue(python_security_review("eval('2+2')\n"))
        self.assertFalse(python_security_review("def safe(x):\n    return x + 1\n"))

    def test_rust_cst_rejects_unsafe_block_but_accepts_pure_code(self):
        findings = rust_security_review(b"pub fn f() { unsafe { core::ptr::read(0 as *const u8); } }\n")
        self.assertTrue(findings)
        self.assertEqual(rust_security_review(b"pub fn f() -> u32 { 4 }\n"), [])
