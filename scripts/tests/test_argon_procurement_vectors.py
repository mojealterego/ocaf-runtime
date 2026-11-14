#!/usr/bin/env python3
"""Independent SHA-256 reference model, NOT a Rust compile/test or sanctions attestation.

Run: python3 -m unittest discover -s scripts/tests -p 'test_argon_procurement_vectors.py' -v
"""
from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import re
import unittest

REPO_ROOT = Path(__file__).resolve().parents[2]
VECTORS = REPO_ROOT / "docs/evidence/ARGON_PROCUREMENT_INTENT_SHA256_REFERENCE_2026-11-14.json"
IDENTIFIER = re.compile(r"[A-Za-z0-9_:.\-]+\Z", re.ASCII)
NAME_WORD = re.compile(r"[A-Za-z0-9&.'\-]+\Z", re.ASCII)
ASCII_WS = " \t\r\n\v\f"


def field(hasher, raw: bytes | str) -> None:
    payload = raw.encode("utf-8") if isinstance(raw, str) else raw
    hasher.update(len(payload).to_bytes(8, "big"))
    hasher.update(payload)


def u64(value: int) -> bytes:
    if not 0 <= value < (1 << 64):
        raise ValueError("not u64")
    return value.to_bytes(8, "big")


def canonical_name(name: str) -> str:
    if not 0 < len(name.encode("utf-8")) <= 160:
        raise ValueError("unsupported name size")
    words = re.split(r"[ \t\r\n\v\f]+", name.strip(ASCII_WS))
    if not words or any(not NAME_WORD.fullmatch(word) for word in words):
        raise ValueError("unsupported name")
    return " ".join(words).upper()


def valid_identifier(value: str) -> bool:
    return bool(
        value and len(value.encode("utf-8")) <= 128
        and IDENTIFIER.fullmatch(value)
        and not value.startswith(".") and not value.endswith(".")
        and ".." not in value
    )


def watchlist_digest(fixture: dict) -> bytes:
    rows = fixture["feed_records"]
    if (
        not valid_identifier(fixture["tenant"])
        or not valid_identifier(fixture["feed_issuer"])
        or not 0 < len(rows) <= 65536
        or not 0 < fixture["feed_issued_unix"] < fixture["feed_expires_unix"]
    ):
        raise ValueError("invalid watchlist")
    normalized = []
    seen = set()
    for row in rows:
        ident = row["id"]
        if not valid_identifier(ident) or ident in seen:
            raise ValueError("invalid/duplicate listing")
        seen.add(ident)
        normalized.append((ident, canonical_name(row["name"])))
    hasher = hashlib.sha256(b"OCAF-PROCUREMENT-FEED-v1")
    for item in (
        fixture["tenant"],
        fixture["feed_issuer"],
        u64(fixture["feed_issued_unix"]),
        u64(fixture["feed_expires_unix"]),
        u64(len(normalized)),
    ):
        field(hasher, item)
    for ident, name in sorted(normalized):
        field(hasher, ident)
        field(hasher, name)
    return hasher.digest()


def policy_digest(fixture: dict) -> bytes:
    hasher = hashlib.sha256(b"OCAF-PROCUREMENT-POLICY-v1")
    for item in (
        u64(fixture["medium_risk_cap_cents"]),
        bytes([fixture["minimum_financial_score"]]),
        u64(fixture["max_feed_age_secs"]),
        bytes([int(fixture["iso_claim_required"])]),
    ):
        field(hasher, item)
    return hasher.digest()


def expiry_field(hasher, expiry: int | None) -> None:
    field(hasher, b"none" if expiry is None else b"some")
    if expiry is not None:
        field(hasher, u64(expiry))


def dossier_digest(fixture: dict) -> bytes:
    owners = sorted({canonical_name(name) for name in fixture["beneficial_owners"]})
    hasher = hashlib.sha256(b"OCAF-PROCUREMENT-DOSSIER-v1")
    for item in (
        fixture["tenant"],
        fixture["supplier_id"],
        canonical_name(fixture["legal_name"]),
        fixture["currency"],
        u64(fixture["amount_cents"]),
    ):
        field(hasher, item)
    field(hasher, u64(len(owners)))
    for name in owners:
        field(hasher, name)
    expiry_field(hasher, fixture["tax_expires_unix"])
    expiry_field(hasher, fixture["insurance_expires_unix"])
    field(hasher, bytes([int(fixture["iso27001_claimed"])]))
    field(hasher, bytes([fixture["financial_score"]]))
    return hasher.digest()


