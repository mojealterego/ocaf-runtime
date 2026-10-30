//! Stage 0.56: bounded and fenced OmniDev role/DAG coordinator.
//!
//! This is a scheduler, not a model or effect executor. It cannot authorize a
//! repository write. Trusted tenant/role MUST come from authenticated ingress.
//! Expired attempts are uncertain and NEVER automatically retried.
//! State is in-memory: persistence and cross-host fencing remain separate gates.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_NODES: usize = 32;
const MAX_ATTEMPTS: u8 = 8;
const MAX_LEASE_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Planner,
    Architect,
    Coder,
    SecurityCritic,
    Tester,
    Verifier,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepContract {
    pub id: String,
    pub role: Role,
    pub prerequisites: Vec<String>,
    pub max_attempts: u8,
    pub max_cost_minor: u64,
    pub lease_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowContract {
    pub tenant: String,
    pub workflow_id: String,
    pub base_commit_sha: String,
    pub deadline_unix: u64,
    pub max_cost_minor: u64,
    pub steps: Vec<StepContract>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub step_id: String,
    pub epoch: u64,
    pub attempt_id: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Ready,
    Leased(Claim),
    Passed { evidence_sha256: String },
    Halted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub epoch: u64,
    pub attempt_id: String,
    pub outcome: Outcome,
    pub evidence_sha256: String,
    pub actual_cost_minor: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    Recorded,
    Replay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutResolution {
    NoEffectConfirmed,
    UnknownOutcome,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkflowError {
    #[error("invalid workflow contract")]
    Invalid,
    #[error("workflow contains missing dependencies or a cycle")]
    DependencyGraph,
    #[error("trusted tenant or agent role does not match contract")]
    Denied,
    #[error("workflow expired or cost budget exhausted")]
    BudgetOrDeadline,
    #[error("a prerequisite has not passed")]
    NotReady,
    #[error("step is already running or terminal")]
    BusyOrTerminal,
    #[error("expired lease needs explicit outcome reconciliation")]
    Unresolved,
    #[error("stale epoch, lease or attempt identity")]
    Fenced,
    #[error("invalid or conflicting completion receipt")]
    ReceiptConflict,
}

#[derive(Debug, Clone)]
struct StepRuntime {
    status: StepStatus,
    attempts: u8,
    last_receipt: Option<Receipt>,
}

#[derive(Debug)]
pub struct OmniDevWorkflow {
    contract: WorkflowContract,
    nodes: BTreeMap<String, StepRuntime>,
    cost_spent_minor: u64,
    cost_reserved_minor: u64,
    last_epoch: u64,
    audit_chain: String,
    audit_count: u64,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

fn valid_git_sha(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn valid_sha(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn visit(
    id: &str,
    contracts: &BTreeMap<String, &StepContract>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<(), WorkflowError> {
    if visited.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id.into()) {
        return Err(WorkflowError::DependencyGraph);
    }
    let contract = contracts.get(id).ok_or(WorkflowError::DependencyGraph)?;
    for prerequisite in &contract.prerequisites {
        visit(prerequisite, contracts, visiting, visited)?;
    }
    visiting.remove(id);
    visited.insert(id.into());
    Ok(())
}

impl OmniDevWorkflow {
    pub fn new(contract: WorkflowContract) -> Result<Self, WorkflowError> {
        if !valid_id(&contract.tenant)
            || !valid_id(&contract.workflow_id)
            || !valid_git_sha(&contract.base_commit_sha)
            || contract.deadline_unix == 0
            || contract.max_cost_minor == 0
            || contract.steps.is_empty()
            || contract.steps.len() > MAX_NODES
        {
            return Err(WorkflowError::Invalid);
        }
        let mut map = BTreeMap::new();
        for step in &contract.steps {
            if !valid_id(&step.id)
                || step.max_attempts == 0
                || step.max_attempts > MAX_ATTEMPTS
                || step.lease_seconds == 0
                || step.lease_seconds > MAX_LEASE_SECS
                || step.max_cost_minor == 0
                || step.max_cost_minor > contract.max_cost_minor
                || step.prerequisites.len() >= MAX_NODES
            {
                return Err(WorkflowError::Invalid);
            }
            let unique = step.prerequisites.iter().collect::<BTreeSet<_>>();
            if unique.len() != step.prerequisites.len() {
                return Err(WorkflowError::DependencyGraph);
            }
            if map.insert(step.id.clone(), step).is_some() {
                return Err(WorkflowError::Invalid);
            }
        }
        let mut visited = BTreeSet::new();
        for id in map.keys() {
            visit(id, &map, &mut BTreeSet::new(), &mut visited)?;
        }
        let nodes = map.keys().map(|id| {
            (id.clone(), StepRuntime {
                status: StepStatus::Ready,
                attempts: 0,
                last_receipt: None,
            })
        }).collect();
        Ok(Self {
            contract,
            nodes,
            cost_spent_minor: 0,
            cost_reserved_minor: 0,
            last_epoch: 0,
            audit_chain: format!("{:x}", Sha256::digest(b"OCAF-OMNIDEV-v1-GENESIS")),
            audit_count: 0,
        })
    }

    fn authorized(&self, trusted_tenant: &str, role: Role, id: &str) -> Result<(), WorkflowError> {
        if self.contract.tenant != trusted_tenant {
            return Err(WorkflowError::Denied);
        }
        match self.contract.steps.iter().find(|s| s.id == id) {
            Some(step) if step.role == role => Ok(()),
            _ => Err(WorkflowError::Denied),
        }
    }

    fn append_audit(&mut self, event: &str, id: &str, epoch: u64, evidence: &str) {
        let mut digest = Sha256::new();
        for field in [
            &b"OCAF-OMNIDEV-v1"[..],
            self.contract.tenant.as_bytes(),
            self.contract.workflow_id.as_bytes(),
            self.audit_chain.as_bytes(),
            event.as_bytes(),
            id.as_bytes(),
            &epoch.to_be_bytes(),
            evidence.as_bytes(),
        ] {
            digest.update((field.len() as u64).to_be_bytes());
            digest.update(field);
        }
        self.audit_chain = format!("{:x}", digest.finalize());
        self.audit_count += 1;
    }

    /// The strictly increasing epoch must be issued by a trusted coordinator.
    /// No candidate code runs; only a bounded claim token is returned.
    pub fn claim(
        &mut self,
        trusted_tenant: &str,
        trusted_role: Role,
        id: &str,
        epoch: u64,
        attempt_id: &str,
        now_unix: u64,
    ) -> Result<Claim, WorkflowError> {
        self.authorized(trusted_tenant, trusted_role, id)?;
        if !valid_id(attempt_id) || epoch == 0 || epoch <= self.last_epoch {
            return Err(WorkflowError::Fenced);
        }
        if now_unix >= self.contract.deadline_unix {
            return Err(WorkflowError::BudgetOrDeadline);
        }
        let step = self.contract.steps.iter().find(|s| s.id == id).ok_or(WorkflowError::Denied)?;
        for prerequisite in &step.prerequisites {
            if !matches!(self.nodes.get(prerequisite).map(|n| &n.status), Some(StepStatus::Passed { .. })) {
                return Err(WorkflowError::NotReady);
            }
        }
        let state = self.nodes.get(id).ok_or(WorkflowError::Denied)?;
        if let StepStatus::Leased(claim) = &state.status {
            if now_unix >= claim.expires_at {
                return Err(WorkflowError::Unresolved);
            }
        }
        if !matches!(state.status, StepStatus::Ready) || state.attempts >= step.max_attempts {
            return Err(WorkflowError::BusyOrTerminal);
        }
        if self.cost_spent_minor.saturating_add(self.cost_reserved_minor)
            .saturating_add(step.max_cost_minor) > self.contract.max_cost_minor
        {
            return Err(WorkflowError::BudgetOrDeadline);
        }
        let expires_at = now_unix.checked_add(step.lease_seconds)
            .ok_or(WorkflowError::BudgetOrDeadline)?;
        let expires_at = expires_at.min(self.contract.deadline_unix);
        let claim = Claim {
            step_id: id.into(),
            epoch,
            attempt_id: attempt_id.into(),
            expires_at,
        };
        let state = self.nodes.get_mut(id).ok_or(WorkflowError::Denied)?;
        state.status = StepStatus::Leased(claim.clone());
        state.attempts += 1;
        self.last_epoch = epoch;
        self.cost_reserved_minor += step.max_cost_minor;
        self.append_audit("CLAIM", id, epoch, attempt_id);
        Ok(claim)
    }

    /// The matching live lease and worker role are required to complete.
    pub fn complete(
        &mut self,
        trusted_tenant: &str,
        trusted_role: Role,
        id: &str,
        receipt: Receipt,
        now_unix: u64,
    ) -> Result<Completion, WorkflowError> {
        self.authorized(trusted_tenant, trusted_role, id)?;
        if !valid_sha(&receipt.evidence_sha256) || !valid_id(&receipt.attempt_id) {
            return Err(WorkflowError::ReceiptConflict);
        }
        let step = self.contract.steps.iter().find(|s| s.id == id).ok_or(WorkflowError::Denied)?;
        let state = self.nodes.get(id).ok_or(WorkflowError::Denied)?;
        if state.last_receipt.as_ref() == Some(&receipt) {
            return Ok(Completion::Replay);
        }
        let StepStatus::Leased(claim) = &state.status else {
            return Err(WorkflowError::ReceiptConflict);
        };
        if receipt.epoch != claim.epoch || receipt.attempt_id != claim.attempt_id {
            return Err(WorkflowError::Fenced);
        }
        if now_unix >= claim.expires_at || now_unix >= self.contract.deadline_unix {
            return Err(WorkflowError::Unresolved);
        }
        if receipt.actual_cost_minor > step.max_cost_minor {
            return Err(WorkflowError::BudgetOrDeadline);
        }
        self.cost_reserved_minor -= step.max_cost_minor;
        self.cost_spent_minor += receipt.actual_cost_minor;
        let state = self.nodes.get_mut(id).ok_or(WorkflowError::Denied)?;
        state.status = match receipt.outcome {
            Outcome::Passed => StepStatus::Passed { evidence_sha256: receipt.evidence_sha256.clone() },
            Outcome::Failed if state.attempts < step.max_attempts => StepStatus::Ready,
            Outcome::Failed => StepStatus::Halted,
        };
        state.last_receipt = Some(receipt.clone());
        self.append_audit("COMPLETE", id, receipt.epoch, &receipt.evidence_sha256);
        Ok(Completion::Recorded)
    }

    /// Explicit timeout reconciliation. Trusted verifier MUST substantiate
    /// NoEffectConfirmed externally; UNKNOWN is terminal and never retried.
    pub fn reconcile_timeout(
        &mut self,
        trusted_tenant: &str,
        trusted_role: Role,
        id: &str,
        epoch: u64,
        resolution: TimeoutResolution,
        evidence_sha256: &str,
        now_unix: u64,
    ) -> Result<(), WorkflowError> {
        if trusted_tenant != self.contract.tenant || trusted_role != Role::Verifier {
            return Err(WorkflowError::Denied);
        }
        if !valid_sha(evidence_sha256) {
            return Err(WorkflowError::ReceiptConflict);
        }
        let state = self.nodes.get(id).ok_or(WorkflowError::Denied)?;
        let StepStatus::Leased(claim) = &state.status else {
            return Err(WorkflowError::BusyOrTerminal);
        };
        if epoch != claim.epoch || now_unix < claim.expires_at {
            return Err(WorkflowError::Fenced);
        }
        let step = self.contract.steps.iter().find(|s| s.id == id).ok_or(WorkflowError::Denied)?;
        let max_attempts = step.max_attempts;
        let reserved = step.max_cost_minor;
        self.cost_reserved_minor -= reserved;
        let state = self.nodes.get_mut(id).ok_or(WorkflowError::Denied)?;
        state.status = match resolution {
            TimeoutResolution::NoEffectConfirmed if state.attempts < max_attempts => StepStatus::Ready,
            _ => StepStatus::Halted,
        };
        self.append_audit("RECONCILE", id, epoch, evidence_sha256);
        Ok(())
    }

    pub fn status(&self, trusted_tenant: &str, id: &str) -> Result<&StepStatus, WorkflowError> {
        if self.contract.tenant != trusted_tenant {
            return Err(WorkflowError::Denied);
        }
        self.nodes.get(id).map(|n| &n.status).ok_or(WorkflowError::Denied)
    }

    pub fn is_completed(&self) -> bool {
        self.nodes.values().all(|n| matches!(n.status, StepStatus::Passed { .. }))
    }

    pub fn budget_state(&self) -> (u64, u64) {
        (self.cost_spent_minor, self.cost_reserved_minor)
    }

    /// Tamper-evident only when the returned head is pinned externally.
    pub fn audit_head(&self) -> (u64, &str) {
        (self.audit_count, &self.audit_chain)
    }
}
