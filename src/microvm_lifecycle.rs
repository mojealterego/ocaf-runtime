//! Stage 0.39.8 lifecycle contract.
//!
//! This module coordinates evidence emitted by a privileged executor. It does
//! not start Firecracker or attach eBPF itself. Every transition is monotonic,
//! tenant/workload scoped, and fail closed until the final teardown receipt.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::firecracker_ebpf_contract::{
    evaluate_hardware_gate, verify_gate_receipt, FirecrackerLaunchRequest, GateReceipt,
    HardwareEvidence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Claimed,
    JailerReady,
    EgressAttached,
    VmRunning,
    ReceiptIssued,
    TornDown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleReceipt {
    pub tenant_id: String,
    pub workload_id: String,
    pub state: LifecycleState,
    pub gate_receipt_sha256: String,
    pub teardown_verified: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    #[error("invalid lifecycle transition from {from:?} to {to:?}")]
    InvalidTransition { from: LifecycleState, to: LifecycleState },
    #[error("request identity does not match lifecycle identity")]
    IdentityMismatch,
    #[error("hardware gate evidence is insufficient")]
    GateRejected,
    #[error("teardown receipt is required")]
    TeardownRequired,
}

#[derive(Debug, Clone)]
pub struct MicrovmLifecycle {
    request: FirecrackerLaunchRequest,
    state: LifecycleState,
    gate_receipt: Option<GateReceipt>,
}

impl MicrovmLifecycle {
    pub fn claim(request: FirecrackerLaunchRequest) -> Result<Self, LifecycleError> {
        crate::firecracker_ebpf_contract::validate_launch_request(&request)
            .map_err(|_| LifecycleError::GateRejected)?;
        Ok(Self { request, state: LifecycleState::Claimed, gate_receipt: None })
    }

    pub fn state(&self) -> LifecycleState { self.state }

    pub fn advance(&mut self, next: LifecycleState) -> Result<(), LifecycleError> {
        let allowed = matches!(
            (self.state, next),
            (LifecycleState::Claimed, LifecycleState::JailerReady)
                | (LifecycleState::JailerReady, LifecycleState::EgressAttached)
        );
        if !allowed {
            return Err(LifecycleError::InvalidTransition { from: self.state, to: next });
        }
        self.state = next;
        Ok(())
    }

    pub fn record_gate(&mut self, evidence: &HardwareEvidence) -> Result<(), LifecycleError> {
        if self.state != LifecycleState::EgressAttached {
            return Err(LifecycleError::InvalidTransition { from: self.state, to: LifecycleState::VmRunning });
        }
        let receipt = evaluate_hardware_gate(&self.request, evidence)
            .map_err(|_| LifecycleError::GateRejected)?;
        self.gate_receipt = Some(receipt);
        self.state = LifecycleState::VmRunning;
        Ok(())
    }

    pub fn issue_receipt(&mut self) -> Result<(), LifecycleError> {
        if self.state != LifecycleState::VmRunning || self.gate_receipt.is_none() {
            return Err(LifecycleError::InvalidTransition { from: self.state, to: LifecycleState::ReceiptIssued });
        }
        self.state = LifecycleState::ReceiptIssued;
        Ok(())
    }

    pub fn teardown(&mut self, teardown_verified: bool) -> Result<LifecycleReceipt, LifecycleError> {
        if self.state != LifecycleState::ReceiptIssued || !teardown_verified {
            return Err(LifecycleError::TeardownRequired);
        }
        let gate = self.gate_receipt.as_ref().ok_or(LifecycleError::TeardownRequired)?;
        verify_gate_receipt(gate).map_err(|_| LifecycleError::GateRejected)?;
        self.state = LifecycleState::TornDown;
        Ok(LifecycleReceipt {
            tenant_id: self.request.tenant_id.clone(),
            workload_id: self.request.workload_id.clone(),
            state: self.state,
            gate_receipt_sha256: gate.receipt_sha256.clone(),
            teardown_verified,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firecracker_ebpf_contract::{EgressPolicy, MicrovmLimits};
    use std::collections::BTreeSet;

    fn request() -> FirecrackerLaunchRequest {
        FirecrackerLaunchRequest {
            tenant_id: "tenant-a".into(), workload_id: "workload-a".into(),
            firecracker_sha256: "a".repeat(64), kernel_sha256: "b".repeat(64), rootfs_sha256: "c".repeat(64),
            jailer_uid: 1000, cgroup_inode: 42,
            limits: MicrovmLimits { vcpu_count: 1, memory_mib: 128, pids_max: 64, wall_time_ms: 1000 },
            egress: EgressPolicy { allow_network: false, allowed_cidrs: BTreeSet::new() }, nonce: "nonce-a".into(),
        }
    }

    fn evidence() -> HardwareEvidence {
        HardwareEvidence { kvm_device_present: true, kvm_api_version: 12, kvm_vm_created: true,
            firecracker_process_started: true, guest_booted: true, ebpf_program_loaded: true,
            ebpf_program_attached: true, egress_policy_enforced: true, teardown_verified: true,
            production_host_attested: false, kernel_program_id: Some(7) }
    }

    #[test]
    fn lifecycle_requires_order_and_teardown() {
        let mut flow = MicrovmLifecycle::claim(request()).unwrap();
        assert!(flow.record_gate(&evidence()).is_err());
        flow.advance(LifecycleState::JailerReady).unwrap();
        flow.advance(LifecycleState::EgressAttached).unwrap();
        flow.record_gate(&evidence()).unwrap();
        flow.issue_receipt().unwrap();
        assert!(flow.teardown(false).is_err());
        assert_eq!(flow.teardown(true).unwrap().state, LifecycleState::TornDown);
    }

    #[test]
    fn incomplete_hardware_never_reaches_vm() {
        let mut flow = MicrovmLifecycle::claim(request()).unwrap();
        flow.advance(LifecycleState::JailerReady).unwrap();
        flow.advance(LifecycleState::EgressAttached).unwrap();
        let mut evidence = evidence(); evidence.ebpf_program_attached = false;
        assert!(flow.record_gate(&evidence).is_err());
        assert_eq!(flow.state(), LifecycleState::EgressAttached);
    }
}
