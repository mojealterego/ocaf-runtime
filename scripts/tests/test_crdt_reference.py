"""Independent deterministic reference-model checks for the OCAF causal register.

This exercises the protocol model in Python; it does NOT execute Rust code.
"""
from __future__ import annotations

from dataclasses import dataclass
from itertools import permutations
from hashlib import sha256
import json
import random
import unittest


@dataclass(frozen=True)
class Op:
    tenant: str
    actor: str
    counter: int
    key: str
    value: bytes | None
    context: tuple[tuple[str, int], ...]

    def context_count(self, actor: str) -> int:
        return dict(self.context).get(actor, 0)


class Replica:
    def __init__(self, tenant: str):
        self.tenant = tenant
        self.operations: dict[tuple[str, int], Op] = {}
        self.observed: dict[str, int] = {}

    def apply(self, op: Op) -> bool:
        if op.tenant != self.tenant:
            raise ValueError("tenant mismatch")
        if op.counter < 1 or op.context_count(op.actor) != op.counter - 1:
            raise ValueError("invalid dot")
        key = (op.actor, op.counter)
        if key in self.operations:
            if self.operations[key] != op:
                raise ValueError("dot collision")
            return False
        if any(op.context_count(old.actor) >= old.counter and old.context_count(op.actor) >= op.counter
               for old in self.operations.values()):
            raise ValueError("inconsistent causal context")
        self.operations[key] = op
        for actor, counter in op.context:
            self.observed[actor] = max(self.observed.get(actor, 0), counter)
        self.observed[op.actor] = max(self.observed.get(op.actor, 0), op.counter)
        return True

    def local(self, actor: str, key: str, value: bytes | None) -> Op:
        op = Op(self.tenant, actor, self.observed.get(actor, 0) + 1, key, value,
                tuple(sorted(self.observed.items())))
        self.apply(op)
        return op

    def merge(self, other: Replica) -> None:
        if self.tenant != other.tenant:
            raise ValueError("tenant mismatch")
        candidate = Replica(self.tenant)
        candidate.operations = self.operations.copy()
        candidate.observed = self.observed.copy()
        for op in other.operations.values():
            candidate.apply(op)
        self.operations, self.observed = candidate.operations, candidate.observed

    def snapshot(self) -> dict[str, str]:
        result = {}
        for key in {op.key for op in self.operations.values()}:
            events = [op for op in self.operations.values() if op.key == key]
            maxima = [op for op in events if not any(
                (other.actor, other.counter) != (op.actor, op.counter)
                and other.context_count(op.actor) >= op.counter for other in events)]
            if not maxima:
                raise AssertionError("no causal maximum")
            winner = max(maxima, key=lambda op: (op.counter, op.actor))
            if winner.value is not None:
                result[key] = winner.value.hex()
        return dict(sorted(result.items()))


class CrdtReferenceTests(unittest.TestCase):
    def test_permutation_convergence(self):
        events = (
            Op("t", "a", 1, "k", b"a", ()),
            Op("t", "b", 1, "k", None, (("a", 1),)),
            Op("t", "c", 1, "k", b"c", ()),
            Op("t", "d", 1, "other", b"d", ()),
        )
        snapshots = set()
        for order in permutations(events):
            r = Replica("t")
            for op in order:
                r.apply(op)
            snapshots.add(json.dumps(r.snapshot(), sort_keys=True))
        self.assertEqual(len(snapshots), 1)
        self.assertEqual(json.loads(next(iter(snapshots))), {"k": "63", "other": "64"})

    def test_reconciliation_1000_seeds(self):
        h = sha256()
        cases = 0
        for seed in range(1000):
            rng = random.Random(seed)
            replicas = [Replica("tenant") for _ in range(3)]
            for i in range(3):
                replicas[i].local(f"actor{i}", "shared", bytes([i]))
            for _ in range(4):
                i, j = rng.sample(range(3), 2)
                replicas[i].merge(replicas[j])
            for i in range(3):
                replicas[i].local(
                    f"actor{i}", "shared",
                    None if rng.randrange(4) == 0 else bytes([i, seed % 256])
                )
            all_ops = {}
            for r in replicas:
                all_ops.update(r.operations)
            expected = Replica("tenant")
            for op in all_ops.values():
                expected.apply(op)
            expected_snapshot = expected.snapshot()
            for r in replicas:
                r.merge(expected)
                self.assertEqual(r.snapshot(), expected_snapshot)
                cases += 1
            h.update(json.dumps(expected_snapshot, sort_keys=True).encode())
        print(f"reference_seed_cases={cases} snapshot_sha256={h.hexdigest()}")
        self.assertEqual(cases, 3000)

    def test_replay_collision_and_tenant(self):
        r = Replica("tenant")
        first = r.local("actor", "k", b"first")
        self.assertFalse(r.apply(first))
        with self.assertRaises(ValueError):
            r.apply(Op("tenant", "actor", 1, "k", b"tampered", ()))
        with self.assertRaises(ValueError):
            r.apply(Op("other", "actor", 2, "k", b"data", (("actor", 1),)))
        self.assertEqual(r.snapshot(), {"k": b"first".hex()})

    def test_atomic_conflicting_merge(self):
        a, b = Replica("tenant"), Replica("tenant")
        a.local("actor", "k", b"a")
        b.local("actor", "k", b"b")
        b.local("other", "other", b"new")
        before = a.snapshot()
        with self.assertRaises(ValueError):
            a.merge(b)
        self.assertEqual(a.snapshot(), before)


if __name__ == "__main__":
    unittest.main(verbosity=2)