def intent_digest(fixture: dict) -> bytes:
    if fixture["currency"] != "USD":
        raise ValueError("Rust contract supports USD only")
    hasher = hashlib.sha256(b"OCAF-PROCUREMENT-PURCHASE-INTENT-v1")
    for item in (
        "procurement.commit_purchase_order",
        fixture["tenant"],
        fixture["supplier_id"],
        "USD",
        u64(fixture["amount_cents"]),
        dossier_digest(fixture),
        watchlist_digest(fixture),
        policy_digest(fixture),
    ):
        field(hasher, item)
    return hasher.digest()


class ProcurementReferenceModelTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reference = json.loads(VECTORS.read_text(encoding="utf-8"))
        cls.fixture = cls.reference["fixture"]

    def test_primary_vectors_are_correct(self):
        expected = self.reference["expected_hashes"]
        for label, actual in (
            ("watchlist_sha256", watchlist_digest(self.fixture)),
            ("policy_sha256", policy_digest(self.fixture)),
            ("dossier_sha256", dossier_digest(self.fixture)),
            ("purchase_payload_sha256", intent_digest(self.fixture)),
        ):
            with self.subTest(vector=label):
                self.assertEqual(actual.hex(), expected[label])

    def test_mutation_vectors_are_correct(self):
        variants = {
            "amount_9001": {"amount_cents": 9001},
            "policy_cap_10001": {"medium_risk_cap_cents": 10001},
            "owner_DIFFERENT_OWNER": {"beneficial_owners": ["DIFFERENT OWNER"]},
            "watchlist_OTHER_UNRELATED_PARTNER": {
                "feed_records": [{"id": "w1", "name": "OTHER UNRELATED PARTNER"}]
            },
        }
        for label, change in variants.items():
            with self.subTest(vector=label):
                actual = intent_digest({**self.fixture, **change}).hex()
                self.assertEqual(
                    actual, self.reference["mutated_expected_purchase_hashes"][label]
                )
                self.assertNotEqual(
                    actual, self.reference["expected_hashes"]["purchase_payload_sha256"]
                )

    def test_watchlist_record_order_is_canonical(self):
        fixture = copy.deepcopy(self.fixture)
        fixture["feed_records"] = [
            {"id": "z1", "name": "OTHER COMPANY"},
            {"id": "a2", "name": "another  supplier"},
            {"id": "m3", "name": " THIRD INC"},
        ]
        baseline = watchlist_digest(fixture)
        fixture["feed_records"].reverse()
        self.assertEqual(watchlist_digest(fixture), baseline)

    def test_beneficial_owner_canonical_set(self):
        fixture = copy.deepcopy(self.fixture)
        fixture["beneficial_owners"] = ["KNOWN OWNER", "SECOND OWNER"]
        baseline = dossier_digest(fixture)
        fixture["beneficial_owners"] = [
            "second  owner", "KNOWN OWNER", "KNOWN OWNER"
        ]
        self.assertEqual(dossier_digest(fixture), baseline)

    def test_unsupported_names_and_duplicate_listings_fail_closed(self):
        with self.assertRaises(ValueError):
            canonical_name("ŻURAW SA")
        fixture = copy.deepcopy(self.fixture)
        fixture["feed_records"].append({"id": "w1", "name": "OTHER SUPPLIER"})
        with self.assertRaises(ValueError):
            watchlist_digest(fixture)

    def test_644_deterministic_mutations_change_intent_digest(self):
        """644 deterministic Python reference comparisons; NOT Rust tests."""
        expected = intent_digest(self.fixture)
        checked = 0
        for n in range(1, 162):
            for change in (
                {"amount_cents": self.fixture["amount_cents"] + n},
                {"medium_risk_cap_cents": self.fixture["medium_risk_cap_cents"] + n},
                {"beneficial_owners": [f"OWNER {n}"]},
                {"feed_records": [{"id": "w1", "name": f"OTHER LTD {n}"}]},
            ):
                with self.subTest(iteration=n, mutation=next(iter(change))):
                    self.assertNotEqual(
                        intent_digest({**self.fixture, **change}), expected
                    )
                    checked += 1
        self.assertEqual(checked, 644)


if __name__ == "__main__":
    unittest.main(verbosity=2)
