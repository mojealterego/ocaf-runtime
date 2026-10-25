"""Two independent Linux kernels must replay a fresh SQL nonce over live WAL."""
import json
import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage040_live_twohost_probe as gate


class FakeRemote:
    boots = {"a": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
             "b": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"}

    def __init__(self):
        self.streaming = True
        self.replayed = True
        self.calls = []

    def __call__(self, node, known_hosts, argv, *, timeout):
        self.calls.append((node["role"], argv))
        role = node["role"]
        if argv[0] == "cat":
            return self.boots[role]
        query = argv[-1]
        if query == "SELECT pg_is_in_recovery()":
            return "f" if role == "a" else "t"
        if "FROM pg_stat_replication" in query:
            return "10.60.0.2" if self.streaming else ""
        if "FROM pg_stat_wal_receiver" in query:
            return json.dumps({"status": "streaming" if self.streaming else "stopped",
                               "sender_host": "10.60.0.1", "sender_port": 5432})
        if query.startswith("CREATE TABLE") or query.startswith("INSERT INTO"):
            return ""
        if query == "SELECT pg_current_wal_lsn()":
            return "0/100"
        if query == "SELECT pg_last_wal_replay_lsn()":
            return "0/101" if self.replayed else "0/FF"
        if query.startswith("SELECT nonce ||"):
            return "a" * 32 + ":" + self.boots["a"]
        raise AssertionError(query)


class LiveTwoHostGate(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.pins = self.root / "known_hosts"
        self.pins.write_text("10.60.0.1 ssh-ed25519 " + "A" * 60 + "\n"
                             "10.60.0.2 ssh-ed25519 " + "B" * 60 + "\n")
        self.pins.chmod(0o600)
        self.inventory = {
            "schema_version": 1, "database": "ocaf_stage040_lab",
            "postgres_port": 5432, "postgres_socket_dir": "/var/run/postgresql",
            "nodes": {
                role: {"role": role, "host": address, "ssh_user": "pgops",
                       "ssh_port": 22, "replication_address": address}
                for role, address in (("a", "10.60.0.1"), ("b", "10.60.0.2"))}}
        self.remote = FakeRemote()

    def invoke(self):
        return gate.run(self.inventory, self.pins, self.root / "receipt.json",
                        ssh=self.remote, nonce="a" * 32, timeout_seconds=.01)

    def test_two_host_streaming_challenge_proves_only_lab_tcp(self):
        result = self.invoke()
        self.assertTrue(result["direct_tcp_stream_observed"])
        self.assertFalse(result["stage_040_complete"])
        self.assertFalse(result["fencing_tested"])
        self.assertEqual(result["fresh_nonce"], "a" * 32)
        self.assertEqual(json.loads((self.root / "receipt.json").read_text()), result)
        self.assertTrue(any("INSERT INTO" in argv[-1] for _, argv in self.remote.calls))

    def test_reused_kernel_or_missing_streaming_is_denied(self):
        self.remote.boots = {"a": FakeRemote.boots["a"], "b": FakeRemote.boots["a"]}
        with self.assertRaises(gate.InterhostPgError):
            self.invoke()
        self.assertFalse((self.root / "receipt.json").exists())
        self.remote.boots = dict(FakeRemote.boots)
        self.remote.streaming = False
        with self.assertRaises(gate.InterhostPgError):
            self.invoke()

    def test_artifact_without_replayed_wal_cannot_pass(self):
        self.remote.replayed = False
        with self.assertRaises(gate.InterhostPgError):
            self.invoke()
        self.assertFalse((self.root / "receipt.json").exists())

    def test_unpinned_or_duplicate_remote_host_is_denied(self):
        self.inventory["nodes"]["b"]["host"] = "10.60.0.1"
        with self.assertRaises(gate.InterhostPgError):
            self.invoke()
        self.inventory["nodes"]["b"]["host"] = "10.60.0.2"
        self.pins.write_text("10.60.0.1 ssh-ed25519 " + "A" * 60 + "\n")
        with self.assertRaises(gate.InterhostPgError):
            self.invoke()


if __name__ == "__main__":
    unittest.main()
