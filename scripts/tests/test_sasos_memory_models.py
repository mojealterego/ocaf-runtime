"""Stage 334 reference memory contracts: byte-exact archive and bounded surprise gate.

These are real algorithms with tests, NOT trained Titans/NICE/R3Mem models.
"""
import os
from pathlib import Path
import tempfile
import unittest
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from sasos_memory_models import LosslessTenantArchive, SurpriseGatedMemory, MemoryContractError


class LosslessArchiveTests(unittest.TestCase):
    def test_binary_roundtrip_and_tenant_isolation(self):
        with tempfile.TemporaryDirectory() as root:
            a = LosslessTenantArchive(Path(root), "tenant-a")
            b = LosslessTenantArchive(Path(root), "tenant-b")
            payload = bytes(range(256)) * 64 + b"\x00\xff\x00"
            digest = a.put(payload)
            self.assertEqual(a.get(digest), payload)
            with self.assertRaises(MemoryContractError):
                b.get(digest)
            self.assertEqual(a.put(payload), digest)
            self.assertEqual(b.put(payload), digest)
            self.assertEqual(b.get(digest), payload)

    def test_rejects_corruption_traversal_and_oversize(self):
        with tempfile.TemporaryDirectory() as root:
            archive = LosslessTenantArchive(Path(root), "tenant-a")
            digest = archive.put(b"immutable\x00payload")
            with self.assertRaises(MemoryContractError):
                archive.get("../" + digest)
            target = Path(root) / "tenant-a" / (digest + ".zlib")
            raw = target.read_bytes()
            target.write_bytes(raw[:-1] + bytes([raw[-1] ^ 1]))
            with self.assertRaises(MemoryContractError):
                archive.get(digest)
            with self.assertRaises(MemoryContractError):
                archive.put(b"z" * (4 * 1024 * 1024 + 1))
            with self.assertRaises(MemoryContractError):
                LosslessTenantArchive(Path(root), "../tenant-a")

    @unittest.skipUnless(hasattr(os, "O_NOFOLLOW"), "POSIX nofollow required")
    def test_rejects_symlinked_archive_object(self):
        with tempfile.TemporaryDirectory() as root:
            archive = LosslessTenantArchive(Path(root), "tenant-a")
            digest = archive.put(b"private")
            target = Path(root) / "tenant-a" / (digest + ".zlib")
            target.unlink()
            target.symlink_to(Path(root) / "nonexistent")
            with self.assertRaises(MemoryContractError):
                archive.get(digest)


class SurpriseMemoryTests(unittest.TestCase):
    def test_surprise_gate_is_bounded_and_tenant_scoped(self):
        memory = SurpriseGatedMemory(dimension=3, capacity=2, threshold=0.25, decay=0.5)
        self.assertTrue(memory.observe("tenant-a", "item-1", [0.0, 0.0, 0.0]))
        self.assertFalse(memory.observe("tenant-a", "item-1", [0.1, 0.1, 0.1]))
        self.assertTrue(memory.observe("tenant-a", "item-1", [1.0, 1.0, 1.0]))
        self.assertIsNone(memory.lookup("tenant-b", "item-1"))
        self.assertTrue(memory.observe("tenant-a", "item-2", [2.0, 0.0, 0.0]))
        self.assertTrue(memory.observe("tenant-a", "item-3", [3.0, 0.0, 0.0]))
        self.assertLessEqual(memory.size, 2)

    def test_checkpoint_recovery_and_invalid_vectors(self):
        memory = SurpriseGatedMemory(dimension=2, capacity=2, threshold=0.1, decay=0.5)
        memory.observe("tenant-a", "one", [1.0, 2.0])
        saved = memory.checkpoint()
        restored = SurpriseGatedMemory.restore(saved)
        self.assertEqual(restored.lookup("tenant-a", "one"), [1.0, 2.0])
        for invalid in ([float("nan"), 0.0], [1.0], [float("inf"), 1.0]):
            with self.assertRaises(MemoryContractError):
                restored.observe("tenant-a", "bad", invalid)
        with self.assertRaises(MemoryContractError):
            SurpriseGatedMemory.restore(saved[:-1] + b"0")


if __name__ == "__main__":
    unittest.main()
