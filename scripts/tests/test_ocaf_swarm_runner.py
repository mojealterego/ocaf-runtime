#!/usr/bin/env python3
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

import sys
sys.path.insert(0, str(Path(__file__).parents[1]))
import ocaf_swarm_runner as runner


class SwarmRunnerTests(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(":memory:", isolation_level=None)
        self.db.row_factory = sqlite3.Row
        runner.init_db(self.db)
        self.db.execute(
            """INSERT INTO stages
            (id,ordinal,title,acceptance,negative,dependencies,status,updated_at_ms)
            VALUES('0.01',1,'first','commit and CI','reject fake proof','[]','READY',0)"""
        )
        self.db.execute(
            """INSERT INTO stages
            (id,ordinal,title,acceptance,negative,dependencies,status,updated_at_ms)
            VALUES('0.02',2,'second','depends on first','reject fake proof','["0.01"]','BLOCKED',0)"""
        )

    def tearDown(self):
        self.db.close()

    def test_claim_is_exclusive_and_dependencies_gate(self):
        first = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        self.assertEqual(first["stage_id"], "0.01")
        self.assertIsNone(runner.claim_next(self.db, "G2", "runtime", "a" * 40))
        proof = {
            "status": "COMPLETE_VERIFIED",
            "evidence": {
                "commit_sha": "b" * 40,
                "ci_runs": ["ci-1"],
                "negative_tests": ["reject-invalid"],
                "independent_review": "review-1",
            },
        }
        result = runner.record_worker_result(self.db, first, "{}", "", proof, 0)
        self.assertEqual(result["status"], "COMPLETE_VERIFIED")
        second = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        self.assertEqual(second["stage_id"], "0.02")

    def test_plan_only_output_cannot_complete(self):
        claim = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        result = runner.record_worker_result(
            self.db, claim, '{"plan":"later"}', "", {"status": "OPEN", "plan": "later"}, 0
        )
        self.assertEqual(result["status"], "OPEN")
        row = self.db.execute("SELECT status FROM stages WHERE id='0.01'").fetchone()
        self.assertEqual(row["status"], "OPEN")
        resumed = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        self.assertEqual(resumed["stage_id"], "0.01")

    def test_open_stage_blocks_later_ready_work_for_other_groups(self):
        self.db.execute(
            """INSERT INTO stages
            (id,ordinal,title,acceptance,negative,dependencies,status,updated_at_ms)
            VALUES('0.03',3,'third','commit and CI','reject fake proof','[]','READY',0)"""
        )
        first = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        runner.record_worker_result(self.db, first, '{"plan":"later"}', "", {"status": "OPEN"}, 0)
        self.assertIsNone(runner.claim_next(self.db, "G2", "runtime", "b" * 40))
        resumed = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        self.assertEqual(resumed["stage_id"], "0.01")

    def test_invalid_completion_evidence_is_rejected(self):
        claim = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        proof = {"status": "COMPLETE_VERIFIED", "evidence": {"commit_sha": "bad"}}
        result = runner.record_worker_result(self.db, claim, "", "", proof, 0)
        self.assertEqual(result["status"], "OPEN")

    def test_expired_lease_returns_to_ready_for_another_group(self):
        claim = runner.claim_next(self.db, "G1", "runtime", "a" * 40, lease_ms=10_000)
        self.db.execute("UPDATE stages SET lease_until_ms=0 WHERE id='0.01'")
        other = runner.claim_next(self.db, "G2", "runtime", "c" * 40)
        self.assertEqual(other["stage_id"], "0.01")
        self.assertEqual(other["group"], "G2")
        self.assertNotEqual(other["lease_token"], claim["lease_token"])

    def test_roster_has_33_unique_specialized_groups(self):
        groups = runner.group_status(self.db)
        self.assertEqual(len(groups), 33)
        self.assertEqual({group["group_id"] for group in groups}, {f"G{i}" for i in range(1, 34)})
        self.assertEqual(len({group["specialization"] for group in groups}), 33)
        self.assertTrue(all(group["status"] == "IDLE" for group in groups))

    def test_group_cannot_hold_two_stages_at_once(self):
        self.db.execute(
            """INSERT INTO stages
            (id,ordinal,title,acceptance,negative,dependencies,status,updated_at_ms)
            VALUES('0.03',3,'third','commit and CI','reject fake proof','[]','READY',0)"""
        )
        first = runner.claim_next(self.db, "G1", "ignored-by-roster", "a" * 40)
        self.assertIsNotNone(first)
        self.assertIsNone(runner.claim_next(self.db, "G1", "ignored-by-roster", "a" * 40))
        working = {group["group_id"]: group for group in runner.group_status(self.db)}
        self.assertEqual(working["G1"]["status"], "WORKING")
        self.assertEqual(working["G1"]["stage_id"], "0.01")

    def test_explicit_stage_claim_cannot_skip_earlier_ready_work(self):
        self.db.execute(
            """INSERT INTO stages
            (id,ordinal,title,acceptance,negative,dependencies,status,updated_at_ms)
            VALUES('0.03',3,'third','commit and CI','reject fake proof','[]','READY',0)"""
        )
        with self.assertRaises(runner.SwarmError):
            runner.claim_next(self.db, "G1", "runtime", "a" * 40, stage_id="0.03")

    def test_historical_undefined_rows_do_not_block_sequential_queue(self):
        self.db.execute(
            "UPDATE stages SET title='Historyczny etap bez zachowanej definicji' WHERE id='0.01'"
        )
        claim = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        self.assertIsNone(claim)

    def test_unknown_group_is_rejected(self):
        with self.assertRaises(runner.SwarmError):
            runner.claim_next(self.db, "G34", "runtime", "a" * 40)

    def test_external_completion_uses_active_lease(self):
        claim = runner.claim_next(self.db, "G1", "runtime", "a" * 40)
        loaded = runner.load_claim(self.db, "G1", claim["stage_id"], claim["lease_token"])
        self.assertEqual(loaded["lease_token"], claim["lease_token"])
        proof = {
            "status": "COMPLETE_VERIFIED",
            "evidence": {
                "commit_sha": "b" * 40,
                "ci_runs": ["ci-1"],
                "negative_tests": ["reject-invalid"],
                "independent_review": "review-1",
            },
        }
        result = runner.record_worker_result(self.db, loaded, "{}", "", proof, 0)
        self.assertEqual(result["status"], "COMPLETE_VERIFIED")
        with self.assertRaises(runner.SwarmError):
            runner.load_claim(self.db, "G1", claim["stage_id"], claim["lease_token"])


if __name__ == "__main__":
    unittest.main()
