//! Cryptographically pinned, single-use operator approval for OCAF evolution.
//!
//! AB-OM-09 (partial): an Ed25519-signed grant for EXACT candidate digest,
//! source revision, tenant, fleet, proposal ID and expiration. Signatures are
//! verified using a public key configured by the trusted host; the Architect
//! cannot sign its own approval. This store does not deploy anything.
//! The host MUST authenticate staging/release callers and secure its SQLite
//! database against rollback; an external audit anchor is needed for the latter.

use openssl::{
    pkey::{Id, PKey, Public},
    sign::Verifier,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex, time::Duration};
use thiserror::Error;

const MAX_UNIX: i64 = 253_402_300_799;
const MAX_TTL_SECONDS: i64 = 31 * 86_400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvolutionCandidate {
    pub tenant: String,
    pub fleet: String,
    pub proposal_id: String,
    /// Lowercase hexadecimal SHA-256 of the exact staged candidate package.
    pub candidate_sha256: String,
    /// Lowercase hexadecimal SHA-256 of the exact evaluated source revision.
    pub revision_sha256: String,
    pub expires_unix: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalState {
    AwaitingOperator,
    Approved,
    Consumed,
}

impl ApprovalState {
    fn parse(code: &str) -> Result<Self, ApprovalError> {
        match code {
            "AWAITING_OPERATOR" => Ok(Self::AwaitingOperator),
            "APPROVED" => Ok(Self::Approved),
            "CONSUMED" => Ok(Self::Consumed),
            _ => Err(ApprovalError::Integrity),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalSnapshot {
    pub candidate: EvolutionCandidate,
    pub state: ApprovalState,
    pub approved_at_unix: Option<i64>,
    pub consumed_at_unix: Option<i64>,
    pub release_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum ApprovalError {
    #[error("invalid operator-approval input")]
    Invalid,
    #[error("authenticated tenant does not match the candidate")]
    Denied,
    #[error("candidate not found")]
    NotFound,
    #[error("candidate cannot be changed after staging")]
    Conflict,
    #[error("operator signature is missing or invalid")]
    Signature,
    #[error("operator consent has expired")]
    Expired,
    #[error("operator approval has already been consumed")]
    Replay,
    #[error("stored approval integrity check failed")]
    Integrity,
    #[error("approval database lock poisoned")]
    Poisoned,
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub struct EvolutionApprovalStore {
    db: Mutex<Connection>,
    operator_key: PKey<Public>,
}

struct StoredCandidate {
    candidate: EvolutionCandidate,
    state: ApprovalState,
    signature: Option<Vec<u8>>,
    approved_at_unix: Option<i64>,
    consumed_at_unix: Option<i64>,
    release_id: Option<String>,
}

fn valid_id(input: &str) -> bool {
    !input.is_empty()
        && input.len() <= 96
        && input
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_time(value: i64) -> bool {
    (0..=MAX_UNIX).contains(&value)
}

fn signed_message(candidate: &EvolutionCandidate) -> Vec<u8> {
    let mut msg = Vec::with_capacity(400);
    for chunk in [
        b"OCAF-ARCHITECT-EVOLUTION-APPROVAL-v1".as_slice(),
        candidate.tenant.as_bytes(),
        candidate.fleet.as_bytes(),
        candidate.proposal_id.as_bytes(),
        candidate.candidate_sha256.as_bytes(),
        candidate.revision_sha256.as_bytes(),
        &candidate.expires_unix.to_be_bytes(),
    ] {
        msg.extend_from_slice(&(chunk.len() as u32).to_be_bytes());
        msg.extend_from_slice(chunk);
    }
    msg
}

impl EvolutionApprovalStore {
    /// Pinned operator public key MUST come from an authenticated control
    /// plane, never from the Architect or an untrusted request field.
    pub fn open(
        path: impl AsRef<Path>,
        trusted_operator_pubkey: &[u8],
    ) -> Result<Self, ApprovalError> {
        if path
            .as_ref()
            .symlink_metadata()
            .is_ok_and(|meta| meta.file_type().is_symlink())
        {
            return Err(ApprovalError::Invalid);
        }
        let operator_key = PKey::public_key_from_raw_bytes(trusted_operator_pubkey, Id::ED25519)
            .map_err(|_| ApprovalError::Invalid)?;
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS architect_evolution_approvals (
               tenant TEXT NOT NULL, fleet TEXT NOT NULL, proposal_id TEXT NOT NULL,
               candidate_json TEXT NOT NULL,
               state TEXT NOT NULL CHECK(state IN ('AWAITING_OPERATOR','APPROVED','CONSUMED')),
               signature BLOB, approved_at INTEGER, consumed_at INTEGER, release_id TEXT,
               PRIMARY KEY (tenant,fleet,proposal_id)
             );",
        )?;
        Ok(Self {
            db: Mutex::new(db),
            operator_key,
        })
    }

    /// Immutable staging only; no operator approval is inferred from a draft.
    pub fn stage(
        &self,
        trusted_tenant: &str,
        candidate: EvolutionCandidate,
        now_unix: i64,
    ) -> Result<(), ApprovalError> {
        if !valid_id(trusted_tenant) || candidate.tenant != trusted_tenant {
            return Err(ApprovalError::Denied);
        }
        if !valid_id(&candidate.fleet)
            || !valid_id(&candidate.proposal_id)
            || !valid_hash(&candidate.candidate_sha256)
            || !valid_hash(&candidate.revision_sha256)
            || !valid_time(now_unix)
            || !valid_time(candidate.expires_unix)
            || !(1..=MAX_TTL_SECONDS).contains(&(candidate.expires_unix - now_unix))
        {
            return Err(ApprovalError::Invalid);
        }
        let json = serde_json::to_string(&candidate)?;
        let mut db = self.db.lock().map_err(|_| ApprovalError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT OR IGNORE INTO architect_evolution_approvals
             (tenant,fleet,proposal_id,candidate_json,state)
             VALUES (?1,?2,?3,?4,'AWAITING_OPERATOR')",
            params![trusted_tenant, candidate.fleet, candidate.proposal_id, json],
        )?;
        let existing: String = tx.query_row(
            "SELECT candidate_json FROM architect_evolution_approvals
             WHERE tenant=?1 AND fleet=?2 AND proposal_id=?3",
            params![trusted_tenant, candidate.fleet, candidate.proposal_id],
            |r| r.get(0),
        )?;
        if existing != json {
            return Err(ApprovalError::Conflict);
        }
        tx.commit()?;
        Ok(())
    }

    fn load(
        tx: &rusqlite::Transaction<'_>,
        tenant: &str,
        fleet: &str,
        proposal_id: &str,
    ) -> Result<StoredCandidate, ApprovalError> {
        struct StoredRow {
            json: String,
            state: String,
            signature: Option<Vec<u8>>,
            approved_at_unix: Option<i64>,
            consumed_at_unix: Option<i64>,
            release_id: Option<String>,
        }
        let row: Option<StoredRow> = tx
            .query_row(
                "SELECT candidate_json,state,signature,approved_at,consumed_at,release_id
             FROM architect_evolution_approvals
             WHERE tenant=?1 AND fleet=?2 AND proposal_id=?3",
                params![tenant, fleet, proposal_id],
                |r| {
                    Ok(StoredRow {
                        json: r.get(0)?,
                        state: r.get(1)?,
                        signature: r.get(2)?,
                        approved_at_unix: r.get(3)?,
                        consumed_at_unix: r.get(4)?,
                        release_id: r.get(5)?,
                    })
                },
            )
            .optional()?;
        let StoredRow {
            json,
            state,
            signature,
            approved_at_unix,
            consumed_at_unix,
            release_id,
        } = row.ok_or(ApprovalError::NotFound)?;
        if json.len() > 1000 {
            return Err(ApprovalError::Integrity);
        }
        let candidate: EvolutionCandidate =
            serde_json::from_str(&json).map_err(|_| ApprovalError::Integrity)?;
        if candidate.tenant != tenant
            || candidate.fleet != fleet
            || candidate.proposal_id != proposal_id
            || !valid_hash(&candidate.candidate_sha256)
            || !valid_hash(&candidate.revision_sha256)
            || !valid_time(candidate.expires_unix)
        {
            return Err(ApprovalError::Integrity);
        }
        let state = ApprovalState::parse(&state)?;
        match state {
            ApprovalState::AwaitingOperator => {
                if signature.is_some()
                    || approved_at_unix.is_some()
                    || release_id.is_some()
                    || consumed_at_unix.is_some()
                {
                    return Err(ApprovalError::Integrity);
                }
            }
            ApprovalState::Approved => {
                if signature.as_ref().is_none_or(|s| s.len() != 64)
                    || approved_at_unix.is_none()
                    || release_id.is_some()
                    || consumed_at_unix.is_some()
                {
                    return Err(ApprovalError::Integrity);
                }
            }
            ApprovalState::Consumed => {
                if signature.as_ref().is_none_or(|s| s.len() != 64)
                    || approved_at_unix.is_none()
                    || consumed_at_unix.is_none()
                    || release_id.as_deref().is_none_or(|s| !valid_id(s))
                {
                    return Err(ApprovalError::Integrity);
                }
            }
        }
        Ok(StoredCandidate {
            candidate,
            state,
            signature,
            approved_at_unix,
            consumed_at_unix,
            release_id,
        })
    }

    fn verify_signature(&self, row: &StoredCandidate) -> Result<(), ApprovalError> {
        if row.state == ApprovalState::AwaitingOperator {
            return Err(ApprovalError::Denied);
        }
        let signature = row.signature.as_deref().ok_or(ApprovalError::Integrity)?;
        let mut verifier = Verifier::new_without_digest(&self.operator_key)
            .map_err(|_| ApprovalError::Signature)?;
        let valid = verifier
            .verify_oneshot(signature, &signed_message(&row.candidate))
            .map_err(|_| ApprovalError::Signature)?;
        if !valid {
            return Err(ApprovalError::Signature);
        }
        Ok(())
    }

    /// Exact bytes to present to the operator and sign with the pinned
    /// Ed25519 private key outside this component. No signing key is stored.
    pub fn signing_message(
        &self,
        tenant: &str,
        fleet: &str,
        proposal_id: &str,
    ) -> Result<Vec<u8>, ApprovalError> {
        if !valid_id(tenant) || !valid_id(fleet) || !valid_id(proposal_id) {
            return Err(ApprovalError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ApprovalError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = Self::load(&tx, tenant, fleet, proposal_id)?;
        let message = signed_message(&row.candidate);
        tx.commit()?;
        Ok(message)
    }

    /// Verify a genuinely pinned operator signature. There is no API to
    /// self-approve by flag, caller-supplied key or unsigned token.
    pub fn approve(
        &self,
        tenant: &str,
        fleet: &str,
        proposal_id: &str,
        signature: &[u8],
        now_unix: i64,
    ) -> Result<(), ApprovalError> {
        if !valid_id(tenant)
            || !valid_id(fleet)
            || !valid_id(proposal_id)
            || !valid_time(now_unix)
            || signature.len() != 64
        {
            return Err(ApprovalError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ApprovalError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = Self::load(&tx, tenant, fleet, proposal_id)?;
        if now_unix > row.candidate.expires_unix {
            return Err(ApprovalError::Expired);
        }
        if row.state != ApprovalState::AwaitingOperator {
            return Err(ApprovalError::Conflict);
        }
        let mut verifier = Verifier::new_without_digest(&self.operator_key)
            .map_err(|_| ApprovalError::Signature)?;
        if !verifier
            .verify_oneshot(signature, &signed_message(&row.candidate))
            .map_err(|_| ApprovalError::Signature)?
        {
            return Err(ApprovalError::Signature);
        }
        tx.execute(
            "UPDATE architect_evolution_approvals
             SET state='APPROVED',signature=?4,approved_at=?5
             WHERE tenant=?1 AND fleet=?2 AND proposal_id=?3",
            params![tenant, fleet, proposal_id, signature, now_unix],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Authorizes one exact release request after re-verification of operator
    /// signature; a separate admission pipeline MUST still enforce security,
    /// tests, signing, OSTc/PQC, availability and deployment authorization.
    #[allow(clippy::too_many_arguments)]
    pub fn consume_for_release(
        &self,
        tenant: &str,
        fleet: &str,
        proposal_id: &str,
        expected_candidate_sha256: &str,
        expected_revision_sha256: &str,
        release_id: &str,
        now_unix: i64,
    ) -> Result<(), ApprovalError> {
        if !valid_id(tenant)
            || !valid_id(fleet)
            || !valid_id(proposal_id)
            || !valid_id(release_id)
            || !valid_time(now_unix)
            || !valid_hash(expected_candidate_sha256)
            || !valid_hash(expected_revision_sha256)
        {
            return Err(ApprovalError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ApprovalError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = Self::load(&tx, tenant, fleet, proposal_id)?;
        if row.candidate.candidate_sha256 != expected_candidate_sha256
            || row.candidate.revision_sha256 != expected_revision_sha256
        {
            return Err(ApprovalError::Denied);
        }
        if now_unix > row.candidate.expires_unix {
            return Err(ApprovalError::Expired);
        }
        if row.state == ApprovalState::Consumed {
            return Err(ApprovalError::Replay);
        }
        self.verify_signature(&row)?;
        tx.execute(
            "UPDATE architect_evolution_approvals
             SET state='CONSUMED',consumed_at=?4,release_id=?5
             WHERE tenant=?1 AND fleet=?2 AND proposal_id=?3",
            params![tenant, fleet, proposal_id, now_unix, release_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn snapshot(
        &self,
        tenant: &str,
        fleet: &str,
        proposal_id: &str,
    ) -> Result<ApprovalSnapshot, ApprovalError> {
        if !valid_id(tenant) || !valid_id(fleet) || !valid_id(proposal_id) {
            return Err(ApprovalError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ApprovalError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = Self::load(&tx, tenant, fleet, proposal_id)?;
        if row.state != ApprovalState::AwaitingOperator {
            self.verify_signature(&row)?;
        }
        let snapshot = ApprovalSnapshot {
            candidate: row.candidate,
            state: row.state,
            approved_at_unix: row.approved_at_unix,
            consumed_at_unix: row.consumed_at_unix,
            release_id: row.release_id,
        };
        tx.commit()?;
        Ok(snapshot)
    }
}
