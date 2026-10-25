//! Stage 0.40 contract for PostgreSQL HA on independent kernels.
//!
//! This is an evidence gate, not a claim that three physical hosts are
//! available.  A laboratory netns or artifact handoff cannot satisfy the
//! independent-kernel fields.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PgHaNodeEvidence {
    pub node_id: String,
    pub boot_id: String,
    pub kernel_id: String,
    pub tcp_endpoint: String,
    pub wal_lsn: String,
    pub replication_streaming: bool,
    pub host_attestation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PgHaEvidence {
    pub primary: PgHaNodeEvidence,
    pub standby_b: PgHaNodeEvidence,
    pub standby_c: PgHaNodeEvidence,
    pub witness_lease: String,
    pub stale_primary_writes_denied: bool,
    pub promotion_receipt: String,
    pub recovery_receipt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PgHaReceipt {
    pub stage_id: String,
    pub evidence_sha256: String,
    pub independent_kernels: bool,
    pub live_tcp_wal: bool,
    pub witness_fencing: bool,
    pub stale_primary_denied: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PgHaError {
    #[error("exactly three distinct nodes are required")]
    NodeCount,
    #[error("node identity fields are incomplete")]
    NodeIdentity,
    #[error("nodes do not prove independent kernels")]
    SharedKernel,
    #[error("live TCP WAL streaming is not proven")]
    WalStream,
    #[error("witness fencing or stale-primary denial is missing")]
    Fencing,
    #[error("promotion and recovery receipts are missing")]
    Recovery,
}

pub fn evaluate_stage_040(evidence: &PgHaEvidence) -> Result<PgHaReceipt, PgHaError> {
    let nodes = [&evidence.primary, &evidence.standby_b, &evidence.standby_c];
    if nodes.len() != 3 {
        return Err(PgHaError::NodeCount);
    }
    if nodes.iter().any(|n| {
        n.node_id.is_empty()
            || n.boot_id.is_empty()
            || n.kernel_id.is_empty()
            || n.tcp_endpoint.is_empty()
            || n.wal_lsn.is_empty()
            || n.host_attestation.is_empty()
    }) {
        return Err(PgHaError::NodeIdentity);
    }
    if distinct(&nodes.iter().map(|n| n.node_id.as_str()).collect::<Vec<_>>()) < 3
        || distinct(&nodes.iter().map(|n| n.boot_id.as_str()).collect::<Vec<_>>()) < 3
        || distinct(
            &nodes
                .iter()
                .map(|n| n.kernel_id.as_str())
                .collect::<Vec<_>>(),
        ) < 3
    {
        return Err(PgHaError::SharedKernel);
    }
    if nodes
        .iter()
        .any(|n| !n.tcp_endpoint.contains(':') || !n.replication_streaming)
    {
        return Err(PgHaError::WalStream);
    }
    if evidence.witness_lease.is_empty() || !evidence.stale_primary_writes_denied {
        return Err(PgHaError::Fencing);
    }
    if evidence.promotion_receipt.is_empty() || evidence.recovery_receipt.is_empty() {
        return Err(PgHaError::Recovery);
    }
    let bytes = serde_json::to_vec(evidence).expect("evidence serializes");
    Ok(PgHaReceipt {
        stage_id: "0.40".into(),
        evidence_sha256: format!("sha256:{:x}", Sha256::digest(bytes)),
        independent_kernels: true,
        live_tcp_wal: true,
        witness_fencing: true,
        stale_primary_denied: true,
    })
}

fn distinct(values: &[&str]) -> usize {
    values
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str) -> PgHaNodeEvidence {
        PgHaNodeEvidence {
            node_id: id.into(),
            boot_id: format!("boot-{id}"),
            kernel_id: format!("kernel-{id}"),
            tcp_endpoint: format!("10.0.0.{id}:5432"),
            wal_lsn: "0/100".into(),
            replication_streaming: true,
            host_attestation: format!("attest-{id}"),
        }
    }
    fn evidence() -> PgHaEvidence {
        PgHaEvidence {
            primary: node("a"),
            standby_b: node("b"),
            standby_c: node("c"),
            witness_lease: "lease-1".into(),
            stale_primary_writes_denied: true,
            promotion_receipt: "promotion-1".into(),
            recovery_receipt: "recovery-1".into(),
        }
    }
    #[test]
    fn independent_three_kernel_evidence_is_accepted() {
        let receipt = evaluate_stage_040(&evidence()).unwrap();
        assert_eq!(receipt.stage_id, "0.40");
        assert!(receipt.live_tcp_wal);
    }
    #[test]
    fn shared_kernel_is_rejected() {
        let mut e = evidence();
        e.standby_c.kernel_id = e.primary.kernel_id.clone();
        assert_eq!(evaluate_stage_040(&e), Err(PgHaError::SharedKernel));
    }
    #[test]
    fn missing_fencing_is_rejected() {
        let mut e = evidence();
        e.stale_primary_writes_denied = false;
        assert_eq!(evaluate_stage_040(&e), Err(PgHaError::Fencing));
    }
    #[test]
    fn artifact_handoff_is_not_tcp_stream() {
        let mut e = evidence();
        e.standby_b.replication_streaming = false;
        assert_eq!(evaluate_stage_040(&e), Err(PgHaError::WalStream));
    }
}
