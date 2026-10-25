"""Fail-closed parsing of the actual guest canary result."""
from __future__ import annotations

from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_guest_host_canary as canary


class GuestHostCanaryTests(unittest.TestCase):
    def test_exact_guest_denial_after_kernel_boot_is_accepted(self):
        nonce = "a" * 32
        canary.require_guest_marker(
            "[    0.000000] Linux version 6.1.188\n"
            "OCAF039_GUEST_HOST_CANARY_DENIED:" + nonce + "\n", nonce)

    def test_missing_or_wrong_nonce_never_passes(self):
        nonce = "a" * 32
        for serial in ("", "OCAF039_GUEST_HOST_CANARY_DENIED:" + "b" * 32,
                       "xOCAF039_GUEST_HOST_CANARY_DENIED:" + nonce,
                       "OCAF039_GUEST_HOST_CANARY_DENIED:" + nonce + "suffix"):
            with self.subTest(serial=serial), self.assertRaises(canary.CanaryDenied):
                canary.require_guest_marker(serial, nonce)

    def test_guest_successful_forbidden_open_fails_even_with_denial_line(self):
        nonce = "a" * 32
        with self.assertRaises(canary.CanaryDenied):
            canary.require_guest_marker(
                "OCAF039_GUEST_HOST_CANARY_DENIED:" + nonce + "\n"
                "OCAF039_GUEST_HOST_CANARY_FAIL:" + nonce + "\n", nonce)


if __name__ == "__main__":
    unittest.main()
