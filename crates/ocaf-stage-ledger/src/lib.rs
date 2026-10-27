//! Dependency ordered stage ledger for the 33 swarms.
//!
//! A swarm can claim only the first eligible stage.  Completion is accepted
//! only with a source commit, test command/result, artifact digest and
//! receipt id.  This is the execution boundary that keeps generated plans and
//! prose claims out of the roadmap's verified state.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
    pub claimed_by: String,
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
    #[error("receipt claimant does not own the stage")]
    ClaimantMismatch,
    #[error("receipt id was already consumed")]
    DuplicateReceipt,
    #[error("stage dependency graph is invalid")]
    InvalidGraph,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageLedger {
    stages: BTreeMap<String, StageRecord>,
    #[serde(default)]
    receipt_ids: BTreeSet<String>,
}

impl StageLedger {
    pub fn new(records: impl IntoIterator<Item = StageRecord>) -> Result<Self, StageLedgerError> {
        let mut stages = BTreeMap::new();
        for record in records {
            validate_stage_id(&record.stage_id)?;
            match (record.status, record.claimed_by.as_deref()) {
                (StageStatus::Claimed, Some(actor)) => validate_actor(actor)?,
                (StageStatus::Claimed, None) => return Err(StageLedgerError::InvalidGraph),
                (_, Some(_)) => return Err(StageLedgerError::InvalidGraph),
                (_, None) => {}
            }
            let mut dependencies = BTreeSet::new();
            for dependency in &record.dependencies {
                validate_stage_id(dependency)?;
                if dependency == &record.stage_id || !dependencies.insert(dependency) {
                    return Err(StageLedgerError::InvalidGraph);
                }
            }
            if stages.insert(record.stage_id.clone(), record).is_some() {
                return Err(StageLedgerError::InvalidGraph);
            }
        }
        if stages.values().any(|stage| {
            stage
                .dependencies
                .iter()
                .any(|dependency| !stages.contains_key(dependency))
        }) || contains_dependency_cycle(&stages)
        {
            return Err(StageLedgerError::InvalidGraph);
        }
        Ok(Self {
            stages,
            receipt_ids: BTreeSet::new(),
        })
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
        let stage = self
            .stages
            .get_mut(&stage_id)
            .expect("selected stage exists");
        stage.status = StageStatus::Claimed;
        stage.claimed_by = Some(swarm_id.to_owned());
        Ok(stage_id)
    }

    pub fn complete(&mut self, receipt: StageReceipt) -> Result<(), StageLedgerError> {
        validate_receipt(&receipt)?;
        if self.receipt_ids.contains(&receipt.receipt_id) {
            return Err(StageLedgerError::DuplicateReceipt);
        }
        let stage = self
            .stages
            .get_mut(&receipt.stage_id)
            .ok_or_else(|| StageLedgerError::UnknownStage(receipt.stage_id.clone()))?;
        if stage.status != StageStatus::Claimed {
            return Err(StageLedgerError::NotClaimable);
        }
        if stage.claimed_by.as_deref() != Some(receipt.claimed_by.as_str()) {
            return Err(StageLedgerError::ClaimantMismatch);
        }
        stage.status = StageStatus::CompleteVerified;
        stage.claimed_by = None;
        self.receipt_ids.insert(receipt.receipt_id);
        Ok(())
    }

    pub fn statuses(&self) -> impl Iterator<Item = &StageRecord> {
        self.stages.values()
    }
}

fn validate_stage_id(value: &str) -> Result<(), StageLedgerError> {
    if value.is_empty()
        || value.len() > 64
        || value
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(StageLedgerError::UnknownStage(value.to_owned()));
    }
    Ok(())
}

fn validate_actor(value: &str) -> Result<(), StageLedgerError> {
    if value.is_empty() || value.len() > 128 || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(StageLedgerError::InvalidReceipt("swarm_id"));
    }
    Ok(())
}

fn validate_receipt(receipt: &StageReceipt) -> Result<(), StageLedgerError> {
    validate_stage_id(&receipt.stage_id)?;
    validate_actor(&receipt.claimed_by)?;
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
    let digest = receipt
        .artifact_digest
        .strip_prefix("sha256:")
        .unwrap_or("");
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(StageLedgerError::InvalidReceipt("artifact_digest"));
    }
    if receipt.test_result.trim() != "PASS" {
        return Err(StageLedgerError::InvalidReceipt("test_result"));
    }
    if !matches!(receipt.source_commit.len(), 40 | 64)
        || !receipt
            .source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(StageLedgerError::InvalidReceipt("source_commit"));
    }
    if receipt.known_limits.len() > 128
        || receipt.known_limits.iter().any(|limit| {
            limit.trim().is_empty() || limit.len() > 1024 || limit.bytes().any(|byte| byte == 0)
        })
    {
        return Err(StageLedgerError::InvalidReceipt("known_limits"));
    }
    Ok(())
}

