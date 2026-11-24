//! Fail-closed reconciliation of signed research evidence, the OM Engine event
//! journal, and a persistent scheduler period (AB-OM-01 / AB-OM-10).
//!
//! This component only confirms a completed signed research workflow; it does
//! not evaluate the truth of external citations, invoke tools or deploy agents.
//! The host MUST authenticate tenant/fleet, provision independent pinned public
//! keys for ResearchEvidenceGate, and protect SQLite from rollback.

use crate::{
    architect_research_evidence::{EvidenceError, ResearchEvidenceGate},
    architect_research_schedule::{ArchitectResearchScheduler, Recorded, ResearchError},
    architect_workflow_bridge::{workflow_id, RegisteredResearch},
    om_engine::{OmEngine, OmError},
    omnidev_workflow::{Outcome, Role, StepStatus},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedResearch {
    pub workflow_id: String,
    pub evidence_sha256: String,
    pub replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizationScan {
    pub scanned: usize,
    pub finalized: usize,
    pub pending: usize,
}

/// Scan up to 64 due periods. Unregistered or incomplete workflows are
/// reported as pending. Corrupt signatures, journal integrity failures and
/// non-idempotent state conflicts abort the scan rather than being skipped.
/// The next tick may safely retry the same periods.
pub fn finalize_ready(
    scheduler: &ArchitectResearchScheduler,
    engine: &OmEngine,
    evidence_gate: &ResearchEvidenceGate,
    tenant: &str,
    fleet: &str,
    now_unix: i64,
    limit: usize,
) -> Result<FinalizationScan, FinalizeError> {
    let due = scheduler.list_due(tenant, fleet, now_unix, limit)?;
    let scanned = due.len();
    let mut finalized = 0;
    let mut pending = 0;
    for period in due {
        let record = RegisteredResearch {
            workflow_id: workflow_id(fleet, period.kind, period.start_unix),
            kind: period.kind,
            period_start_unix: period.start_unix,
        };
        match finalize_due(
            scheduler,
            engine,
            evidence_gate,
            tenant,
            fleet,
            &record,
            now_unix,
        ) {
            Ok(result) => {
                if !result.replay {
                    finalized += 1;
                }
            }
            Err(FinalizeError::Unverified)
            | Err(FinalizeError::Evidence(EvidenceError::NotFound))
            | Err(FinalizeError::Engine(OmError::NotFound))
            | Err(FinalizeError::Scheduler(ResearchError::Busy)) => pending += 1,
            Err(err) => return Err(err),
        }
    }
    Ok(FinalizationScan {
        scanned,
        finalized,
        pending,
    })
}

#[derive(Debug, Error)]
pub enum FinalizeError {
    #[error("workflow identity does not match tenant/fleet/period")]
    Mismatch,
    #[error("OM Engine research lacks a completed signed three-role evidence chain")]
    Unverified,
    #[error(transparent)]
    Evidence(#[from] EvidenceError),
    #[error(transparent)]
    Engine(#[from] OmError),
    #[error(transparent)]
    Scheduler(#[from] ResearchError),
}

/// Reconcile exactly one immutable scheduler period. No external effects.
/// A signed Verifier receipt is essential but insufficient on its own: all
/// preceding signed steps and OM Engine receipts must match as well.
#[allow(clippy::too_many_arguments)]
pub fn finalize_due(
    scheduler: &ArchitectResearchScheduler,
    engine: &OmEngine,
    evidence_gate: &ResearchEvidenceGate,
    tenant: &str,
    fleet: &str,
    record: &RegisteredResearch,
    now_unix: i64,
) -> Result<FinalizedResearch, FinalizeError> {
    if record.workflow_id != workflow_id(fleet, record.kind, record.period_start_unix) {
        return Err(FinalizeError::Mismatch);
    }
    let id = &record.workflow_id;
    let mut research_digest = None;
    let mut verified_digest = None;

    for (step, role) in [
        ("plan", Role::Planner),
        ("research", Role::Architect),
        ("verify", Role::Verifier),
    ] {
        let status = engine.status(tenant, id, step)?;
        let receipt = engine.last_receipt(tenant, id, step)?;
        let Some(receipt) = receipt else {
            return Err(FinalizeError::Unverified);
        };
        if receipt.outcome != Outcome::Passed
            || !matches!(
                status,
                StepStatus::Passed { evidence_sha256 } if evidence_sha256 == receipt.evidence_sha256
            )
        {
            return Err(FinalizeError::Unverified);
        }
        let signed = evidence_gate.read(tenant, id, step, receipt.epoch, &receipt.attempt_id)?;
        if signed.digest_sha256 != receipt.evidence_sha256
            || signed.evidence.role != role
            || signed.evidence.outcome != Outcome::Passed
            || signed.evidence.actual_cost_minor != receipt.actual_cost_minor
            || signed.evidence.review_of_sha256.as_deref()
                != if step == "verify" {
                    research_digest.as_deref()
                } else {
                    None
                }
        {
            return Err(FinalizeError::Unverified);
        }
        match step {
            "research" => research_digest = Some(signed.digest_sha256),
            "verify" => verified_digest = Some(signed.digest_sha256),
            _ => {}
        }
    }

    let evidence_sha256 = verified_digest.ok_or(FinalizeError::Unverified)?;
    // This is an atomic SQLite state transition. It deliberately cannot
    // hijack a live or expired scheduler lease; those require independent
    // timeout reconciliation, not a hidden retry.
    let outcome = scheduler.seal_verified(
        tenant,
        fleet,
        record.kind,
        record.period_start_unix,
        &evidence_sha256,
        now_unix,
    )?;
    Ok(FinalizedResearch {
        workflow_id: id.clone(),
        evidence_sha256,
        replay: outcome == Recorded::Replay,
    })
}
