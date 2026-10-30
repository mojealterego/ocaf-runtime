"""0.56 — genuine Tree-sitter CST, incremental and fail-closed tests."""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omnidev_tree_sitter import RustSymbolIndex, SyntaxIndexError, parse_rust


class RustSymbolIndexTests(unittest.TestCase):
    def test_parses_real_rust_functions_structures_and_calls(self):
        result = parse_rust(b"pub struct Engine;\npub fn compute() -> u32 { 4 }\nfn run() { compute(); }\n")
        self.assertEqual([s["name"] for s in result["symbols"]],
                         ["Engine", "compute", "run"])
        self.assertIn("compute", result["references"])
        self.assertEqual(len(result["sha256"]), 64)

    def test_malformed_rust_is_rejected_not_accepted(self):
        with self.assertRaises(SyntaxIndexError):
            parse_rust(b"pub fn incomplete( {")
        with self.assertRaises(SyntaxIndexError):
            parse_rust(b"pub fn value() {\x00}")
        with self.assertRaises(SyntaxIndexError):
            parse_rust(b"x" * (1024 * 1024 + 1))

    def test_incremental_impact_and_atomic_update(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "src").mkdir()
            (root / "src" / "lib.rs").write_text("pub fn calculate() {}\n")
            (root / "src" / "consumer.rs").write_text("fn use_it() { calculate(); }\n")
            index = RustSymbolIndex(root)
            self.assertEqual(index.update(["src/lib.rs", "src/consumer.rs"])["changed"],
                             ["src/consumer.rs", "src/lib.rs"])
            self.assertEqual(index.impacted(["src/lib.rs"])["potential_dependents"],
                             ["src/consumer.rs"])
            old = index.fingerprint()
            self.assertEqual(index.update(["src/lib.rs"])["changed"], [])
            self.assertEqual(old, index.fingerprint())
            (root / "src" / "lib.rs").write_text("pub fn broken( {\n")
            with self.assertRaises(SyntaxIndexError):
                index.update(["src/lib.rs"])
            self.assertEqual(old, index.fingerprint())
            (root / "src" / "lib.rs").unlink()
            self.assertEqual(index.update(["src/lib.rs"])["removed"], ["src/lib.rs"])

    def test_symlink_and_traversal_are_denied(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "src").mkdir()
            index = RustSymbolIndex(root)
            with self.assertRaises(SyntaxIndexError):
                index.update(["src/../unsafe.rs"])
            (root / "src" / "escape.rs").symlink_to(root / "untrusted.rs")
            with self.assertRaises(SyntaxIndexError):
                index.update(["src/escape.rs"])