fn contains_dependency_cycle(stages: &BTreeMap<String, StageRecord>) -> bool {
    fn visit(
        id: &str,
        stages: &BTreeMap<String, StageRecord>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visited.contains(id) {
            return false;
        }
        if !visiting.insert(id.to_owned()) {
            return true;
        }
        if stages[id]
            .dependencies
            .iter()
            .any(|dependency| visit(dependency, stages, visiting, visited))
        {
            return true;
        }
        visiting.remove(id);
        visited.insert(id.to_owned());
        false
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    stages
        .keys()
        .any(|id| visit(id, stages, &mut visiting, &mut visited))
}

fn compare_stage_ids(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |value: &str| {
        value
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(u64::MAX))
            .collect::<Vec<_>>()
    };
    parse(left).cmp(&parse(right))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, dependencies: &[&str]) -> StageRecord {
        StageRecord {
            stage_id: id.into(),
            dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
            status: StageStatus::Ready,
            claimed_by: None,
        }
    }

    fn receipt(stage_id: &str) -> StageReceipt {
        StageReceipt {
            stage_id: stage_id.into(),
            claimed_by: "OCAF-SWARM-01".into(),
            source_commit: "a".repeat(40),
            test_command: "cargo test".into(),
            test_result: "PASS".into(),
            artifact_digest: format!("sha256:{}", "b".repeat(64)),
            receipt_id: format!("receipt-{stage_id}"),
            known_limits: vec![],
        }
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
        assert!(ledger.next_eligible().is_none());
        ledger.complete(receipt("1")).unwrap();
        assert_eq!(ledger.claim_next("OCAF-SWARM-01").unwrap(), "2");
    }

    #[test]
    fn fake_or_failed_receipt_is_rejected() {
        let mut ledger = StageLedger::new([record("1", &[])]).unwrap();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        let mut bad = receipt("1");
        bad.test_result = "FAIL".into();
        assert_eq!(
            ledger.complete(bad),
            Err(StageLedgerError::InvalidReceipt("test_result"))
        );
    }

    #[test]
    fn pass_substrings_and_malformed_evidence_are_rejected() {
        let mut ledger = StageLedger::new([record("1", &[])]).unwrap();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        for value in ["BYPASS", "NOT PASS", "PASS with failures"] {
            let mut bad = receipt("1");
            bad.test_result = value.into();
            assert_eq!(
                ledger.complete(bad),
                Err(StageLedgerError::InvalidReceipt("test_result"))
            );
        }
        let mut bad = receipt("1");
        bad.artifact_digest = "sha256:not-a-digest".into();
        assert_eq!(
            ledger.complete(bad),
            Err(StageLedgerError::InvalidReceipt("artifact_digest"))
        );
    }

    #[test]
    fn only_claiming_swarm_can_complete_and_receipt_ids_are_unique() {
        let mut ledger = StageLedger::new([record("1", &[]), record("2", &[])]).unwrap();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        let mut wrong = receipt("1");
        wrong.claimed_by = "OCAF-SWARM-02".into();
        assert_eq!(
            ledger.complete(wrong),
            Err(StageLedgerError::ClaimantMismatch)
        );
        let first = receipt("1");
        ledger.complete(first.clone()).unwrap();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        let mut duplicate = receipt("2");
        duplicate.receipt_id = first.receipt_id;
        assert_eq!(
            ledger.complete(duplicate),
            Err(StageLedgerError::DuplicateReceipt)
        );
    }

    #[test]
    fn invalid_dependency_graphs_are_rejected() {
        assert!(matches!(
            StageLedger::new([record("1", &["9"])]),
            Err(StageLedgerError::InvalidGraph)
        ));
        assert!(matches!(
            StageLedger::new([record("1", &["1"])]),
            Err(StageLedgerError::InvalidGraph)
        ));
        assert!(matches!(
            StageLedger::new([record("1", &["2"]), record("2", &["1"])]),
            Err(StageLedgerError::InvalidGraph)
        ));
    }
}
