import unittest
from scripts.stage041_fault_evidence import EvidenceError,classify


def receipts():
    return (
        dict(status='ONE_HOST_TWO_JAILED_VMS_SIGNED_GUEST_QUARANTINE_LAB',
             tenant_b_survived_a_quarantine=True,production_approved=False),
        dict(status='ONE_HOST_GUEST_AYA_LIVE_BROKER_SIGKILL_DENIED',tenant_a=dict(
             broker_process_sigkill_observed=True,tenant_b_live_during_denial=True,
             permit_issued=False,host_aya_program_id=5,guest_aya_program_id=6)),
        dict(status='TWO_LINUX_KERNELS_REAL_PHYSICAL_WAL_PITR_RECOVERY_PASSED',
             source_boot_id='a',recovery_boot_id='b',real_wal_replay_exited_recovery=True,
             production_approved=False,stage_041_complete=False))

class FaultEvidence(unittest.TestCase):
    def test_separate_success_never_claims_integrated_fault(self):
        result=classify(*receipts())
        self.assertFalse(result['broker_vm_postgres_integrated_fault_tested'])
    def test_reject_missing_quarantine_broker_or_pg(self):
        variants=[(0,'tenant_b_survived_a_quarantine',False),
                  (1,'status','SIMULATED'),(2,'recovery_boot_id','a'),
                  (2,'stage_041_complete',True)]
        for index,key,value in variants:
            data=list(receipts());data[index][key]=value
            with self.assertRaises(EvidenceError): classify(*data)
