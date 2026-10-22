//! Stage 0.40 acceptance boundary for independent-host PostgreSQL HA.
//!
//! Local netns or multiple postmasters in one kernel do not prove this stage.
//! The contract records the facts that must come from distinct kernels, a
//! witness, fencing and recovery. Missing facts remain fail-closed.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterhostEvidence {
    pub host_a_uuid: String,
    pub host_b_uuid: String,
    pub witness_uuid: String,
    pub distinct_kernel_ids: bool,
    pub streaming_replication_verified: bool,
    pub witness_fencing_verified: bool,
    pub power_loss_recovery_verified: bool,
    pub revoked_key_rejected: bool,
    pub recovery_receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InterhostError {
    #[error("independent host identity is incomplete")]
    HostIdentity,
    #[error("evidence does not prove distinct kernels")]
    SharedKernel,
    #[error("streaming replication is not verified")]
    ReplicationMissing,
    #[error("witness fencing is not verified")]
    FencingMissing,
    #[error("power-loss recovery is not verified")]
    RecoveryMissing,
    #[error("revoked key was not rejected")]
    RevokedKeyAccepted,
    #[error("recovery receipt digest is missing")]
    ReceiptMissing,
}

/// Returns an acceptance receipt only when every independent-host fact exists.
pub fn accept(evidence: &InterhostEvidence) -> Result<String, InterhostError> {
    if evidence.host_a_uuid.trim().is_empty()
        || evidence.host_b_uuid.trim().is_empty()
        || evidence.witness_uuid.trim().is_empty()
        || evidence.host_a_uuid == evidence.host_b_uuid
        || evidence.host_a_uuid == evidence.witness_uuid
        || evidence.host_b_uuid == evidence.witness_uuid
    {
        return Err(InterhostError::HostIdentity);
    }
    if !evidence.distinct_kernel_ids {
        return Err(InterhostError::SharedKernel);
    }
    if !evidence.streaming_replication_verified {
        return Err(InterhostError::ReplicationMissing);
    }
    if !evidence.witness_fencing_verified {
        return Err(InterhostError::FencingMissing);
    }
    if !evidence.power_loss_recovery_verified {
        return Err(InterhostError::RecoveryMissing);
    }
    if !evidence.revoked_key_rejected {
        return Err(InterhostError::RevokedKeyAccepted);
    }
    if !evidence.recovery_receipt_sha256.starts_with("sha256:") {
        return Err(InterhostError::ReceiptMissing);
    }
    Ok(evidence.recovery_receipt_sha256.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> InterhostEvidence {
        InterhostEvidence {
            host_a_uuid: "host-a".into(),
            host_b_uuid: "host-b".into(),
            witness_uuid: "witness".into(),
            distinct_kernel_ids: true,
            streaming_replication_verified: true,
            witness_fencing_verified: true,
            power_loss_recovery_verified: true,
            revoked_key_rejected: true,
            recovery_receipt_sha256: "sha256:recovery".into(),
        }
    }

    #[test]
    fn accepts_only_independent_host_evidence() {
        assert_eq!(accept(&evidence()).unwrap(), "sha256:recovery");
    }

    #[test]
    fn shared_kernel_is_no_go() {
        let mut e = evidence();
        e.distinct_kernel_ids = false;
        assert_eq!(accept(&e), Err(InterhostError::SharedKernel));
    }

    #[test]
    fn missing_witness_fencing_is_no_go() {
        let mut e = evidence();
        e.witness_fencing_verified = false;
        assert_eq!(accept(&e), Err(InterhostError::FencingMissing));
    }
}
