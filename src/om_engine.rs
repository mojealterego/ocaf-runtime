//! OM Engine: durable, tenant-scoped orchestration for the existing OmniDev scheduler.
//!
//! This is NOT an executor, an LLM, an authentication gateway or a tool grant.
//! Trusted tenant/role, epoch, time and evidence must originate outside this module.
//! SQLite event chains detect accidental corruption; external anchoring is needed
//! to resist a malicious database administrator or rollback.
use crate::omnidev_workflow::{
    Claim, Completion, OmniDevWorkflow, Outcome, Receipt, Role, StepStatus,
    TimeoutReconciliation, TimeoutResolution, WorkflowContract, WorkflowError,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Mutex, time::Duration};
use thiserror::Error;

const MAX_EVENTS: u64 = 4096;
const MAX_CONTRACT_BYTES: usize = 128 * 1024;
const MAX_MEMORY_BYTES: usize = 32 * 1024;
const BRIDGE_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum OmError {
    #[error("invalid or excessive OM Engine input")]
    Invalid,
    #[error("authenticated tenant does not match the resource")]
    Denied,
    #[error("workflow does not exist")]
    NotFound,
    #[error("workflow id already has a different contract")]
    Conflict,
    #[error("event journal integrity or replay failed")]
    Integrity,
    #[error("event journal capacity exhausted")]
    Capacity,
    #[error("OM Engine database lock poisoned")]
    Poisoned,
    #[error(transparent)]
    Workflow(#[from] WorkflowError),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditHead {
    pub events: u64,
    pub event_sha256: String,
    pub scheduler_events: u64,
    pub scheduler_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum EngineEvent {
    Claim {
        role: Role,
        step: String,
        epoch: u64,
        attempt: String,
        now_unix: u64,
    },
    Complete {
        role: Role,
        step: String,
        epoch: u64,
        attempt: String,
        passed: bool,
        evidence_sha256: String,
        actual_cost_minor: u64,
        now_unix: u64,
    },
    Reconcile {
        step: String,
        epoch: u64,
        no_effect_confirmed: bool,
        evidence_sha256: String,
        now_unix: u64,
    },
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn genesis() -> String {
    digest(b"OCAF-OM-ENGINE-JOURNAL-v1-GENESIS")
}

fn event_hash(previous: &str, event_json: &str) -> String {
    let mut hasher = Sha256::new();
    for part in [b"OCAF-OM-ENGINE-EVENT-v1".as_slice(), previous.as_bytes(), event_json.as_bytes()] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    format!("{:x}", hasher.finalize())
}

fn replay(workflow: &mut OmniDevWorkflow, tenant: &str, event: EngineEvent) -> Result<(), OmError> {
    match event {
        EngineEvent::Claim { role, step, epoch, attempt, now_unix } => {
            workflow.claim(tenant, role, &step, epoch, &attempt, now_unix)?;
        }
        EngineEvent::Complete {
            role, step, epoch, attempt, passed, evidence_sha256, actual_cost_minor, now_unix,
        } => {
            let outcome = if passed { Outcome::Passed } else { Outcome::Failed };
            let recorded = workflow.complete(tenant, role, &step, Receipt {
                epoch, attempt_id: attempt, outcome, evidence_sha256, actual_cost_minor,
            }, now_unix)?;
            if recorded != Completion::Recorded {
                return Err(OmError::Integrity);
            }
        }
        EngineEvent::Reconcile {
            step, epoch, no_effect_confirmed, evidence_sha256, now_unix,
        } => {
            workflow.reconcile_timeout(tenant, Role::Verifier, &step, TimeoutReconciliation {
                epoch,
                resolution: if no_effect_confirmed {
                    TimeoutResolution::NoEffectConfirmed
                } else {
                    TimeoutResolution::UnknownOutcome
                },
                evidence_sha256: &evidence_sha256,
                now_unix,
            })?;
        }
    }
    Ok(())
}

fn load(tx: &Transaction<'_>, tenant: &str, workflow_id: &str) -> Result<(OmniDevWorkflow, u64, String), OmError> {
    let row: Option<(String, String)> = tx.query_row(
        "SELECT contract_json, contract_sha256 FROM om_workflows WHERE tenant=?1 AND workflow_id=?2",
        params![tenant, workflow_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?;
    let (contract_json, contract_sha) = row.ok_or(OmError::NotFound)?;
    if contract_json.len() > MAX_CONTRACT_BYTES || digest(contract_json.as_bytes()) != contract_sha {
        return Err(OmError::Integrity);
    }
    let contract: WorkflowContract = serde_json::from_str(&contract_json).map_err(|_| OmError::Integrity)?;
    if contract.tenant != tenant || contract.workflow_id != workflow_id {
        return Err(OmError::Integrity);
    }
    let mut workflow = OmniDevWorkflow::new(contract).map_err(|_| OmError::Integrity)?;
    let mut statement = tx.prepare(
        "SELECT seq, payload_json, previous_sha256, event_sha256 FROM om_events \
         WHERE tenant=?1 AND workflow_id=?2 ORDER BY seq ASC",
    )?;
    let mut rows = statement.query(params![tenant, workflow_id])?;
    let mut count = 0_u64;
    let mut previous = genesis();
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let payload: String = row.get(1)?;
        let prev: String = row.get(2)?;
        let hash: String = row.get(3)?;
        count = count.checked_add(1).ok_or(OmError::Integrity)?;
        if count > MAX_EVENTS || seq != count as i64 || prev != previous || event_hash(&prev, &payload) != hash {
            return Err(OmError::Integrity);
        }
        let event: EngineEvent = serde_json::from_str(&payload).map_err(|_| OmError::Integrity)?;
        replay(&mut workflow, tenant, event).map_err(|_| OmError::Integrity)?;
        previous = hash;
    }
    Ok((workflow, count, previous))
}

/// A single-process, transactional SQLite journal. Each operation reconstructs
/// the scheduler from committed events under an IMMEDIATE transaction, avoiding
/// divergence between an in-memory scheduler and a failed database commit.
/// Cross-process writers are serialized by SQLite; this is not distributed HA.
pub struct OmEngine {
    db: Mutex<Connection>,
}

impl OmEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, OmError> {
        if path.as_ref().symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(OmError::Invalid);
        }
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; \
             CREATE TABLE IF NOT EXISTS om_workflows ( \
               tenant TEXT NOT NULL, workflow_id TEXT NOT NULL, \
               contract_json TEXT NOT NULL, contract_sha256 TEXT NOT NULL, \
               PRIMARY KEY(tenant,workflow_id)); \
             CREATE TABLE IF NOT EXISTS om_events ( \
               tenant TEXT NOT NULL, workflow_id TEXT NOT NULL, seq INTEGER NOT NULL, \
               payload_json TEXT NOT NULL, previous_sha256 TEXT NOT NULL, event_sha256 TEXT NOT NULL, \
               PRIMARY KEY(tenant,workflow_id,seq), \
               FOREIGN KEY(tenant,workflow_id) REFERENCES om_workflows(tenant,workflow_id));",
        )?;
        Ok(Self { db: Mutex::new(db) })
    }

    pub fn register(&self, trusted_tenant: &str, contract: WorkflowContract) -> Result<(), OmError> {
        if !valid_id(trusted_tenant) || contract.tenant != trusted_tenant {
            return Err(OmError::Denied);
        }
        OmniDevWorkflow::new(contract.clone())?;
        let json = serde_json::to_string(&contract)?;
        if json.len() > MAX_CONTRACT_BYTES { return Err(OmError::Invalid); }
        let hash = digest(json.as_bytes());
        let mut db = self.db.lock().map_err(|_| OmError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT OR IGNORE INTO om_workflows(tenant,workflow_id,contract_json,contract_sha256) VALUES (?1,?2,?3,?4)",
            params![trusted_tenant, contract.workflow_id, json, hash],
        )?;
        let stored: (String, String) = tx.query_row(
            "SELECT contract_json,contract_sha256 FROM om_workflows WHERE tenant=?1 AND workflow_id=?2",
            params![trusted_tenant, contract.workflow_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if stored != (json, hash) { return Err(OmError::Conflict); }
        tx.commit()?;
        Ok(())
    }

    fn change<T>(&self, trusted_tenant: &str, workflow_id: &str,
        operation: impl FnOnce(&mut OmniDevWorkflow) -> Result<(T, Option<EngineEvent>), OmError>,
    ) -> Result<T, OmError> {
        if !valid_id(trusted_tenant) || !valid_id(workflow_id) { return Err(OmError::Invalid); }
        let mut db = self.db.lock().map_err(|_| OmError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (mut workflow, count, previous) = load(&tx, trusted_tenant, workflow_id)?;
        let (result, event) = operation(&mut workflow)?;
        if let Some(event) = event {
            if count >= MAX_EVENTS { return Err(OmError::Capacity); }
            let payload = serde_json::to_string(&event)?;
            let hash = event_hash(&previous, &payload);
            tx.execute(
                "INSERT INTO om_events(tenant,workflow_id,seq,payload_json,previous_sha256,event_sha256) VALUES (?1,?2,?3,?4,?5,?6)",
                params![trusted_tenant, workflow_id, (count + 1) as i64, payload, previous, hash],
            )?;
        }
        tx.commit()?;
        Ok(result)
    }

    pub fn claim(&self, tenant: &str, workflow_id: &str, role: Role, step: &str,
        epoch: u64, attempt: &str, now_unix: u64) -> Result<Claim, OmError> {
        self.change(tenant, workflow_id, |wf| {
            let claim = wf.claim(tenant, role, step, epoch, attempt, now_unix)?;
            Ok((claim, Some(EngineEvent::Claim {
                role, step: step.to_owned(), epoch, attempt: attempt.to_owned(), now_unix,
            })))
        })
    }

    pub fn complete(&self, tenant: &str, workflow_id: &str, role: Role, step: &str,
        receipt: Receipt, now_unix: u64) -> Result<Completion, OmError> {
        self.change(tenant, workflow_id, |wf| {
            let completion = wf.complete(tenant, role, step, receipt.clone(), now_unix)?;
            let event = (completion == Completion::Recorded).then(|| EngineEvent::Complete {
                role, step: step.to_owned(), epoch: receipt.epoch,
                attempt: receipt.attempt_id, passed: receipt.outcome == Outcome::Passed,
                evidence_sha256: receipt.evidence_sha256,
                actual_cost_minor: receipt.actual_cost_minor, now_unix,
            });
            Ok((completion, event))
        })
    }

    /// Only a separately trusted Verifier can reconcile an expired lease.
    /// UNKNOWN outcomes are terminal and never automatically retried.
    pub fn reconcile(&self, tenant: &str, workflow_id: &str, role: Role, step: &str,
        epoch: u64, resolution: TimeoutResolution, evidence_sha256: &str,
        now_unix: u64) -> Result<(), OmError> {
        if role != Role::Verifier { return Err(OmError::Denied); }
        self.change(tenant, workflow_id, |wf| {
            wf.reconcile_timeout(tenant, role, step, TimeoutReconciliation {
                epoch, resolution, evidence_sha256, now_unix,
            })?;
            Ok(((), Some(EngineEvent::Reconcile {
                step: step.to_owned(), epoch,
                no_effect_confirmed: resolution == TimeoutResolution::NoEffectConfirmed,
                evidence_sha256: evidence_sha256.to_owned(), now_unix,
            })))
        })
    }

    pub fn status(&self, tenant: &str, workflow_id: &str, step: &str) -> Result<StepStatus, OmError> {
        if !valid_id(tenant) || !valid_id(workflow_id) { return Err(OmError::Invalid); }
        let mut db = self.db.lock().map_err(|_| OmError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (wf, _, _) = load(&tx, tenant, workflow_id)?;
        let status = wf.status(tenant, step)?.clone();
        tx.commit()?;
        Ok(status)
    }

    pub fn audit_head(&self, tenant: &str, workflow_id: &str) -> Result<AuditHead, OmError> {
        if !valid_id(tenant) || !valid_id(workflow_id) { return Err(OmError::Invalid); }
        let mut db = self.db.lock().map_err(|_| OmError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (wf, events, event_sha256) = load(&tx, tenant, workflow_id)?;
        let (scheduler_events, scheduler_sha256) = wf.audit_head(tenant)?;
        let result = AuditHead { events, event_sha256, scheduler_events, scheduler_sha256: scheduler_sha256.to_owned() };
        tx.commit()?;
        Ok(result)
    }

    /// An inert, typed JSON interchange envelope for future Digital-Life
    /// integration. It does not authenticate peers or execute payloads.
    pub fn export_memory(&self, tenant: &str, workflow_id: &str,
        namespace: &str, record_id: &str, payload: Value) -> Result<MemoryEnvelope, OmError> {
        self.audit_head(tenant, workflow_id)?;
        MemoryEnvelope::new(tenant, workflow_id, namespace, record_id, payload)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MemoryEnvelope {
    pub schema_version: u32,
    pub source_engine: String,
    pub tenant_id: String,
    pub workflow_id: String,
    pub namespace: String,
    pub record_id: String,
    pub payload: Value,
    pub payload_sha256: String,
}

impl MemoryEnvelope {
    pub fn new(tenant: &str, workflow: &str, namespace: &str,
        record: &str, payload: Value) -> Result<Self, OmError> {
        if [tenant, workflow, namespace, record].iter().any(|v| !valid_id(v)) {
            return Err(OmError::Invalid);
        }
        if !payload.is_object() { return Err(OmError::Invalid); }
        let encoded = serde_json::to_vec(&payload)?;
        if encoded.is_empty() || encoded.len() > MAX_MEMORY_BYTES { return Err(OmError::Invalid); }
        Ok(Self {
            schema_version: BRIDGE_VERSION, source_engine: "ocaf.om_engine".to_owned(),
            tenant_id: tenant.to_owned(), workflow_id: workflow.to_owned(),
            namespace: namespace.to_owned(), record_id: record.to_owned(),
            payload_sha256: digest(&encoded), payload,
        })
    }

    /// Data integrity and tenant binding only. The caller MUST separately
    /// authenticate the Digital-Life peer and authorize the target namespace.
    pub fn verify(&self, trusted_tenant: &str) -> Result<(), OmError> {
        if self.schema_version != BRIDGE_VERSION
            || !matches!(self.source_engine.as_str(), "ocaf.om_engine" | "digital-life.noevaris")
            || !valid_id(trusted_tenant)
            || self.tenant_id != trusted_tenant
            || [self.workflow_id.as_str(), self.namespace.as_str(), self.record_id.as_str()]
                .iter().any(|v| !valid_id(v))
            || !self.payload.is_object()
        {
            return Err(OmError::Denied);
        }
        let encoded = serde_json::to_vec(&self.payload)?;
        if encoded.is_empty() || encoded.len() > MAX_MEMORY_BYTES
            || digest(&encoded) != self.payload_sha256 {
            return Err(OmError::Integrity);
        }
        Ok(())
    }
}
