"""Real process probes for the narrow ephemeral rollback behavior gate."""
from __future__ import annotations

import io
import tarfile
import tempfile
import unittest
from pathlib import Path

from scripts.stage042_rollback_drill import _probe, database_probe
from scripts.verify_release_bundle import ARCHIVE, BundleError


class RollbackDrillTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.bundle = Path(self.temp.name)

    def archive(self, body: bytes) -> None:
        with tarfile.open(self.bundle / ARCHIVE, "w:gz") as archive:
            member = tarfile.TarInfo("ocaf-cluster-health")
            member.size = len(body)
            member.mode = 0o755
            archive.addfile(member, io.BytesIO(body))

    def test_executes_expected_negative_cli_contract(self) -> None:
        self.archive(b'#!/bin/sh\necho "unknown option" >&2\nexit 1\n')
        self.assertEqual(_probe(self.bundle)["invalid_option_exit"], 1)

    def test_rejects_unexpected_cli_behavior(self) -> None:
        self.archive(b"#!/bin/sh\nexit 0\n")
        with self.assertRaisesRegex(BundleError, "behavior mismatch"):
            _probe(self.bundle)

    def test_rejects_missing_binary(self) -> None:
        with tarfile.open(self.bundle / ARCHIVE, "w:gz"):
            pass
        with self.assertRaises(BundleError):
            _probe(self.bundle)

    def test_postgres_probe_requires_expected_state(self) -> None:
        self.archive(b'#!/bin/sh\necho \'{"snapshot":{"jobs_pending":2}}\'\n')
        result = database_probe(self.bundle, 2, "host=127.0.0.1")
        self.assertEqual(result["jobs_pending"], 2)
        with self.assertRaisesRegex(BundleError, "state mismatch"):
            database_probe(self.bundle, 1, "host=127.0.0.1")

    def test_postgres_probe_rejects_empty_dsn(self) -> None:
        with self.assertRaisesRegex(BundleError, "configuration"):
            database_probe(self.bundle, 2, "")


if __name__ == "__main__":
    unittest.main()
