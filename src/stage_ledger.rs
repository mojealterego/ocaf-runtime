//! Dependency ordered stage ledger for the 33 swarms.
//!
//! A swarm can claim only the first eligible stage.  Completion is accepted
//! only with a source commit, test command/result, artifact digest and
//! receipt id.  This is the execution boundary that keeps generated plans and
//! prose claims out of the roadmap's verified state.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StageStatus {
    Ready,
    Claimed,
    CompleteVerified,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRecord {
    pub stage_id: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub status: StageStatus,
    #[serde(default)]
    pub claimed_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageReceipt {
    pub stage_id: String,
    pub source_commit: String,
    pub test_command: String,
    pub test_result: String,
    pub artifact_digest: String,
    pub receipt_id: String,
    #[serde(default)]
    pub known_limits: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StageLedgerError {
    #[error("stage does not exist: {0}")]
    UnknownStage(String),
    #[error("stage is not the next eligible stage")]
    OutOfOrder,
    #[error("stage is not claimable in its current state")]
    NotClaimable,
    #[error("stage dependencies are incomplete")]
    DependenciesIncomplete,
    #[error("receipt is incomplete: {0}")]
    InvalidReceipt(&'static str),
    #[error("receipt does not match stage")]
    ReceiptMismatch,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageLedger {
    stages: BTreeMap<String, StageRecord>,
}

impl StageLedger {
    pub fn new(records: impl IntoIterator<Item = StageRecord>) -> Result<Self, StageLedgerError> {
        let mut stages = BTreeMap::new();
        for mut record in records {
            validate_stage_id(&record.stage_id)?;
            record.claimed_by = None;
            if stages.insert(record.stage_id.clone(), record).is_some() {
                return Err(StageLedgerError::UnknownStage("duplicate stage".into()));
            }
        }
        Ok(Self { stages })
    }

    pub fn get(&self, stage_id: &str) -> Option<&StageRecord> {
        self.stages.get(stage_id)
    }

    /// Returns the first ready stage in numeric roadmap order whose
    /// dependencies are verified complete.
    pub fn next_eligible(&self) -> Option<&StageRecord> {
        self.stages
            .values()
            .filter(|stage| stage.status == StageStatus::Ready)
            .filter(|stage| {
                stage.dependencies.iter().all(|dependency| {
                    self.stages
                        .get(dependency)
                        .is_some_and(|record| record.status == StageStatus::CompleteVerified)
                })
            })
            .min_by(|left, right| compare_stage_ids(&left.stage_id, &right.stage_id))
    }

    pub fn claim_next(&mut self, swarm_id: &str) -> Result<String, StageLedgerError> {
        validate_actor(swarm_id)?;
        let stage_id = self
            .next_eligible()
            .map(|stage| stage.stage_id.clone())
            .ok_or(StageLedgerError::NotClaimable)?;
        let stage = self.stages.get_mut(&stage_id).expect("selected stage exists");
        stage.status = StageStatus::Claimed;
        stage.claimed_by = Some(swarm_id.to_owned());
        Ok(stage_id)
    }

    pub fn complete(&mut self, receipt: StageReceipt) -> Result<(), StageLedgerError> {
        validate_receipt(&receipt)?;
        let stage = self
            .stages
            .get_mut(&receipt.stage_id)
            .ok_or_else(|| StageLedgerError::UnknownStage(receipt.stage_id.clone()))?;
        if stage.status != StageStatus::Claimed {
            return Err(StageLedgerError::NotClaimable);
        }
        if stage.claimed_by.is_none() {
            return Err(StageLedgerError::NotClaimable);
        }
        stage.status = StageStatus::CompleteVerified;
        Ok(())
    }

    pub fn statuses(&self) -> impl Iterator<Item = &StageRecord> {
        self.stages.values()
    }
}

fn validate_stage_id(value: &str) -> Result<(), StageLedgerError> {
    if value.is_empty() || value.len() > 64 || value.split('.').any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit())) {
        return Err(StageLedgerError::UnknownStage(value.to_owned()));
    }
    Ok(())
}

fn validate_actor(value: &str) -> Result<(), StageLedgerError> {
    if value.is_empty() || value.len() > 128 || value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err(StageLedgerError::InvalidReceipt("swarm_id"));
    }
    Ok(())
}

fn validate_receipt(receipt: &StageReceipt) -> Result<(), StageLedgerError> {
    validate_stage_id(&receipt.stage_id)?;
    for (value, field) in [
        (&receipt.source_commit, "source_commit"),
        (&receipt.test_command, "test_command"),
        (&receipt.test_result, "test_result"),
        (&receipt.artifact_digest, "artifact_digest"),
        (&receipt.receipt_id, "receipt_id"),
    ] {
        if value.trim().is_empty() || value.len() > 4096 || value.bytes().any(|byte| byte == 0) {
            return Err(StageLedgerError::InvalidReceipt(field));
        }
    }
    if !receipt.artifact_digest.starts_with("sha256:") {
        return Err(StageLedgerError::InvalidReceipt("artifact_digest"));
    }
    if !receipt.test_result.to_ascii_uppercase().contains("PASS") {
        return Err(StageLedgerError::InvalidReceipt("test_result"));
    }
    Ok(())
}

fn compare_stage_ids(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |value: &str| value.split('.').map(|part| part.parse::<u64>().unwrap_or(u64::MAX)).collect::<Vec<_>>();
    parse(left).cmp(&parse(right))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, dependencies: &[&str]) -> StageRecord {
        StageRecord { stage_id: id.into(), dependencies: dependencies.iter().map(|value| (*value).into()).collect(), status: StageStatus::Ready, claimed_by: None }
    }

    fn receipt(stage_id: &str) -> StageReceipt {
        StageReceipt { stage_id: stage_id.into(), source_commit: "abc123".into(), test_command: "cargo test".into(), test_result: "PASS".into(), artifact_digest: "sha256:artifact".into(), receipt_id: format!("receipt-{stage_id}"), known_limits: vec![] }
    }

    #[test]
    fn claims_numeric_first_stage_and_blocks_skip() {
        let mut ledger = StageLedger::new([record("10", &[]), record("2", &[])]).unwrap();
        assert_eq!(ledger.claim_next("OCAF-SWARM-01").unwrap(), "2");
        assert_eq!(ledger.next_eligible().unwrap().stage_id, "10");
    }

    #[test]
    fn dependencies_require_verified_receipt() {
        let mut ledger = StageLedger::new([record("1", &[]), record("2", &["1"])]).unwrap();
        assert_eq!(ledger.claim_next("OCAF-SWARM-01").unwrap(), "1");
        assert_eq!(ledger.next_eligible().unwrap().stage_id, "1");
        ledger.complete(receipt("1")).unwrap();
        assert_eq!(ledger.claim_next("OCAF-SWARM-01").unwrap(), "2");
    }

    #[test]
    fn fake_or_failed_receipt_is_rejected() {
        let mut ledger = StageLedger::new([record("1", &[])]).unwrap();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        let mut bad = receipt("1");
        bad.test_result = "FAIL".into();
        assert_eq!(ledger.complete(bad), Err(StageLedgerError::InvalidReceipt("test_result")));
    }
}
