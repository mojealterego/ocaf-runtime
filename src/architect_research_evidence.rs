//! Signed evidence ingress for the real OM Engine journal (AB-OM-01/06-08 partial).
//!
//! This is NOT a research crawler, source-fact checker, OMAt executor or
//! production release gate. It accepts signed and immutable bounded evidence
//! from externally authenticated role-specific workers, verifies OM Engine
//! fencing, then records a verifiable content digest as an OM Engine receipt.
//! The host must provision distinct signing keys, authenticate callers, control
//! time and retain an external audit anchor against SQLite rollback.

use crate::{
    architect_source_archive::{ArchiveError, SourceArchive},
    om_engine::{OmEngine, OmError},
    omnidev_workflow::{Completion, Outcome, Receipt, Role, StepStatus},
};
use openssl::{
    pkey::{Id, PKey, Public},
    sign::Verifier,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::Path, str::FromStr, sync::Mutex, time::Duration};
use thiserror::Error;
use url::Url;

const MAX_REPORT_BYTES: usize = 32 * 1024;
const MAX_SOURCES: usize = 32;
const MAX_SUMMARY_BYTES: usize = 8 * 1024;
const MAX_UNIX: u64 = 253_402_300_799;
const SOURCE_LOOKBACK_SECONDS: u64 = 31 * 86_400;
const SIGNING_PREFIX: &[u8] = b"OCAF-RESEARCH-EVIDENCE-v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    /// An unfetched citation pointer; not proof the URL is truthful or reachable.
    pub url: String,
    pub retrieved_unix: u64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchEvidence {
    pub schema_version: u16,
    pub tenant: String,
    pub workflow_id: String,
    pub step: String,
    pub role: Role,
    pub epoch: u64,
    pub attempt_id: String,
    pub observed_at_unix: u64,
    pub actual_cost_minor: u64,
    pub outcome: Outcome,
    pub summary: String,
    pub sources: Vec<SourceRecord>,
    /// Verifier's independent signature must bind the exact prior research
    /// digest observed in the OM Engine journal, not an unbound summary.
    pub review_of_sha256: Option<String>,
}

