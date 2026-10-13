"""Prove fail-closed invariants of the resolved Zenoh feature graph."""
import importlib.util
from pathlib import Path
import sys
import unittest

MODULE = Path(__file__).resolve().parents[1] / "zenoh_feature_policy.py"
spec = importlib.util.spec_from_file_location("zenoh_feature_policy", MODULE)
checker = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = checker
spec.loader.exec_module(checker)


def metadata(*, zenoh=None, transport=None):
    if zenoh is None:
        zenoh = ["transport_tcp", "transport_tls"]
    if transport is None:
        transport = ["transport_tcp", "transport_tls"]
    return {
        "packages": [
            {"name": "zenoh", "id": "registry+zenoh#1.10.1"},
            {"name": "zenoh-transport", "id": "registry+zenoh-transport#1.10.1"},
        ],
        "resolve": {"nodes": [
            {"id": "registry+zenoh#1.10.1", "features": zenoh},
            {"id": "registry+zenoh-transport#1.10.1", "features": transport},
        ]},
    }


class FeaturePolicyTests(unittest.TestCase):
    def test_accepts_tcp_and_tls_exclusively(self):
        report = checker.inspect(metadata())
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["packages_checked"], 2)

    def test_rejects_compression_on_zenoh(self):
        with self.assertRaises(checker.PolicyError):
            checker.inspect(metadata(
                zenoh=["transport_tcp", "transport_tls", "transport_compression"]
            ))

    def test_rejects_compression_on_zenoh_transport(self):
        with self.assertRaises(checker.PolicyError):
            checker.inspect(metadata(
                transport=["transport_tcp", "transport_tls", "transport_compression"]
            ))

    def test_rejects_public_key_auth_and_password_auth(self):
        for flag in ("auth_pubkey", "auth_usrpwd"):
            with self.subTest(flag=flag), self.assertRaises(checker.PolicyError):
                checker.inspect(metadata(
                    zenoh=["transport_tcp", "transport_tls", flag]
                ))

    def test_rejects_unexpected_transport_features(self):
        for flag in (
            "transport_quic", "transport_ws", "transport_udp", "transport_multilink"
        ):
            with self.subTest(flag=flag), self.assertRaises(checker.PolicyError):
                checker.inspect(metadata(
                    transport=["transport_tcp", "transport_tls", flag]
                ))

    def test_requires_tls_and_tcp_at_both_layers(self):
        for layer in ("zenoh", "transport"):
            for needed in ("transport_tcp", "transport_tls"):
                kwargs = {layer: ["transport_tcp", "transport_tls"]}
                kwargs[layer].remove(needed)
                with self.subTest(
                    layer=layer, needed=needed
                ), self.assertRaises(checker.PolicyError):
                    checker.inspect(metadata(**kwargs))

    def test_denies_missing_feature_graph(self):
        malformed = (
            {}, {"packages": [], "resolve": {"nodes": []}},
            {"packages": [], "resolve": None},
            {"packages": "wrong", "resolve": {"nodes": []}},
        )
        for value in malformed:
            with self.subTest(metadata=value), self.assertRaises(checker.PolicyError):
                checker.inspect(value)

    def test_denies_unresolved_or_duplicate_nodes(self):
        wrong = metadata()
        wrong["resolve"]["nodes"].pop()
        with self.assertRaises(checker.PolicyError):
            checker.inspect(wrong)
        duplicate = metadata()
        duplicate["resolve"]["nodes"].append(
            duplicate["resolve"]["nodes"][0]
        )
        with self.assertRaises(checker.PolicyError):
            checker.inspect(duplicate)

    def test_denies_non_string_features(self):
        with self.assertRaises(checker.PolicyError):
            checker.inspect(metadata(
                zenoh=["transport_tcp", "transport_tls", 12]
            ))


if __name__ == "__main__":
    unittest.main()
