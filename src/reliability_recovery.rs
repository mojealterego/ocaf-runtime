//! Stage 0.41 reliability/recovery contract.
//!
//! This is an evidence boundary for backup/WAL restore. It does not claim to
//! operate PostgreSQL or an offsite provider; every external operation must
//! supply a verified digest, boot identity, target LSN and integrity receipt.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    Planned,
    BackupVerified,
    WalVerified,
    ReplayCompleted,
    IntegrityVerified,
    ReceiptIssued,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryRequest {
    pub tenant_id: String,
    pub restore_id: String,
    pub source_boot_id: String,
    pub restore_boot_id: String,
    pub target_lsn: u64,
    pub base_backup_sha256: String,
    pub wal_archive_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryReceipt {
    pub tenant_id: String,
    pub restore_id: String,
    pub target_lsn: u64,
    pub base_backup_sha256: String,
    pub wal_archive_sha256: String,
    pub integrity_sha256: String,
    pub replayed_rows: u64,
    pub state: RecoveryState,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecoveryError {
    #[error("invalid recovery identity")]
    InvalidIdentity,
    #[error("boot identity must change across restore")]
    SameBoot,
    #[error("invalid SHA-256 digest")]
    InvalidDigest,
    #[error("invalid recovery transition")]
    InvalidTransition,
    #[error("backup digest mismatch")]
    BackupMismatch,
    #[error("WAL archive digest mismatch")]
    WalMismatch,
    #[error("replayed LSN is below target")]
    LsnNotReached,
    #[error("integrity digest mismatch")]
    IntegrityMismatch,
    #[error("recovery evidence is incomplete")]
    IncompleteEvidence,
}

#[derive(Debug, Clone)]
pub struct ReliabilityRecovery {
    request: RecoveryRequest,
    state: RecoveryState,
    replayed_rows: u64,
    integrity_sha256: Option<String>,
}

impl ReliabilityRecovery {
    pub fn begin(request: RecoveryRequest) -> Result<Self, RecoveryError> {
        validate_request(&request)?;
        Ok(Self { request, state: RecoveryState::Planned, replayed_rows: 0, integrity_sha256: None })
    }

    pub fn state(&self) -> RecoveryState { self.state }

    pub fn verify_backup(&mut self, observed_sha256: &str) -> Result<(), RecoveryError> {
        if self.state != RecoveryState::Planned { return Err(RecoveryError::InvalidTransition); }
        if observed_sha256 != self.request.base_backup_sha256 { return Err(RecoveryError::BackupMismatch); }
        self.state = RecoveryState::BackupVerified;
        Ok(())
    }

    pub fn verify_wal(&mut self, observed_sha256: &str) -> Result<(), RecoveryError> {
        if self.state != RecoveryState::BackupVerified { return Err(RecoveryError::InvalidTransition); }
        if observed_sha256 != self.request.wal_archive_sha256 { return Err(RecoveryError::WalMismatch); }
        self.state = RecoveryState::WalVerified;
        Ok(())
    }

    pub fn record_replay(&mut self, replayed_lsn: u64, rows: u64) -> Result<(), RecoveryError> {
        if self.state != RecoveryState::WalVerified || replayed_lsn < self.request.target_lsn {
            return Err(if replayed_lsn < self.request.target_lsn { RecoveryError::LsnNotReached } else { RecoveryError::InvalidTransition });
        }
        self.replayed_rows = rows;
        self.state = RecoveryState::ReplayCompleted;
        Ok(())
    }

    pub fn verify_integrity(&mut self, expected_sha256: &str, actual_bytes: &[u8]) -> Result<(), RecoveryError> {
        if self.state != RecoveryState::ReplayCompleted || !valid_sha256(expected_sha256) {
            return Err(RecoveryError::IntegrityMismatch);
        }
        let actual = format!("sha256:{:x}", Sha256::digest(actual_bytes));
        if actual != expected_sha256 { return Err(RecoveryError::IntegrityMismatch); }
        self.integrity_sha256 = Some(actual);
        self.state = RecoveryState::IntegrityVerified;
        Ok(())
    }

    pub fn issue_receipt(&mut self) -> Result<RecoveryReceipt, RecoveryError> {
        if self.state != RecoveryState::IntegrityVerified { return Err(RecoveryError::IncompleteEvidence); }
        let integrity_sha256 = self.integrity_sha256.clone().ok_or(RecoveryError::IncompleteEvidence)?;
        self.state = RecoveryState::ReceiptIssued;
        Ok(RecoveryReceipt { tenant_id: self.request.tenant_id.clone(), restore_id: self.request.restore_id.clone(), target_lsn: self.request.target_lsn, base_backup_sha256: self.request.base_backup_sha256.clone(), wal_archive_sha256: self.request.wal_archive_sha256.clone(), integrity_sha256, replayed_rows: self.replayed_rows, state: self.state })
    }
}

fn validate_request(request: &RecoveryRequest) -> Result<(), RecoveryError> {
    if request.tenant_id.is_empty() || request.restore_id.is_empty() || request.source_boot_id.is_empty() || request.restore_boot_id.is_empty() { return Err(RecoveryError::InvalidIdentity); }
    if request.source_boot_id == request.restore_boot_id { return Err(RecoveryError::SameBoot); }
    if request.target_lsn == 0 || !valid_sha256(&request.base_backup_sha256) || !valid_sha256(&request.wal_archive_sha256) { return Err(RecoveryError::InvalidDigest); }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    let hex = value.strip_prefix("sha256:").unwrap_or("");
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RecoveryRequest {
        RecoveryRequest { tenant_id: "tenant-a".into(), restore_id: "restore-1".into(), source_boot_id: "boot-a".into(), restore_boot_id: "boot-b".into(), target_lsn: 42, base_backup_sha256: format!("sha256:{}", "a".repeat(64)), wal_archive_sha256: format!("sha256:{}", "b".repeat(64)) }
    }

    #[test]
    fn recovery_requires_order_and_emits_receipt() {
        let mut recovery = ReliabilityRecovery::begin(request()).unwrap();
        assert!(recovery.issue_receipt().is_err());
        recovery.verify_backup("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        recovery.verify_wal("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        recovery.record_replay(42, 35).unwrap();
        let bytes = b"restored-state";
        let digest = format!("sha256:{:x}", Sha256::digest(bytes));
        recovery.verify_integrity(&digest, bytes).unwrap();
        assert_eq!(recovery.issue_receipt().unwrap().state, RecoveryState::ReceiptIssued);
    }

    #[test]
    fn same_boot_is_rejected() {
        let mut r = request(); r.restore_boot_id = r.source_boot_id.clone();
        assert_eq!(ReliabilityRecovery::begin(r).unwrap_err(), RecoveryError::SameBoot);
    }

    #[test]
    fn stale_lsn_and_bad_integrity_are_rejected() {
        let mut recovery = ReliabilityRecovery::begin(request()).unwrap();
        recovery.verify_backup("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        recovery.verify_wal("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        assert_eq!(recovery.record_replay(41, 35).unwrap_err(), RecoveryError::LsnNotReached);
        recovery.record_replay(42, 35).unwrap();
        assert_eq!(recovery.verify_integrity("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", b"wrong").unwrap_err(), RecoveryError::IntegrityMismatch);
    }
}
