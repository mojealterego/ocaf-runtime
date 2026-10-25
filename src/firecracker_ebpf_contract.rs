//! Fail-closed contract for a Firecracker/KVM and Aya/eBPF boundary (stage 219).
//!
//! This module does not start a VM or load an eBPF program.  It is the typed
//! hand-off between that privileged executor and the runtime: a launch can be
//! accepted only when the executor reports the required KVM, guest, and
//! kernel-enforcement facts.  A local fixture cannot turn those facts on.
//! Hardware and backend verification therefore remains an explicit gate.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

const MAX_ID_BYTES: usize = 128;
const MAX_CIDRS: usize = 64;
const KVM_API_VERSION: u32 = 12;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicrovmLimits {
    pub vcpu_count: u16,
    pub memory_mib: u32,
    pub pids_max: u32,
    pub wall_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EgressPolicy {
    pub allow_network: bool,
    #[serde(default)]
    pub allowed_cidrs: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirecrackerLaunchRequest {
    pub tenant_id: String,
    pub workload_id: String,
    pub firecracker_sha256: String,
    pub kernel_sha256: String,
    pub rootfs_sha256: String,
    pub jailer_uid: u32,
    pub cgroup_inode: u64,
    pub limits: MicrovmLimits,
    pub egress: EgressPolicy,
    pub nonce: String,
}

/// Facts returned by the privileged Linux/KVM executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareEvidence {
    pub kvm_device_present: bool,
    pub kvm_api_version: u32,
    pub kvm_vm_created: bool,
    pub firecracker_process_started: bool,
    pub guest_booted: bool,
    pub ebpf_program_loaded: bool,
    pub ebpf_program_attached: bool,
    pub egress_policy_enforced: bool,
    pub teardown_verified: bool,
    /// A lab proof may leave this false.  It must be true before production
    /// approval is reported.
    pub production_host_attested: bool,
    pub kernel_program_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateOutcome {
    VerifiedLab,
    ProductionApproved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateReceipt {
    pub schema_version: u32,
    pub request_sha256: String,
    pub evidence_sha256: String,
    pub kvm_api_version: u32,
    pub kernel_program_id: u32,
    pub outcome: GateOutcome,
    pub production_approved: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GateError {
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(&'static str),
    #[error("invalid SHA-256 digest: {0}")]
    InvalidHash(&'static str),
    #[error("invalid resource limit: {0}")]
    InvalidLimit(&'static str),
    #[error("network policy contains rules while network access is denied")]
    NetworkPolicyConflict,
    #[error("invalid egress rule")]
    InvalidEgressRule,
    #[error("hardware evidence is missing or false: {0}")]
    HardwareEvidenceMissing(&'static str),
    #[error("KVM API version is unsupported")]
    UnsupportedKvmApi,
    #[error("eBPF program id is missing or invalid")]
    InvalidKernelProgramId,
    #[error("receipt integrity check failed")]
    ReceiptIntegrity,
    #[error("receipt serialization failed: {0}")]
    Serialization(String),
}

/// Validate a launch request without invoking Firecracker or touching KVM.
pub fn validate_launch_request(request: &FirecrackerLaunchRequest) -> Result<(), GateError> {
    validate_id(&request.tenant_id, "tenant_id")?;
    validate_id(&request.workload_id, "workload_id")?;
    validate_id(&request.nonce, "nonce")?;
    validate_hash(&request.firecracker_sha256, "firecracker_sha256")?;
    validate_hash(&request.kernel_sha256, "kernel_sha256")?;
    validate_hash(&request.rootfs_sha256, "rootfs_sha256")?;
    if request.jailer_uid == 0 {
        return Err(GateError::InvalidIdentifier("jailer_uid"));
    }
    if request.cgroup_inode == 0 {
        return Err(GateError::InvalidIdentifier("cgroup_inode"));
    }
    if !(1..=64).contains(&request.limits.vcpu_count) {
        return Err(GateError::InvalidLimit("vcpu_count"));
    }
    if !(16..=1_048_576).contains(&request.limits.memory_mib) {
        return Err(GateError::InvalidLimit("memory_mib"));
    }
    if !(1..=100_000).contains(&request.limits.pids_max) {
        return Err(GateError::InvalidLimit("pids_max"));
    }
    if !(1..=86_400_000).contains(&request.limits.wall_time_ms) {
        return Err(GateError::InvalidLimit("wall_time_ms"));
    }
    if request.egress.allowed_cidrs.len() > MAX_CIDRS {
        return Err(GateError::InvalidEgressRule);
    }
    if !request.egress.allow_network && !request.egress.allowed_cidrs.is_empty() {
        return Err(GateError::NetworkPolicyConflict);
    }
    for rule in &request.egress.allowed_cidrs {
        if rule.is_empty() || rule.len() > MAX_ID_BYTES || rule.bytes().any(|byte| byte <= 0x20) {
            return Err(GateError::InvalidEgressRule);
        }
    }
    Ok(())
}

/// Require every KVM, guest lifecycle, and eBPF enforcement fact before
/// issuing a receipt.  The function deliberately has no simulation mode.
pub fn evaluate_hardware_gate(
    request: &FirecrackerLaunchRequest,
    evidence: &HardwareEvidence,
) -> Result<GateReceipt, GateError> {
    validate_launch_request(request)?;
    if !evidence.kvm_device_present {
        return Err(GateError::HardwareEvidenceMissing("kvm_device_present"));
    }
    if evidence.kvm_api_version != KVM_API_VERSION {
        return Err(GateError::UnsupportedKvmApi);
    }
    let checks = [
        (evidence.kvm_vm_created, "kvm_vm_created"),
        (
            evidence.firecracker_process_started,
            "firecracker_process_started",
        ),
        (evidence.guest_booted, "guest_booted"),
        (evidence.ebpf_program_loaded, "ebpf_program_loaded"),
        (evidence.ebpf_program_attached, "ebpf_program_attached"),
        (evidence.egress_policy_enforced, "egress_policy_enforced"),
        (evidence.teardown_verified, "teardown_verified"),
    ];
    for (passed, name) in checks {
        if !passed {
            return Err(GateError::HardwareEvidenceMissing(name));
        }
    }
    let kernel_program_id = evidence
        .kernel_program_id
        .filter(|program_id| *program_id > 0)
        .ok_or(GateError::InvalidKernelProgramId)?;
    let production_approved = evidence.production_host_attested;
    let outcome = if production_approved {
        GateOutcome::ProductionApproved
    } else {
        GateOutcome::VerifiedLab
    };
    let request_sha256 = hash_json(request)?;
    let evidence_sha256 = hash_json(evidence)?;
    let mut receipt = GateReceipt {
        schema_version: 1,
        request_sha256,
        evidence_sha256,
        kvm_api_version: evidence.kvm_api_version,
        kernel_program_id,
        outcome,
        production_approved,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_digest(&receipt)?;
    Ok(receipt)
}

pub fn verify_gate_receipt(receipt: &GateReceipt) -> Result<(), GateError> {
    validate_hash(&receipt.request_sha256, "request_sha256")?;
    validate_hash(&receipt.evidence_sha256, "evidence_sha256")?;
    validate_hash(&receipt.receipt_sha256, "receipt_sha256")?;
    if receipt.schema_version != 1
        || receipt.kvm_api_version != KVM_API_VERSION
        || receipt.kernel_program_id == 0
        || (receipt.production_approved
            != matches!(receipt.outcome, GateOutcome::ProductionApproved))
    {
        return Err(GateError::ReceiptIntegrity);
    }
    if receipt_digest(receipt)? != receipt.receipt_sha256 {
        return Err(GateError::ReceiptIntegrity);
    }
    Ok(())
}

fn validate_id(value: &str, field: &'static str) -> Result<(), GateError> {
    if value.is_empty() || value.len() > MAX_ID_BYTES || value.bytes().any(|byte| byte == 0) {
        return Err(GateError::InvalidIdentifier(field));
    }
    Ok(())
}

fn validate_hash(value: &str, field: &'static str) -> Result<(), GateError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GateError::InvalidHash(field));
    }
    Ok(())
}

fn hash_json<T: Serialize>(value: &T) -> Result<String, GateError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| GateError::Serialization(error.to_string()))?;
    Ok(hex(&Sha256::digest(bytes)))
}

fn receipt_digest(receipt: &GateReceipt) -> Result<String, GateError> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    hash_json(&unsigned)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
