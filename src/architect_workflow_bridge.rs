//! Bridges persisted Meta Agent Architect research periods into the real OM
//! Engine journal, without claiming execution or dispatching an external tool.
//!
//! Idempotent registration happens BEFORE any lease is claimed. An OM Engine
//! registration failure leaves the scheduler's READY row untouched. The
//! trusted caller must pin the base source commit and cost budget, and later
//! provide authenticated model/OMAs/OMAt execution behind the OM Engine role
//! authorization. Two independent SQLite journals are not a 2PC system.

use crate::{
    architect_research_schedule::{
        ArchitectResearchScheduler, ResearchError, ResearchKind, ResearchPeriod,
    },
    om_engine::{OmEngine, OmError},
    omnidev_workflow::{Role, StepContract, WorkflowContract},
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const EXTENSION_DAYS: i64 = 120;
const DAY: i64 = 86_400;
const MAX_BUDGET_MINOR: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredResearch {
    pub kind: ResearchKind,
    pub period_start_unix: i64,
    pub workflow_id: String,
}

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("invalid or excessive trusted research registration policy")]
    Invalid,
    #[error("research period missed its fixed OM Engine deadline")]
    Expired,
    #[error(transparent)]
    Scheduler(#[from] ResearchError),
    #[error(transparent)]
    Engine(#[from] OmError),
}

fn valid_commit(input: &str) -> bool {
    input.len() == 40
        && input
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn workflow_id(fleet: &str, kind: ResearchKind, period_start: i64) -> String {
    let fleet_digest = format!("{:x}", Sha256::digest(fleet.as_bytes()));
    let kind_key = match kind {
        ResearchKind::Competitive => "competition",
        ResearchKind::Market => "market",
        ResearchKind::SystemEvolution => "evolution",
    };
    format!("arch.{fleet_digest}.{kind_key}.{period_start}")
}

fn contract(
    tenant: &str,
    fleet: &str,
    period: &ResearchPeriod,
    base_commit_sha: &str,
    max_cost_minor: u64,
    now_unix: i64,
) -> Result<WorkflowContract, BridgeError> {
    let deadline = period
        .end_unix
        .checked_add(EXTENSION_DAYS * DAY)
        .ok_or(BridgeError::Invalid)?;
    if deadline <= now_unix {
        return Err(BridgeError::Expired);
    }
    let budget_per_step = max_cost_minor / 3;
    let workflow = WorkflowContract {
        tenant: tenant.to_owned(),
        workflow_id: workflow_id(fleet, period.kind, period.start_unix),
        base_commit_sha: base_commit_sha.to_owned(),
        deadline_unix: u64::try_from(deadline).map_err(|_| BridgeError::Invalid)?,
        max_cost_minor,
        steps: vec![
            StepContract {
                id: "plan".to_owned(),
                role: Role::Planner,
                prerequisites: vec![],
                max_attempts: 1,
                max_cost_minor: budget_per_step,
                lease_seconds: 300,
            },
            StepContract {
                id: "research".to_owned(),
                role: Role::Architect,
                prerequisites: vec!["plan".to_owned()],
                max_attempts: 2,
                max_cost_minor: budget_per_step,
                lease_seconds: 300,
            },
            StepContract {
                id: "verify".to_owned(),
                role: Role::Verifier,
                prerequisites: vec!["research".to_owned()],
                max_attempts: 1,
                max_cost_minor: budget_per_step,
                lease_seconds: 300,
            },
        ],
    };
    Ok(workflow)
}

/// Register bounded, due research periods in the genuine OM Engine journal.
///
/// This method never claims the scheduler lease, never claims an OM Engine step
/// and never invents a successful study. Workers later claim individual steps
/// through OM Engine using authenticated roles and independent evidence.
/// Failures during a batch may leave earlier contracts registered; repeating
/// the same pinned policy is idempotent and safely repairs partial batches.
#[allow(clippy::too_many_arguments)]
pub fn register_due(
    scheduler: &ArchitectResearchScheduler,
    engine: &OmEngine,
    trusted_tenant: &str,
    fleet: &str,
    base_commit_sha: &str,
    max_cost_minor: u64,
    now_unix: i64,
    limit: usize,
) -> Result<Vec<RegisteredResearch>, BridgeError> {
    if !valid_commit(base_commit_sha)
        || !(3..=MAX_BUDGET_MINOR).contains(&max_cost_minor)
        || !(1..=64).contains(&limit)
    {
        return Err(BridgeError::Invalid);
    }
    let periods = scheduler.list_due(trusted_tenant, fleet, now_unix, limit)?;
    let mut registered = Vec::with_capacity(periods.len());
    for period in &periods {
        let contract = contract(
            trusted_tenant,
            fleet,
            period,
            base_commit_sha,
            max_cost_minor,
            now_unix,
        )?;
        let id = contract.workflow_id.clone();
        engine.register(trusted_tenant, contract)?;
        registered.push(RegisteredResearch {
            kind: period.kind,
            period_start_unix: period.start_unix,
            workflow_id: id,
        });
    }
    Ok(registered)
}
