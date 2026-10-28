//! Bounded, deterministic local stage advice backed by ocaf_stage_ledger.

use serde::Serialize;
use thiserror::Error;

use crate::security::{AgentConfig, SecurityError};
use crate::{StageLedger, StageLedgerError, StageRecord, StageStatus, AGENT_ID, GROUP_ID};

pub const MAX_STAGES: usize = 4_096;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error(transparent)]
    Ledger(#[from] StageLedgerError),
    #[error("invalid stage JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("input exceeds configured byte limit")]
    InputTooLarge,
    #[error("stage count exceeds 4096")]
    TooManyStages,
    #[error("claim requires an explicit local operator approval")]
    ApprovalRequired,
    #[error("Asterion may claim only for G01")]
    WrongGroup,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Candidate {
    pub stage_id: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StagePlan {
    pub agent_id: &'static str,
    pub mode: &'static str,
    pub candidates: Vec<Candidate>,
    pub ready_waiting_for_dependencies: usize,
    pub exhausted: bool,
    pub active_admission: &'static str,
}

pub fn parse_ledger(config: &AgentConfig, bytes: &[u8]) -> Result<StageLedger, AgentError> {
    config.validate()?;
    if bytes.len() > config.max_input_bytes {
        return Err(AgentError::InputTooLarge);
    }
    let records: Vec<StageRecord> = serde_json::from_slice(bytes)?;
    if records.len() > MAX_STAGES {
        return Err(AgentError::TooManyStages);
    }
    Ok(StageLedger::new(records)?)
}

pub fn plan(config: &AgentConfig, ledger: &StageLedger) -> Result<StagePlan, AgentError> {
    config.validate()?;
    let mut scratch = ledger.clone();
    let mut candidates = Vec::new();
    for _ in 0..config.max_steps {
        let Some(stage) = scratch.next_eligible() else {
            break;
        };
        candidates.push(Candidate {
            stage_id: stage.stage_id.clone(),
            dependencies: stage.dependencies.clone(),
        });
        // Claim only in a scratch copy to reveal other currently independent
        // candidates. The caller's ledger is unchanged by planning.
        scratch.claim_next(GROUP_ID)?;
    }
    let waiting = ledger
        .statuses()
        .filter(|stage| {
            stage.status == StageStatus::Ready
                && stage.dependencies.iter().any(|dependency| {
                    ledger
                        .get(dependency)
                        .is_none_or(|dep| dep.status != StageStatus::CompleteVerified)
                })
        })
        .count();
    Ok(StagePlan {
        agent_id: AGENT_ID,
        mode: "bounded-local-stage-assistance",
        candidates,
        ready_waiting_for_dependencies: waiting,
        exhausted: scratch.next_eligible().is_none(),
        active_admission: "DENY",
    })
}

/// The caller owns the only mutation. The CLI requires --approve and prints
/// the updated records without writing to disk.
pub fn claim_next(
    config: &AgentConfig,
    ledger: &mut StageLedger,
    group: &str,
    approved_by_operator: bool,
) -> Result<String, AgentError> {
    config.validate()?;
    if !approved_by_operator {
        return Err(AgentError::ApprovalRequired);
    }
    if group != GROUP_ID {
        return Err(AgentError::WrongGroup);
    }
    Ok(ledger.claim_next(group)?)
}

pub fn snapshot(ledger: &StageLedger) -> Vec<StageRecord> {
    ledger.statuses().cloned().collect()
}
