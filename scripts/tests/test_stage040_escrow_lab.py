import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage040_escrow_lab import EscrowLabError, validate_report


class EscrowReceiptTests(unittest.TestCase):
    def setUp(self):
        self.receipt = {
            "status": "LOCAL_PG_ESCROW_SYNC_FAILOVER_REPLAY_PASS",
            "synchronous_remote_apply_observed": True,
            "first_reservation_replayed": True,
            "first_settlement_replayed": True,
            "duplicate_settlement_denied_after_promotion": True,
            "stale_epoch_denied_after_promotion": True,
            "new_epoch_settlement_committed": True,
            "old_primary_stopped_before_promotion": True,
            "both_postmasters_stopped_on_cleanup": True,
            "distinct_kernel_boot_ids": False,
            "external_effect_exactly_once_proved": False,
            "stage_040_13_complete": False,
            "production_approved": False,
        }

    def test_accepts_narrow_local_receipt(self):
        self.assertEqual(validate_report(self.receipt), self.receipt)

    def test_missing_replay_or_duplicate_denial_fails(self):
        for key in ("first_reservation_replayed", "first_settlement_replayed",
                    "duplicate_settlement_denied_after_promotion",
                    "stale_epoch_denied_after_promotion",
                    "synchronous_remote_apply_observed"):
            bad = copy.deepcopy(self.receipt)
            bad[key] = False
            with self.subTest(key=key), self.assertRaises(EscrowLabError):
                validate_report(bad)

    def test_unsupported_distributed_or_production_claim_fails(self):
        for key in ("distinct_kernel_boot_ids", "external_effect_exactly_once_proved",
                    "stage_040_13_complete", "production_approved"):
            bad = copy.deepcopy(self.receipt)
            bad[key] = True
            with self.subTest(key=key), self.assertRaises(EscrowLabError):
                validate_report(bad)


if __name__ == "__main__":
    unittest.main()
