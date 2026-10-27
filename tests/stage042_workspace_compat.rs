//! Keep the original public path working while the ledger becomes a workspace crate.
use ocaf_runtime::stage_ledger::{StageLedger, StageRecord, StageStatus};

#[test]
fn old_public_ledger_path_claims_a_real_stage() {
    let mut ledger = StageLedger::new([StageRecord {
        stage_id: "0.42.1".into(),
        dependencies: vec![],
        status: StageStatus::Ready,
        claimed_by: None,
    }])
    .expect("valid stage graph");
    assert_eq!(ledger.claim_next("workspace-migration").unwrap(), "0.42.1");
    assert_eq!(ledger.get("0.42.1").unwrap().status, StageStatus::Claimed);
}
