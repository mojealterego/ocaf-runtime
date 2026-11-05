//! Durable PDF-1.27 / PDF-1.28 tool-intent and audit journal.
//! Source IDs are NOT the canonical Apex 1.27/1.28.
//! This is an additional admission ledger; it does NOT execute external tools.
//! PENDING requests MUST be reconciled with the external provider after failure;
//! never assume a timeout means the side effect did not occur.
//! The hash chain is tamper-evident only with an externally pinned AuditHead.
use crate::governance_contract::{
    ActionRequest, AgentContract, ApprovalAuthority, DenyReason, PreflightDecision, RiskTier,
};
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

const MAX_TENANT_AUDIT_ROWS: u64 = 32768;
const GENESIS: &str = "OCAF-AUDIT-GENESIS-v1";

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("invalid journal path or authenticated admission input")]
    Invalid,
    #[error("durable database unavailable")]
    Database,
    #[error("audit log corrupted, truncated or no longer matches pinned head")]
    Integrity,
    #[error("journal capacity exceeded")]
    Limit,
    #[error("external outcome or idempotency identity conflicts")]
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditHead {
    pub sequence: u64,
    pub digest: String,
}

impl Default for AuditHead {
    fn default() -> Self {
        Self {
            sequence: 0,
            digest: GENESIS.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// A durable intent exists; caller must still pass every real executor gate.
    FreshPending,
    /// Unknown external outcome. No automatic retry of the effect is allowed.
    ReplayPending,
    /// Existing result stored; a fresh effect is forbidden.
    ReplayCommitted,
    Denied(DenyReason),
    DeniedApprovalReplay,
    DeniedIdempotencyCollision,
}

pub struct GovernanceJournal {
    path: PathBuf,
}

fn db<T>(r: rusqlite::Result<T>) -> Result<T, JournalError> {
    r.map_err(|_| JournalError::Database)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

struct AuditDigest<'a> {
    tenant: &'a str,
    sequence: u64,
    actor: &'a str,
    action: &'a str,
    request: &'a str,
    decision: &'a str,
    reason: &'a str,
    payload_sha256: &'a [u8],
    previous: &'a str,
}

fn hash_row(row: AuditDigest<'_>) -> String {
    let mut h = Sha256::new();
    h.update(b"OCAF-POLICY-AUDIT-v1");
    for field in [
        row.tenant.as_bytes(),
        &row.sequence.to_be_bytes(),
        row.actor.as_bytes(),
        row.action.as_bytes(),
        row.request.as_bytes(),
        row.decision.as_bytes(),
        row.reason.as_bytes(),
        row.payload_sha256,
        row.previous.as_bytes(),
    ] {
        h.update((field.len() as u64).to_be_bytes());
        h.update(field);
    }
    format!("{:x}", h.finalize())
}

type StoredOutcome = (String, String, Vec<u8>, String, Option<Vec<u8>>);

impl GovernanceJournal {
    /// Database must be created by the operator under a private trusted
    /// directory; only a preexisting regular file is opened (NOFOLLOW).
    /// Never allow a caller to choose the path or a fresh database.
    pub fn open_existing(path: &Path) -> Result<Self, JournalError> {
        if !path.is_absolute()
            || !path.is_file()
            || path
                .symlink_metadata()
                .map_err(|_| JournalError::Invalid)?
                .file_type()
                .is_symlink()
        {
            return Err(JournalError::Invalid);
        }
        let this = Self { path: path.into() };
        let conn = this.connection()?;
        db(conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS governance_intents_v1 (
                tenant TEXT NOT NULL,
                action TEXT NOT NULL,
                idem TEXT NOT NULL,
                actor TEXT NOT NULL,
                request_id TEXT NOT NULL,
                payload_sha256 BLOB NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('PENDING','COMMITTED')),
                outcome_sha256 BLOB,
                PRIMARY KEY (tenant, action, idem)
             );
             CREATE TABLE IF NOT EXISTS governance_approvals_v1 (
                tenant TEXT NOT NULL,
                approval_id TEXT NOT NULL,
                request_id TEXT NOT NULL,
                PRIMARY KEY (tenant, approval_id)
             );
             CREATE TABLE IF NOT EXISTS governance_audit_v1 (
                tenant TEXT NOT NULL,
                seq INTEGER NOT NULL,
                actor TEXT NOT NULL,
                action TEXT NOT NULL,
                request_id TEXT NOT NULL,
                decision TEXT NOT NULL,
                reason TEXT NOT NULL,
                payload_sha256 BLOB NOT NULL,
                previous TEXT NOT NULL,
                digest TEXT NOT NULL,
                PRIMARY KEY (tenant, seq)
             );",
        ))?;
        Ok(this)
    }

