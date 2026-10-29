import sys
from pathlib import Path
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from stage041_pitr_soak import summarize
from test_stage041_pitr import evidence


def cycle(ms=123):
    return {**evidence(), 'crash_to_verified_restore_ms':ms,
            'missing_target_rows':0, 'deliberately_excluded_post_target_rows':5,
            'rpo_seconds':None, 'rpo_scope':'exact_selected_lsn_not_latest_pre_crash_commit'}


class Soak(unittest.TestCase):
    def test_measured_percentiles_and_scope(self):
        result=summarize([cycle(100),cycle(200)],1801,1800)
        self.assertEqual(result['rto_p50_ms'],100)
        self.assertEqual(result['rto_p99_ms'],200)
        self.assertIsNone(result['rpo_seconds'])
        self.assertFalse(result['stage_041_complete'])

    def test_diagnostic_is_not_soak_acceptance(self):
        self.assertEqual(summarize([cycle()],30,30)['status'],'LOCAL_PITR_SMOKE_PASS')

    def test_reject_short_empty_and_invalid_duration(self):
        for cycles,elapsed,requested in (([],1800,1800),([cycle()],1799,1800),
                ([cycle()],float('nan'),1800),([cycle()],7200,120)):
            with self.assertRaises(ValueError): summarize(cycles,elapsed,requested)

    def test_reject_loss_false_rpo_timing_or_production_claim(self):
        for change in ({'missing_target_rows':1},{'rpo_seconds':0},
                       {'crash_to_verified_restore_ms':True},
                       {'crash_to_verified_restore_ms':-1},
                       {'actual_recovered_rows':34},{'production_approved':True}):
            with self.assertRaises(ValueError):
                summarize([{**cycle(),**change}],1800,1800)
