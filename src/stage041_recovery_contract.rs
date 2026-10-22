//! Stage 0.41 recovery and chaos acceptance contract.
//!
//! This module records the evidence boundary for RTO/RPO, backup restore and
//! power-loss recovery. It does not claim that a local process or a copied
//! file is an independent host. Such evidence is rejected unless failure
//! domains and restore verification are explicitly recorded.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const STAGE_ID: &str = "0.41";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryTarget {
    pub rto_ms: u64,
    pub rpo_bytes: u64,
    pub required_failure_domains: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupReceipt {
    pub backup_id: String,
    pub source_lsn: u64,
    pub snapshot_sha256: String,
    pub source_failure_domain: String,
    pub backup_failure_domain: String,
    pub restore_test_id: String,
    pub restore_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEvidence {
    pub source_commit: String,
    pub test_command: String,
    pub artifact_digest: String,
    pub receipt_id: String,
    pub backup: BackupReceipt,
    pub wal_replay_verified: bool,
    pub fencing_verified: bool,
    pub power_loss_recovered: bool,
    pub observed_rto_ms: u64,
    pub observed_rpo_bytes: u64,
    pub failure_domain_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryReceipt {
    pub stage_id: String,
    pub receipt_id: String,
    pub accepted: bool,
    pub decision: String,
    pub source_commit: String,
    pub artifact_digest: String,
    pub backup_id: String,
    pub snapshot_sha256: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    #[error("recovery target is outside safe bounds")]
    InvalidTarget,
    #[error("required evidence field is missing: {0}")]
    MissingEvidence(&'static str),
    #[error("backup and source must use distinct failure domains")]
    SameFailureDomain,
    #[error("independent failure-domain evidence is insufficient")]
    InsufficientFailureDomains,
    #[error("restore verification is required")]
    RestoreNotVerified,
    #[error("recovery evidence is incomplete")]
    RecoveryNotVerified,
    #[error("observed RTO/RPO exceeds target")]
    TargetExceeded,
}

pub fn accept_recovery(
    target: &RecoveryTarget,
    evidence: &RecoveryEvidence,
) -> Result<RecoveryReceipt, RecoveryError> {
    validate_target(target)?;
    required(&evidence.source_commit, "source_commit")?;
    required(&evidence.test_command, "test_command")?;
    required(&evidence.artifact_digest, "artifact_digest")?;
    required(&evidence.receipt_id, "receipt_id")?;
    if !valid_commit(&evidence.source_commit) {
        return Err(RecoveryError::MissingEvidence("source_commit"));
    }
    if !valid_digest(&evidence.artifact_digest) {
        return Err(RecoveryError::MissingEvidence("artifact_digest"));
    }
    required(&evidence.backup.backup_id, "backup_id")?;
    required(&evidence.backup.restore_test_id, "restore_test_id")?;
    required(
        &evidence.backup.source_failure_domain,
        "source_failure_domain",
    )?;
    required(
        &evidence.backup.backup_failure_domain,
        "backup_failure_domain",
    )?;
    if evidence.backup.source_lsn == 0 {
        return Err(RecoveryError::MissingEvidence("source_lsn"));
    }
    if !valid_digest(&evidence.backup.snapshot_sha256) {
        return Err(RecoveryError::MissingEvidence("snapshot_sha256"));
    }
    if evidence.backup.source_failure_domain == evidence.backup.backup_failure_domain {
        return Err(RecoveryError::SameFailureDomain);
    }
    if evidence.failure_domain_count < target.required_failure_domains {
        return Err(RecoveryError::InsufficientFailureDomains);
    }
    if !evidence.backup.restore_verified {
        return Err(RecoveryError::RestoreNotVerified);
    }
    if !evidence.wal_replay_verified || !evidence.fencing_verified || !evidence.power_loss_recovered
    {
        return Err(RecoveryError::RecoveryNotVerified);
    }
    if evidence.observed_rto_ms > target.rto_ms || evidence.observed_rpo_bytes > target.rpo_bytes {
        return Err(RecoveryError::TargetExceeded);
    }
    Ok(RecoveryReceipt {
        stage_id: STAGE_ID.into(),
        receipt_id: evidence.receipt_id.clone(),
        accepted: true,
        decision: "COMPLETE_VERIFIED".into(),
        source_commit: evidence.source_commit.clone(),
        artifact_digest: evidence.artifact_digest.clone(),
        backup_id: evidence.backup.backup_id.clone(),
        snapshot_sha256: evidence.backup.snapshot_sha256.clone(),
    })
}

fn validate_target(target: &RecoveryTarget) -> Result<(), RecoveryError> {
    if target.rto_ms == 0 || target.rto_ms > 86_400_000 || target.required_failure_domains < 2 {
        return Err(RecoveryError::InvalidTarget);
    }
    Ok(())
}

fn required(value: &str, field: &'static str) -> Result<(), RecoveryError> {
    if value.trim().is_empty() || value.len() > 4096 {
        return Err(RecoveryError::MissingEvidence(field));
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> RecoveryEvidence {
        RecoveryEvidence {
            source_commit: "a".repeat(40),
            test_command: "recovery-chaos --power-loss".into(),
            artifact_digest: format!("sha256:{}", "c".repeat(64)),
            receipt_id: "receipt-041".into(),
            backup: BackupReceipt {
                backup_id: "backup-041".into(),
                source_lsn: 42,
                snapshot_sha256: format!("sha256:{}", "a".repeat(64)),
                source_failure_domain: "host-a".into(),
                backup_failure_domain: "host-b".into(),
                restore_test_id: "restore-041".into(),
                restore_verified: true,
            },
            wal_replay_verified: true,
            fencing_verified: true,
            power_loss_recovered: true,
            observed_rto_ms: 500,
            observed_rpo_bytes: 0,
            failure_domain_count: 2,
        }
    }

    fn target() -> RecoveryTarget {
        RecoveryTarget {
            rto_ms: 1_000,
            rpo_bytes: 1_024,
            required_failure_domains: 2,
        }
    }

    #[test]
    fn valid_recovery_receipt_is_accepted() {
        let receipt = accept_recovery(&target(), &evidence()).unwrap();
        assert_eq!(receipt.stage_id, STAGE_ID);
        assert!(receipt.accepted);
    }

    #[test]
    fn local_copy_cannot_claim_independent_backup() {
        let mut item = evidence();
        item.backup.backup_failure_domain = "host-a".into();
        assert_eq!(
            accept_recovery(&target(), &item),
            Err(RecoveryError::SameFailureDomain)
        );
    }

    #[test]
    fn restore_and_power_loss_are_required() {
        let mut item = evidence();
        item.backup.restore_verified = false;
        assert_eq!(
            accept_recovery(&target(), &item),
            Err(RecoveryError::RestoreNotVerified)
        );
        let mut item = evidence();
        item.power_loss_recovered = false;
        assert_eq!(
            accept_recovery(&target(), &item),
            Err(RecoveryError::RecoveryNotVerified)
        );
    }

    #[test]
    fn target_breach_is_rejected() {
        let mut item = evidence();
        item.observed_rto_ms = 1_001;
        assert_eq!(
            accept_recovery(&target(), &item),
            Err(RecoveryError::TargetExceeded)
        );
    }

    #[test]
    fn malformed_backup_identity_or_digest_is_rejected() {
        let mut item = evidence();
        item.backup.snapshot_sha256 = "sha256:stub".into();
        assert_eq!(
            accept_recovery(&target(), &item),
            Err(RecoveryError::MissingEvidence("snapshot_sha256"))
        );
        let mut item = evidence();
        item.backup.source_lsn = 0;
        assert_eq!(
            accept_recovery(&target(), &item),
            Err(RecoveryError::MissingEvidence("source_lsn"))
        );
    }
}