impl ResearchEvidence {
    /// Domain-separated deterministic bytes to be signed by an authorized
    /// role-specific Ed25519 private key. No secret enters the evidence gate.
    pub fn signing_message(&self) -> Result<Vec<u8>, EvidenceError> {
        let encoded = serde_json::to_vec(self)?;
        if encoded.is_empty() || encoded.len() > MAX_REPORT_BYTES {
            return Err(EvidenceError::Invalid);
        }
        let mut bytes = Vec::with_capacity(SIGNING_PREFIX.len() + 4 + encoded.len());
        bytes.extend_from_slice(SIGNING_PREFIX);
        bytes.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&encoded);
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence: ResearchEvidence,
    pub digest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmittedEvidence {
    pub digest_sha256: String,
    pub completion: Completion,
}

#[derive(Debug, Error)]
pub enum EvidenceError {
    #[error("invalid or excessive evidence data")]
    Invalid,
    #[error("authenticated tenant differs from evidence")]
    Denied,
    #[error("evidence does not match an active OM Engine lease")]
    LeaseNotActive,
    #[error("unknown evidence item")]
    NotFound,
    #[error("evidence conflicts with immutable prior record")]
    Conflict,
    #[error("Ed25519 worker signature is missing or invalid")]
    Signature,
    #[error("evidence JSON or digest is corrupt")]
    Integrity,
    #[error("evidence database lock poisoned")]
    Poisoned,
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error(transparent)]
    Engine(#[from] OmError),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub struct ResearchEvidenceGate {
    db: Mutex<Connection>,
    planner_key: PKey<Public>,
    architect_key: PKey<Public>,
    verifier_key: PKey<Public>,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

fn valid_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_source(source: &SourceRecord, observed_at: u64) -> bool {
    if source.url.is_empty()
        || source.url.len() > 2048
        || !valid_sha256(&source.content_sha256)
        || source.retrieved_unix > observed_at
        || observed_at.saturating_sub(source.retrieved_unix) > SOURCE_LOOKBACK_SECONDS
    {
        return false;
    }
    // The evidence gate never fetches sources; URL validation is for bounded
    // metadata. A downstream fetcher still needs independent SSRF/egress policy.
    let Ok(url) = Url::parse(&source.url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && host.contains('.')
        && !host.ends_with(".local")
        && !host.ends_with(".localhost")
        && !host.ends_with('.')
        && !host.eq_ignore_ascii_case("localhost")
        && std::net::IpAddr::from_str(host).is_err()
}

impl ResearchEvidenceGate {
    pub fn open(
        path: impl AsRef<Path>,
        pinned_planner_key: &[u8],
        pinned_architect_key: &[u8],
        pinned_verifier_key: &[u8],
    ) -> Result<Self, EvidenceError> {
        let path = path.as_ref();
        if path
            .symlink_metadata()
            .is_ok_and(|v| v.file_type().is_symlink())
            || [
                pinned_planner_key,
                pinned_architect_key,
                pinned_verifier_key,
            ]
            .iter()
            .any(|key| key.len() != 32)
            || pinned_planner_key == pinned_architect_key
            || pinned_planner_key == pinned_verifier_key
            || pinned_architect_key == pinned_verifier_key
        {
            return Err(EvidenceError::Invalid);
        }
        let make_key =
            |b| PKey::public_key_from_raw_bytes(b, Id::ED25519).map_err(|_| EvidenceError::Invalid);
        let planner_key = make_key(pinned_planner_key)?;
        let architect_key = make_key(pinned_architect_key)?;
        let verifier_key = make_key(pinned_verifier_key)?;
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS architect_research_evidence (
                 tenant TEXT NOT NULL, workflow_id TEXT NOT NULL,
                 step TEXT NOT NULL CHECK(step IN ('plan','research','verify')),
                 epoch INTEGER NOT NULL, attempt_id TEXT NOT NULL,
                 evidence_json TEXT NOT NULL,
                 evidence_sha256 TEXT NOT NULL,
                 signature BLOB NOT NULL,
                 PRIMARY KEY (tenant,workflow_id,step,epoch,attempt_id)
             );",
        )?;
        Ok(Self {
            db: Mutex::new(db),
            planner_key,
            architect_key,
            verifier_key,
        })
    }

    fn key(&self, role: Role) -> Result<&PKey<Public>, EvidenceError> {
        match role {
            Role::Planner => Ok(&self.planner_key),
            Role::Architect => Ok(&self.architect_key),
            Role::Verifier => Ok(&self.verifier_key),
            _ => Err(EvidenceError::Invalid),
        }
    }

    fn verify_signature(
        &self,
        evidence: &ResearchEvidence,
        signature: &[u8],
    ) -> Result<(), EvidenceError> {
        if signature.len() != 64 {
            return Err(EvidenceError::Signature);
        }
        let message = evidence.signing_message()?;
        let mut verifier = Verifier::new_without_digest(self.key(evidence.role)?)
            .map_err(|_| EvidenceError::Signature)?;
        if !verifier
            .verify_oneshot(signature, &message)
            .map_err(|_| EvidenceError::Signature)?
        {
            return Err(EvidenceError::Signature);
        }
        Ok(())
    }

    fn validate(&self, evidence: &ResearchEvidence, now: u64) -> Result<Vec<u8>, EvidenceError> {
        let valid_role = matches!(
            (evidence.step.as_str(), evidence.role),
            ("plan", Role::Planner) | ("research", Role::Architect) | ("verify", Role::Verifier)
        );
        if evidence.schema_version != 1
            || !valid_id(&evidence.tenant)
            || !valid_id(&evidence.workflow_id)
            || !valid_id(&evidence.attempt_id)
            || !valid_role
            || evidence.epoch == 0
            || now > MAX_UNIX
            || evidence.observed_at_unix > now
            || now.saturating_sub(evidence.observed_at_unix) > 3600
            || evidence.summary.trim().len() < 12
            || evidence.summary.len() > MAX_SUMMARY_BYTES
            || evidence.summary.contains('\0')
            || evidence.sources.len() > MAX_SOURCES
        {
            return Err(EvidenceError::Invalid);
        }
        if evidence.step == "research" {
            if evidence.sources.len() < 2 || evidence.review_of_sha256.is_some() {
                return Err(EvidenceError::Invalid);
            }
        } else if evidence.step == "verify" {
            if !evidence
                .review_of_sha256
                .as_deref()
                .is_some_and(valid_sha256)
            {
                return Err(EvidenceError::Invalid);
            }
        } else if evidence.review_of_sha256.is_some() {
            return Err(EvidenceError::Invalid);
        }
        let mut distinct_urls = HashSet::new();
        for source in &evidence.sources {
            if !validate_source(source, evidence.observed_at_unix)
                || !distinct_urls.insert(source.url.as_str())
            {
                return Err(EvidenceError::Invalid);
            }
        }
        let encoded = serde_json::to_vec(evidence)?;
        if encoded.len() > MAX_REPORT_BYTES {
            return Err(EvidenceError::Invalid);
        }
        Ok(encoded)
    }

    fn stored(
        &self,
        tenant: &str,
        workflow_id: &str,
        step: &str,
        epoch: u64,
        attempt_id: &str,
    ) -> Result<(ResearchEvidence, String, Vec<u8>), EvidenceError> {
        let db = self.db.lock().map_err(|_| EvidenceError::Poisoned)?;
        let row: Option<(String, String, Vec<u8>)> = db
            .query_row(
                "SELECT evidence_json,evidence_sha256,signature
             FROM architect_research_evidence
             WHERE tenant=?1 AND workflow_id=?2 AND step=?3 AND epoch=?4 AND attempt_id=?5",
                params![tenant, workflow_id, step, epoch, attempt_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (json, sha, sig) = row.ok_or(EvidenceError::NotFound)?;
        if json.len() > MAX_REPORT_BYTES || digest(json.as_bytes()) != sha {
            return Err(EvidenceError::Integrity);
        }
        let evidence: ResearchEvidence =
            serde_json::from_str(&json).map_err(|_| EvidenceError::Integrity)?;
        // The signature authenticates the deterministic serialized structure.
        // Stored bytes MUST equal that representation exactly; otherwise a
        // malicious DB administrator could rewrite whitespace/key order and
        // recompute an unrelated raw-JSON digest without a new signature.
        let canonical = serde_json::to_vec(&evidence).map_err(|_| EvidenceError::Integrity)?;
        if canonical.as_slice() != json.as_bytes() {
            return Err(EvidenceError::Integrity);
        }
        if evidence.tenant != tenant
            || evidence.workflow_id != workflow_id
            || evidence.step != step
            || evidence.epoch != epoch
            || evidence.attempt_id != attempt_id
        {
            return Err(EvidenceError::Integrity);
        }
        self.verify_signature(&evidence, &sig)
            .map_err(|_| EvidenceError::Integrity)?;
        Ok((evidence, sha, sig))
    }

    pub fn read(
        &self,
        tenant: &str,
        workflow_id: &str,
        step: &str,
        epoch: u64,
        attempt_id: &str,
    ) -> Result<EvidenceRecord, EvidenceError> {
        if !valid_id(tenant)
            || !valid_id(workflow_id)
            || !valid_id(attempt_id)
            || !matches!(step, "plan" | "research" | "verify")
            || epoch == 0
        {
            return Err(EvidenceError::Invalid);
        }
        let (evidence, digest_sha256, _) =
            self.stored(tenant, workflow_id, step, epoch, attempt_id)?;
        Ok(EvidenceRecord {
            evidence,
            digest_sha256,
        })
    }

    /// Strict operator-facing evidence ingress: verify that every signed
    /// source reference has an exact, readable archived body before accepting
    /// the signed OM Engine receipt. This does not independently authenticate
    /// the source server; the trusted fetcher and source archive custody are
    /// external requirements. The older submit() remains a legacy entrypoint,
    /// so production callers MUST enforce this stronger API boundary.
    pub fn submit_with_archive(
        &self,
        engine: &OmEngine,
        archive: &SourceArchive,
        trusted_tenant: &str,
        evidence: &ResearchEvidence,
        signature: &[u8],
        now_unix: u64,
    ) -> Result<SubmittedEvidence, EvidenceError> {
        if !valid_id(trusted_tenant) || evidence.tenant != trusted_tenant {
            return Err(EvidenceError::Denied);
        }
        self.validate(evidence, now_unix)?;
        self.verify_signature(evidence, signature)?;
        archive.verify_sources(trusted_tenant, &evidence.sources)?;
        self.submit(engine, trusted_tenant, evidence, signature, now_unix)
    }

    /// Stage signed, immutable evidence BEFORE recording the OM Engine event.
    /// A failed engine completion can leave an evidence row; replaying the
    /// identical signed report is safe, and a changed report cannot overwrite it.
    pub fn submit(
        &self,
        engine: &OmEngine,
        trusted_tenant: &str,
        evidence: &ResearchEvidence,
        signature: &[u8],
        now_unix: u64,
    ) -> Result<SubmittedEvidence, EvidenceError> {
        if !valid_id(trusted_tenant) || evidence.tenant != trusted_tenant {
            return Err(EvidenceError::Denied);
        }
        let encoded = self.validate(evidence, now_unix)?;
        self.verify_signature(evidence, signature)?;

        if evidence.step == "verify" {
            let status = engine.status(trusted_tenant, &evidence.workflow_id, "research")?;
            let StepStatus::Passed { evidence_sha256 } = status else {
                return Err(EvidenceError::LeaseNotActive);
            };
            if evidence.review_of_sha256.as_deref() != Some(evidence_sha256.as_str()) {
                return Err(EvidenceError::LeaseNotActive);
            }
        }
        let proof_sha = digest(&encoded);
        let receipt = Receipt {
            epoch: evidence.epoch,
            attempt_id: evidence.attempt_id.clone(),
            outcome: evidence.outcome,
            evidence_sha256: proof_sha.clone(),
            actual_cost_minor: evidence.actual_cost_minor,
        };
        let status = engine.status(trusted_tenant, &evidence.workflow_id, &evidence.step)?;
        let live = match status {
            StepStatus::Leased(ref claim) => {
                claim.epoch == evidence.epoch && claim.attempt_id == evidence.attempt_id
            }
            StepStatus::Passed {
                ref evidence_sha256,
            } => evidence_sha256 == &proof_sha,
            StepStatus::Ready | StepStatus::Halted => {
                engine.last_receipt(trusted_tenant, &evidence.workflow_id, &evidence.step)?
                    == Some(receipt.clone())
            }
        };
        if !live {
            return Err(EvidenceError::LeaseNotActive);
        }

        {
            let mut db = self.db.lock().map_err(|_| EvidenceError::Poisoned)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT OR IGNORE INTO architect_research_evidence
                 (tenant,workflow_id,step,epoch,attempt_id,evidence_json,evidence_sha256,signature)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    trusted_tenant,
                    evidence.workflow_id,
                    evidence.step,
                    evidence.epoch,
                    evidence.attempt_id,
                    std::str::from_utf8(&encoded).map_err(|_| EvidenceError::Invalid)?,
                    proof_sha,
                    signature
                ],
            )?;
            let stored: (String, String, Vec<u8>) = tx.query_row(
                "SELECT evidence_json,evidence_sha256,signature
                 FROM architect_research_evidence
                 WHERE tenant=?1 AND workflow_id=?2 AND step=?3 AND epoch=?4 AND attempt_id=?5",
                params![
                    trusted_tenant,
                    evidence.workflow_id,
                    evidence.step,
                    evidence.epoch,
                    evidence.attempt_id
                ],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            if stored.0.as_bytes() != encoded || stored.1 != proof_sha || stored.2 != signature {
                return Err(EvidenceError::Conflict);
            }
            tx.commit()?;
        }
        let completion = engine.complete(
            trusted_tenant,
            &evidence.workflow_id,
            evidence.role,
            &evidence.step,
            receipt,
            now_unix,
        )?;
        Ok(SubmittedEvidence {
            digest_sha256: proof_sha,
            completion,
        })
    }
}

#[cfg(test)]
mod reserved_domain_tests {
    use super::*;

    #[test]
    fn reserved_local_dns_suffixes_cannot_be_signed_as_public_sources() {
        for name in ["foo.localhost", "foo.local", "foo.localhost.", "localhost"] {
            let record = SourceRecord {
                url: format!("https://{name}/probe"),
                retrieved_unix: 1_795_480_000,
                content_sha256: "a".repeat(64),
            };
            assert!(!validate_source(&record, 1_795_480_010), "{name}");
        }
        let record = SourceRecord {
            url: "https://example.com/research".to_owned(),
            retrieved_unix: 1_795_480_000,
            content_sha256: "a".repeat(64),
        };
        assert!(validate_source(&record, 1_795_480_010));
    }
}