    fn connection(&self) -> Result<Connection, JournalError> {
        let conn = db(Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        ))?;
        db(conn.busy_timeout(Duration::from_secs(5)))?;
        db(conn.execute_batch("PRAGMA synchronous=FULL; PRAGMA journal_mode=DELETE;"))?;
        Ok(conn)
    }

    /// Reads the entire tenant audit chain and verifies every digest.
    /// Pin the returned head OUTSIDE this database to detect truncation.
    pub fn inspect(
        &self,
        tenant: &str,
        expected: Option<&AuditHead>,
    ) -> Result<AuditHead, JournalError> {
        if !valid_id(tenant) {
            return Err(JournalError::Invalid);
        }
        let mut conn = self.connection()?;
        let tx = db(conn.transaction())?;
        let head = scan_audit(&tx, tenant)?;
        if expected.is_some_and(|pinned| pinned != &head) {
            return Err(JournalError::Integrity);
        }
        Ok(head)
    }

    /// The trusted tenant and actor must originate from mTLS/OMAt, never JSON.
    /// A policy DENY is persisted before returning it, without creating intent.
    /// A fresh ALLOW and its idempotency intent commit atomically.
    pub fn begin(
        &self,
        contract: &AgentContract,
        request: &ActionRequest<'_>,
        approval_authority: Option<&ApprovalAuthority>,
        trusted_peer: (&str, &str),
        now_unix: u64,
        idempotency_key: &str,
    ) -> Result<Admission, JournalError> {
        let (trusted_tenant, trusted_actor) = trusted_peer;
        if !valid_id(trusted_tenant)
            || !valid_id(trusted_actor)
            || !valid_id(idempotency_key)
            || !valid_id(request.request_id)
            || !valid_id(request.action)
            || request.payload_sha256 == [0; 32]
        {
            return Err(JournalError::Invalid);
        }
        let mut conn = self.connection()?;
        let tx = db(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let head = scan_audit(&tx, trusted_tenant)?;
        if head.sequence >= MAX_TENANT_AUDIT_ROWS {
            return Err(JournalError::Limit);
        }
        let decision = if request.tenant == trusted_tenant && request.actor == trusted_actor {
            contract.evaluate(request, approval_authority, now_unix)
        } else {
            PreflightDecision::Deny(DenyReason::IdentityMismatch)
        };
        if let PreflightDecision::Deny(reason) = decision {
            append_audit(
                &tx,
                &head,
                trusted_tenant,
                trusted_actor,
                request,
                "DENY",
                &format!("{reason:?}"),
            )?;
            db(tx.commit())?;
            return Ok(Admission::Denied(reason));
        }
        let prior: Option<(String, String, Vec<u8>, String, Option<Vec<u8>>)> = db(tx
            .query_row(
                "SELECT actor,request_id,payload_sha256,status,outcome_sha256
             FROM governance_intents_v1
             WHERE tenant=?1 AND action=?2 AND idem=?3",
                params![trusted_tenant, request.action, idempotency_key],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional())?;
        if let Some((actor, request_id, hash, status, receipt)) = prior {
            let stored_state_valid = match (status.as_str(), receipt.as_deref()) {
                ("PENDING", None) => true,
                ("COMMITTED", Some(bytes)) => {
                    bytes.len() == 32 && bytes.iter().any(|byte| *byte != 0)
                }
                _ => false,
            };
            if hash.len() != 32
                || hash.iter().all(|byte| *byte == 0)
                || !stored_state_valid
                || !valid_id(&actor)
                || !valid_id(&request_id)
            {
                return Err(JournalError::Integrity);
            }
            if actor != trusted_actor
                || request_id != request.request_id
                || hash.as_slice() != request.payload_sha256.as_slice()
            {
                append_audit(
                    &tx,
                    &head,
                    trusted_tenant,
                    trusted_actor,
                    request,
                    "DENY",
                    "IDEMPOTENCY_COLLISION",
                )?;
                db(tx.commit())?;
                return Ok(Admission::DeniedIdempotencyCollision);
            }
            let outcome = match (status.as_str(), receipt.as_deref()) {
                ("PENDING", None) => Admission::ReplayPending,
                ("COMMITTED", Some(hash))
                    if hash.len() == 32 && hash.iter().any(|byte| *byte != 0) =>
                {
                    Admission::ReplayCommitted
                }
                _ => return Err(JournalError::Integrity),
            };
            append_audit(
                &tx,
                &head,
                trusted_tenant,
                trusted_actor,
                request,
                "REPLAY",
                &format!("{outcome:?}"),
            )?;
            db(tx.commit())?;
            return Ok(outcome);
        }
        if request.risk >= RiskTier::Medium {
            let approval = request.approval.ok_or(JournalError::Integrity)?;
            let prior_approval: Option<String> = db(tx
                .query_row(
                    "SELECT request_id FROM governance_approvals_v1
                 WHERE tenant=?1 AND approval_id=?2",
                    params![trusted_tenant, approval.approval_id],
                    |row| row.get(0),
                )
                .optional())?;
            if prior_approval.is_some() {
                append_audit(
                    &tx,
                    &head,
                    trusted_tenant,
                    trusted_actor,
                    request,
                    "DENY",
                    "APPROVAL_ALREADY_CONSUMED",
                )?;
                db(tx.commit())?;
                return Ok(Admission::DeniedApprovalReplay);
            }
            db(tx.execute(
                "INSERT INTO governance_approvals_v1 VALUES (?1,?2,?3)",
                params![trusted_tenant, approval.approval_id, request.request_id],
            ))?;
        }
        db(tx.execute(
            "INSERT INTO governance_intents_v1
             (tenant,action,idem,actor,request_id,payload_sha256,status,outcome_sha256)
             VALUES (?1,?2,?3,?4,?5,?6,'PENDING',NULL)",
            params![
                trusted_tenant,
                request.action,
                idempotency_key,
                trusted_actor,
                request.request_id,
                request.payload_sha256.as_slice()
            ],
        ))?;
        append_audit(
            &tx,
            &head,
            trusted_tenant,
            trusted_actor,
            request,
            "ALLOW_INTENT",
            "PENDING_EXTERNAL_GATE",
        )?;
        db(tx.commit())?;
        Ok(Admission::FreshPending)
    }

    /// Only record an independently verified external outcome receipt.
    /// The caller remains responsible for proving the external effect.
    pub fn record_verified_outcome(
        &self,
        trusted_tenant: &str,
        trusted_actor: &str,
        request: &ActionRequest<'_>,
        idempotency_key: &str,
        outcome_sha256: [u8; 32],
    ) -> Result<bool, JournalError> {
        if !valid_id(trusted_tenant)
            || !valid_id(trusted_actor)
            || !valid_id(idempotency_key)
            || !valid_id(request.request_id)
            || !valid_id(request.action)
            || outcome_sha256 == [0; 32]
            || request.payload_sha256 == [0; 32]
            || trusted_tenant != request.tenant
            || trusted_actor != request.actor
        {
            return Err(JournalError::Invalid);
        }
        let mut conn = self.connection()?;
        let tx = db(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let head = scan_audit(&tx, trusted_tenant)?;
        let existing: Option<StoredOutcome> = db(tx
            .query_row(
                "SELECT actor,request_id,payload_sha256,status,outcome_sha256
             FROM governance_intents_v1
             WHERE tenant=?1 AND action=?2 AND idem=?3",
                params![trusted_tenant, request.action, idempotency_key],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional())?;
        let Some((actor, request_id, hash, phase, existing_outcome)) = existing else {
            return Err(JournalError::Conflict);
        };
        if hash.len() != 32
            || hash.iter().all(|byte| *byte == 0)
            || !valid_id(&actor)
            || !valid_id(&request_id)
        {
            return Err(JournalError::Integrity);
        }
        if actor != trusted_actor
            || request_id != request.request_id
            || hash.as_slice() != request.payload_sha256.as_slice()
        {
            return Err(JournalError::Conflict);
        }
        if phase == "COMMITTED" {
            if existing_outcome.as_ref().is_none_or(|stored| {
                stored.len() != 32 || stored.iter().all(|byte| *byte == 0)
            }) {
                return Err(JournalError::Integrity);
            }
            if existing_outcome.as_deref() == Some(outcome_sha256.as_slice()) {
                return Ok(false);
            }
            return Err(JournalError::Conflict);
        }
        if phase != "PENDING" || existing_outcome.is_some() {
            return Err(JournalError::Integrity);
        }
        // A replay of an already committed receipt is read-only even at the audit cap.
        // A new transition must still reserve capacity for its audit entry.
        if head.sequence >= MAX_TENANT_AUDIT_ROWS {
            return Err(JournalError::Limit);
        }
        let updated = db(tx.execute(
            "UPDATE governance_intents_v1
             SET status='COMMITTED', outcome_sha256=?4
             WHERE tenant=?1 AND action=?2 AND idem=?3 AND status='PENDING'",
            params![
                trusted_tenant,
                request.action,
                idempotency_key,
                outcome_sha256.as_slice()
            ],
        ))?;
        if updated != 1 {
            return Err(JournalError::Integrity);
        }
        append_audit(
            &tx,
            &head,
            trusted_tenant,
            trusted_actor,
            request,
            "OUTCOME_RECEIPT",
            "EXTERNALLY_VERIFIED_BY_CALLER",
        )?;
        db(tx.commit())?;
        Ok(true)
    }
}

fn scan_audit(tx: &Transaction<'_>, tenant: &str) -> Result<AuditHead, JournalError> {
    let mut stmt = db(tx.prepare(
        "SELECT seq,actor,action,request_id,decision,reason,payload_sha256,previous,digest
         FROM governance_audit_v1 WHERE tenant=?1 ORDER BY seq",
    ))?;
    let mut rows = db(stmt.query(params![tenant]))?;
    let mut head = AuditHead::default();
    while let Some(row) = db(rows.next())? {
        let seq: u64 = row.get(0).map_err(|_| JournalError::Integrity)?;
        let actor: String = row.get(1).map_err(|_| JournalError::Integrity)?;
        let action: String = row.get(2).map_err(|_| JournalError::Integrity)?;
        let request: String = row.get(3).map_err(|_| JournalError::Integrity)?;
        let decision: String = row.get(4).map_err(|_| JournalError::Integrity)?;
        let reason: String = row.get(5).map_err(|_| JournalError::Integrity)?;
        let payload: Vec<u8> = row.get(6).map_err(|_| JournalError::Integrity)?;
        let previous: String = row.get(7).map_err(|_| JournalError::Integrity)?;
        let hash: String = row.get(8).map_err(|_| JournalError::Integrity)?;
        if seq != head.sequence + 1
            || !matches!(
                decision.as_str(),
                "DENY" | "REPLAY" | "ALLOW_INTENT" | "OUTCOME_RECEIPT"
            )
            || seq > MAX_TENANT_AUDIT_ROWS
            || payload.len() != 32
            || payload.iter().all(|byte| *byte == 0)
            || !valid_id(&actor)
            || !valid_id(&action)
            || !valid_id(&request)
            || (decision == "ALLOW_INTENT" && reason != "PENDING_EXTERNAL_GATE")
            || (decision == "OUTCOME_RECEIPT" && reason != "EXTERNALLY_VERIFIED_BY_CALLER")
            || previous != head.digest
            || hash
                != hash_row(AuditDigest {
                    tenant,
                    sequence: seq,
                    actor: &actor,
                    action: &action,
                    request: &request,
                    decision: &decision,
                    reason: &reason,
                    payload_sha256: &payload,
                    previous: &previous,
                })
        {
            return Err(JournalError::Integrity);
        }
        head = AuditHead {
            sequence: seq,
            digest: hash,
        };
    }
    Ok(head)
}

fn append_audit(
    tx: &Transaction<'_>,
    head: &AuditHead,
    tenant: &str,
    actor: &str,
    request: &ActionRequest<'_>,
    decision: &str,
    reason: &str,
) -> Result<(), JournalError> {
    let seq = head.sequence.checked_add(1).ok_or(JournalError::Limit)?;
    if seq > MAX_TENANT_AUDIT_ROWS {
        return Err(JournalError::Limit);
    }
    let hash = hash_row(AuditDigest {
        tenant,
        sequence: seq,
        actor,
        action: request.action,
        request: request.request_id,
        decision,
        reason,
        payload_sha256: &request.payload_sha256,
        previous: &head.digest,
    });
    db(tx.execute(
        "INSERT INTO governance_audit_v1
         (tenant,seq,actor,action,request_id,decision,reason,payload_sha256,previous,digest)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            tenant,
            seq,
            actor,
            request.action,
            request.request_id,
            decision,
            reason,
            request.payload_sha256.as_slice(),
            head.digest,
            hash
        ],
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance_contract::{
        payload_sha256, Accountability, Domain, ProviderAccess, SocialCommitment,
    };
    use std::collections::BTreeSet;

    fn contract() -> AgentContract {
        AgentContract {
            contract_id: "c1".into(),
            version: "v1".into(),
            tenant: "tenant_a".into(),
            actor: "agent_a".into(),
            obligations: vec![SocialCommitment {
                debtor: "agent_a".into(),
                creditor: "user".into(),
                condition: "Verify".into(),
            }],
            permissions: BTreeSet::from(["read".into()]),
            prohibitions: BTreeSet::new(),
            accountability: Accountability {
                owner: "owner".into(),
                escalation_reviewer: "reviewer".into(),
                audit_authority: "auditor".into(),
            },
        }
    }

    fn request<'a>() -> ActionRequest<'a> {
        ActionRequest {
            request_id: "req_1",
            tenant: "tenant_a",
            actor: "agent_a",
            action: "read",
            domain: Domain::CoreRetrieval,
            risk: RiskTier::Low,
            payload_sha256: payload_sha256(b"payload"),
            uses_external_provider: false,
            provider: None::<&'a ProviderAccess>,
            approval: None,
        }
    }

    fn setup() -> (tempfile::NamedTempFile, GovernanceJournal) {
        let file = tempfile::NamedTempFile::new().unwrap();
        let journal = GovernanceJournal::open_existing(file.path()).unwrap();
        (file, journal)
    }

    #[test]
    fn deny_is_audited_and_never_reserves_intent() {
        let (_file, journal) = setup();
        let c = contract();
        let mut r = request();
        r.action = "missing";
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "key_1")
                .unwrap(),
            Admission::Denied(DenyReason::NotAllowlisted)
        );
        assert_eq!(journal.inspect("tenant_a", None).unwrap().sequence, 1);
        r.action = "read";
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "key_1")
                .unwrap(),
            Admission::FreshPending
        );
    }

    #[test]
    fn restart_replay_is_pending_not_double_execution() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        let first = journal
            .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "idem")
            .unwrap();
        assert_eq!(first, Admission::FreshPending);
        let head = journal.inspect("tenant_a", None).unwrap();
        let reopened = GovernanceJournal::open_existing(file.path()).unwrap();
        assert_eq!(
            reopened
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "idem")
                .unwrap(),
            Admission::ReplayPending
        );
        assert!(matches!(
            reopened.inspect("tenant_a", Some(&head)),
            Err(JournalError::Integrity)
        ));
        assert_eq!(reopened.inspect("tenant_a", None).unwrap().sequence, 2);
    }

    #[test]
    fn idempotency_collision_is_denied_and_does_not_replace_pending() {
        let (_file, journal) = setup();
        let c = contract();
        let mut r = request();
        journal
            .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "idem")
            .unwrap();
        r.payload_sha256 = payload_sha256(b"different");
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "idem")
                .unwrap(),
            Admission::DeniedIdempotencyCollision
        );
        r.payload_sha256 = payload_sha256(b"payload");
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "idem")
                .unwrap(),
            Admission::ReplayPending
        );
    }

    #[test]
    fn invalid_receipt_request_identifiers_do_not_touch_audit() {
        let (_file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "invalid_receipt")
                .unwrap(),
            Admission::FreshPending
        );
        let pinned = journal.inspect("tenant_a", None).unwrap();
        for (action, request_id) in [
            ("read/private", "req_1"),
            ("read", "invalid/request"),
            ("", "req_1"),
            ("read", ""),
        ] {
            let mut malformed = request();
            malformed.action = action;
            malformed.request_id = request_id;
            assert!(matches!(
                journal.record_verified_outcome(
                    "tenant_a",
                    "agent_a",
                    &malformed,
                    "invalid_receipt",
                    payload_sha256(b"synthetic-receipt"),
                ),
                Err(JournalError::Invalid)
            ));
            assert_eq!(journal.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
        }
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "invalid_receipt")
                .unwrap(),
            Admission::ReplayPending
        );
    }

    #[test]
    fn zero_receipt_digest_cannot_commit_pending_intent() {
        let (_file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "zero_receipt")
                .unwrap(),
            Admission::FreshPending
        );
        assert!(matches!(
            journal.record_verified_outcome("tenant_a", "agent_a", &r, "zero_receipt", [0; 32]),
            Err(JournalError::Invalid)
        ));
        assert_eq!(journal.inspect("tenant_a", None).unwrap().sequence, 1);
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "zero_receipt")
                .unwrap(),
            Admission::ReplayPending
        );
    }

    #[test]
    fn forged_receipt_cannot_change_pending_intent_or_audit_chain() {
        let (_file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "receipt_guard")
                .unwrap(),
            Admission::FreshPending
        );
        let initial = journal.inspect("tenant_a", None).unwrap();
        let receipt = payload_sha256(b"synthetic-receipt");
        let mut forged = request();
        forged.payload_sha256 = payload_sha256(b"substituted-request");
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &forged,
                "receipt_guard",
                receipt,
            ),
            Err(JournalError::Conflict)
        ));
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "other_actor",
                &r,
                "receipt_guard",
                receipt,
            ),
            Err(JournalError::Invalid)
        ));
        assert!(matches!(
            journal.record_verified_outcome("tenant_a", "agent_a", &r, "unknown_key", receipt,),
            Err(JournalError::Conflict)
        ));
        assert_eq!(
            journal.inspect("tenant_a", Some(&initial)).unwrap(),
            initial
        );
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "receipt_guard")
                .unwrap(),
            Admission::ReplayPending
        );
    }

    #[test]
    fn receipt_committed_once_and_cross_tenant_receipt_denied() {
        let (_file, journal) = setup();
        let c = contract();
        let r = request();
        journal
            .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "k")
            .unwrap();
        let digest = payload_sha256(b"external receipt");
        assert!(matches!(
            journal.record_verified_outcome("tenant_b", "agent_a", &r, "k", digest),
            Err(JournalError::Invalid)
        ));
        assert!(journal
            .record_verified_outcome("tenant_a", "agent_a", &r, "k", digest)
            .unwrap());
        assert!(!journal
            .record_verified_outcome("tenant_a", "agent_a", &r, "k", digest)
            .unwrap());
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "k")
                .unwrap(),
            Admission::ReplayCommitted
        );
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &r,
                "k",
                payload_sha256(b"other")
            ),
            Err(JournalError::Conflict)
        ));
    }

    #[test]
    fn committed_receipt_is_idempotent_across_database_restart() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "restart_receipt")
                .unwrap(),
            Admission::FreshPending
        );
        let receipt = payload_sha256(b"synthetic-restart-receipt");
        assert!(journal
            .record_verified_outcome("tenant_a", "agent_a", &r, "restart_receipt", receipt)
            .unwrap());
        let pinned = journal.inspect("tenant_a", None).unwrap();
        assert_eq!(pinned.sequence, 2);
        drop(journal);

        let reopened = GovernanceJournal::open_existing(file.path()).unwrap();
        assert!(!reopened
            .record_verified_outcome("tenant_a", "agent_a", &r, "restart_receipt", receipt)
            .unwrap());
        assert_eq!(reopened.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
        assert!(matches!(
            reopened.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &r,
                "restart_receipt",
                payload_sha256(b"conflicting-receipt"),
            ),
            Err(JournalError::Conflict)
        ));
        assert_eq!(reopened.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
        assert_eq!(
            reopened
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "restart_receipt")
                .unwrap(),
            Admission::ReplayCommitted
        );
    }

    #[test]
    fn modified_hash_chain_and_head_truncation_are_rejected() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        journal
            .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "k")
            .unwrap();
        let trusted_head = journal.inspect("tenant_a", None).unwrap();
        let conn = Connection::open(file.path()).unwrap();
        conn.execute("UPDATE governance_audit_v1 SET reason='tampered'", [])
            .unwrap();
        assert!(matches!(
            journal.inspect("tenant_a", None),
            Err(JournalError::Integrity)
        ));
        assert!(matches!(
            journal.begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "k"),
            Err(JournalError::Integrity)
        ));
        conn.execute("DELETE FROM governance_audit_v1", []).unwrap();
        assert!(matches!(
            journal.inspect("tenant_a", Some(&trusted_head)),
            Err(JournalError::Integrity)
        ));
    }

    #[test]
    fn audit_scan_rejects_rehashed_noncanonical_intent_reason() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "reason_semantic")
                .unwrap(),
            Admission::FreshPending
        );
        let bad_reason = "UNVERIFIED_EXTERNAL_EFFECT";
        let digest = hash_row(AuditDigest {
            tenant: "tenant_a",
            sequence: 1,
            actor: "agent_a",
            action: "read",
            request: "req_1",
            decision: "ALLOW_INTENT",
            reason: bad_reason,
            payload_sha256: &r.payload_sha256,
            previous: GENESIS,
        });
        let conn = Connection::open(file.path()).unwrap();
        conn.execute(
            "UPDATE governance_audit_v1 SET reason=?1, digest=?2 WHERE tenant='tenant_a' AND seq=1",
            params![bad_reason, digest],
        )
        .unwrap();
        assert!(matches!(
            journal.inspect("tenant_a", None),
            Err(JournalError::Integrity)
        ));
    }

    #[test]
    fn audit_scan_rejects_semantically_invalid_but_rehashed_row() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "audit_semantic")
                .unwrap(),
            Admission::FreshPending
        );
        let conn = Connection::open(file.path()).unwrap();
        let bad_actor = "invalid/actor";
        let digest = hash_row(AuditDigest {
            tenant: "tenant_a",
            sequence: 1,
            actor: bad_actor,
            action: "read",
            request: "req_1",
            decision: "ALLOW_INTENT",
            reason: "PENDING_EXTERNAL_GATE",
            payload_sha256: &r.payload_sha256,
            previous: GENESIS,
        });
        conn.execute(
            "UPDATE governance_audit_v1 SET actor=?1, digest=?2 WHERE tenant='tenant_a' AND seq=1",
            params![bad_actor, digest],
        )
        .unwrap();
        assert!(matches!(
            journal.inspect("tenant_a", None),
            Err(JournalError::Integrity)
        ));
    }

    #[test]
    fn zero_request_digest_is_rejected_before_database_mutation() {
        let (_file, journal) = setup();
        let c = contract();
        let mut r = request();
        r.payload_sha256 = [0; 32];
        assert!(matches!(
            journal.begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "zero_request"),
            Err(JournalError::Invalid)
        ));
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &r,
                "zero_request",
                payload_sha256(b"synthetic-receipt"),
            ),
            Err(JournalError::Invalid)
        ));
        assert_eq!(journal.inspect("tenant_a", None).unwrap().sequence, 0);
    }

    #[test]
    fn zeroed_persisted_intent_digest_blocks_replay_and_commit() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "zero_intent")
                .unwrap(),
            Admission::FreshPending
        );
        let pinned = journal.inspect("tenant_a", None).unwrap();
        let conn = Connection::open(file.path()).unwrap();
        conn.execute(
            "UPDATE governance_intents_v1 SET payload_sha256=zeroblob(32) WHERE idem='zero_intent'",
            [],
        )
        .unwrap();
        assert!(matches!(
            journal.begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "zero_intent"),
            Err(JournalError::Integrity)
        ));
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &r,
                "zero_intent",
                payload_sha256(b"synthetic-receipt"),
            ),
            Err(JournalError::Integrity)
        ));
        assert_eq!(journal.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
    }

    #[test]
    fn malformed_persisted_identities_fail_closed_during_outcome_commit() {
        for (column, bad_value) in [("actor", "bad/actor"), ("request_id", "bad/request")] {
            let (file, journal) = setup();
            let c = contract();
            let r = request();
            assert_eq!(
                journal
                    .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "bad_commit_id")
                    .unwrap(),
                Admission::FreshPending
            );
            let pinned = journal.inspect("tenant_a", None).unwrap();
            let conn = Connection::open(file.path()).unwrap();
            conn.execute(
                &format!(
                    "UPDATE governance_intents_v1 SET {column}=?1 WHERE idem='bad_commit_id'"
                ),
                params![bad_value],
            )
            .unwrap();
            assert!(matches!(
                journal.record_verified_outcome(
                    "tenant_a",
                    "agent_a",
                    &r,
                    "bad_commit_id",
                    payload_sha256(b"synthetic-receipt"),
                ),
                Err(JournalError::Integrity)
            ));
            assert_eq!(journal.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
        }
    }

    #[test]
    fn malformed_persisted_actor_and_request_ids_fail_closed_on_replay() {
        for (column, bad_value) in [("actor", "bad/actor"), ("request_id", "bad/request")] {
            let (file, journal) = setup();
            let c = contract();
            let r = request();
            assert_eq!(
                journal
                    .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "bad_identity")
                    .unwrap(),
                Admission::FreshPending
            );
            let pinned = journal.inspect("tenant_a", None).unwrap();
            let conn = Connection::open(file.path()).unwrap();
            conn.execute(
                &format!("UPDATE governance_intents_v1 SET {column}=?1 WHERE idem='bad_identity'"),
                params![bad_value],
            )
            .unwrap();
            assert!(matches!(
                journal.begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "bad_identity"),
                Err(JournalError::Integrity)
            ));
            assert_eq!(journal.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
        }
    }

    #[test]
    fn corrupt_committed_state_is_rejected_before_idempotency_collision_audit() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "corrupt_collision")
                .unwrap(),
            Admission::FreshPending
        );
        let conn = Connection::open(file.path()).unwrap();
        conn.execute(
            "UPDATE governance_intents_v1 SET status='COMMITTED', outcome_sha256=NULL WHERE idem='corrupt_collision'",
            [],
        )
        .unwrap();
        let pinned = journal.inspect("tenant_a", None).unwrap();
        let mut conflicting = request();
        conflicting.payload_sha256 = payload_sha256(b"different");
        assert!(matches!(
            journal.begin(
                &c,
                &conflicting,
                None,
                ("tenant_a", "agent_a"),
                170,
                "corrupt_collision"
            ),
            Err(JournalError::Integrity)
        ));
        assert_eq!(journal.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
    }

    #[test]
    fn replay_rejects_corrupted_persisted_intent_and_receipt() {
        for (idem, column, corrupted) in [
            ("replay_bad_payload", "payload_sha256", "x'AB'"),
            ("replay_zero_receipt", "outcome_sha256", "zeroblob(32)"),
        ] {
            let (file, journal) = setup();
            let c = contract();
            let r = request();
            assert_eq!(
                journal
                    .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, idem)
                    .unwrap(),
                Admission::FreshPending
            );
            if column == "outcome_sha256" {
                assert!(journal
                    .record_verified_outcome(
                        "tenant_a",
                        "agent_a",
                        &r,
                        idem,
                        payload_sha256(b"synthetic-receipt"),
                    )
                    .unwrap());
            }
            let pinned = journal.inspect("tenant_a", None).unwrap();
            let conn = Connection::open(file.path()).unwrap();
            conn.execute(
                &format!(
                    "UPDATE governance_intents_v1 SET {column}={corrupted} WHERE idem=?1"
                ),
                params![idem],
            )
            .unwrap();
            assert!(matches!(
                journal.begin(&c, &r, None, ("tenant_a", "agent_a"), 170, idem),
                Err(JournalError::Integrity)
            ));
            assert_eq!(journal.inspect("tenant_a", Some(&pinned)).unwrap(), pinned);
        }
    }

    #[test]
    fn malformed_stored_intent_payload_hash_is_integrity_failure() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "bad_payload_hash")
                .unwrap(),
            Admission::FreshPending
        );
        let conn = Connection::open(file.path()).unwrap();
        conn.execute(
            "UPDATE governance_intents_v1 SET payload_sha256=x'AB' WHERE idem='bad_payload_hash'",
            [],
        )
        .unwrap();
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &r,
                "bad_payload_hash",
                payload_sha256(b"synthetic-receipt"),
            ),
            Err(JournalError::Integrity)
        ));
        assert_eq!(journal.inspect("tenant_a", None).unwrap().sequence, 1);
    }

    #[test]
    fn zeroed_persisted_receipt_digest_is_integrity_failure() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "zeroed_digest")
                .unwrap(),
            Admission::FreshPending
        );
        let receipt = payload_sha256(b"synthetic-receipt");
        assert!(journal
            .record_verified_outcome("tenant_a", "agent_a", &r, "zeroed_digest", receipt)
            .unwrap());
        let conn = Connection::open(file.path()).unwrap();
        conn.execute(
            "UPDATE governance_intents_v1 SET outcome_sha256=zeroblob(32) WHERE idem='zeroed_digest'",
            [],
        )
        .unwrap();
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &r,
                "zeroed_digest",
                receipt,
            ),
            Err(JournalError::Integrity)
        ));
    }

    #[test]
    fn malformed_persisted_receipt_digest_is_integrity_failure() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "malformed_digest")
                .unwrap(),
            Admission::FreshPending
        );
        let receipt = payload_sha256(b"synthetic-receipt");
        assert!(journal
            .record_verified_outcome("tenant_a", "agent_a", &r, "malformed_digest", receipt)
            .unwrap());
        let conn = Connection::open(file.path()).unwrap();
        conn.execute(
            "UPDATE governance_intents_v1 SET outcome_sha256=x'AB' WHERE idem='malformed_digest'",
            [],
        )
        .unwrap();
        assert!(matches!(
            journal.record_verified_outcome(
                "tenant_a",
                "agent_a",
                &r,
                "malformed_digest",
                receipt,
            ),
            Err(JournalError::Integrity)
        ));
    }

    #[test]
    fn corrupted_committed_receipt_is_not_replayed() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "receipt")
                .unwrap(),
            Admission::FreshPending
        );
        let db = Connection::open(file.path()).unwrap();
        db.execute(
            "UPDATE governance_intents_v1 SET status='COMMITTED', outcome_sha256=NULL WHERE idem='receipt'",
            [],
        ).unwrap();
        assert!(matches!(
            journal.begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "receipt"),
            Err(JournalError::Integrity)
        ));
    }
    #[test]
    fn hash_valid_unknown_audit_decision_is_rejected() {
        let (file, journal) = setup();
        let c = contract();
        let r = request();
        journal
            .begin(&c, &r, None, ("tenant_a", "agent_a"), 170, "audit")
            .unwrap();
        let forged = hash_row(AuditDigest {
            tenant: "tenant_a",
            sequence: 1,
            actor: "agent_a",
            action: "read",
            request: "req_1",
            decision: "EXECUTE_ANYWAY",
            reason: "PENDING_EXTERNAL_GATE",
            payload_sha256: &r.payload_sha256,
            previous: GENESIS,
        });
        let conn = Connection::open(file.path()).unwrap();
        conn.execute(
            "UPDATE governance_audit_v1 SET decision='EXECUTE_ANYWAY', digest=?1 WHERE tenant='tenant_a'",
            params![forged],
        ).unwrap();
        assert!(matches!(
            journal.inspect("tenant_a", None),
            Err(JournalError::Integrity)
        ));
    }

    #[test]
    fn peer_is_derived_from_trusted_context_not_requested_identity() {
        let (_file, journal) = setup();
        let c = contract();
        let r = request();
        assert_eq!(
            journal
                .begin(&c, &r, None, ("tenant_b", "agent_a"), 170, "k")
                .unwrap(),
            Admission::Denied(DenyReason::IdentityMismatch)
        );
        assert_eq!(journal.inspect("tenant_a", None).unwrap().sequence, 0);
        assert_eq!(journal.inspect("tenant_b", None).unwrap().sequence, 1);
    }
}
