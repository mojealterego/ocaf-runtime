//! Fail-closed signed-evidence adapter for the historical stage ledger.
//!
//! Unlike StageLedger::complete, this wrapper cannot be initialized with
//! preverified stages and never exposes mutable access to its inner ledger.
//! For now only TESTED_LOCAL is eligible: CI/device/release require external
//! evidence validation and a stronger cross-system receipt contract.

use crate::evidence_gate::{
    DenyExternalEvidence, EvidenceAuthorities, EvidenceError, EvidenceGate, EvidenceReceipt,
    ProofContext, VerificationStatus,
};
use ocaf_stage_ledger::{StageLedger, StageLedgerError, StageReceipt, StageRecord, StageStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifiedStageError {
    Ledger(StageLedgerError),
    Evidence(EvidenceError),
    BindingMismatch,
    InsufficientAssurance,
}

impl From<StageLedgerError> for VerifiedStageError {
    fn from(value: StageLedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<EvidenceError> for VerifiedStageError {
    fn from(value: EvidenceError) -> Self {
        Self::Evidence(value)
    }
}

pub struct VerifiedStageLedger {
    ledger: StageLedger,
    gate: EvidenceGate,
}

impl VerifiedStageLedger {
    /// Prevent a caller from importing self-declared COMPLETE_VERIFIED stages.
    pub fn new(
        records: impl IntoIterator<Item = StageRecord>,
        authorities: EvidenceAuthorities,
    ) -> Result<Self, StageLedgerError> {
        let records: Vec<StageRecord> = records.into_iter().collect();
        if records.iter().any(|record| {
            !matches!(record.status, StageStatus::Ready | StageStatus::Blocked)
                || record.claimed_by.is_some()
        }) {
            return Err(StageLedgerError::InvalidGraph);
        }
        Ok(Self {
            ledger: StageLedger::new(records)?,
            gate: EvidenceGate::new(authorities),
        })
    }

    pub fn get(&self, stage_id: &str) -> Option<&StageRecord> {
        self.ledger.get(stage_id)
    }

    pub fn claim_next(&mut self, actor: &str) -> Result<String, VerifiedStageError> {
        self.ledger.claim_next(actor).map_err(Into::into)
    }

    /// No effect on the ledger unless every binding and Ed25519 check passes.
    /// This is a local-test evidence gate, not a production approval API.
    pub fn complete_signed_local(
        &mut self,
        receipt: StageReceipt,
        evidence: &EvidenceReceipt,
    ) -> Result<(), VerifiedStageError> {
        if evidence.status != VerificationStatus::TestedLocal {
            return Err(VerifiedStageError::InsufficientAssurance);
        }
        let ProofContext::Local { test_command, .. } = &evidence.proof else {
            return Err(VerifiedStageError::BindingMismatch);
        };
        if evidence.stage_id != receipt.stage_id
            || evidence.source_commit != receipt.source_commit
            || test_command != &receipt.test_command
            || receipt.test_result != "PASS"
            || !receipt
                .artifact_digest
                .eq_ignore_ascii_case(&format!("sha256:{}", encode_hex(&evidence.artifact_sha256)))
        {
            return Err(VerifiedStageError::BindingMismatch);
        }
        self.gate.verify(evidence, &DenyExternalEvidence)?;
        self.ledger.complete(receipt)?;
        Ok(())
    }
}

fn encode_hex(digest: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::pkey::{PKey, Private};
    use openssl::sign::Signer;

    fn record() -> StageRecord {
        StageRecord {
            stage_id: "320".into(),
            dependencies: vec![],
            status: StageStatus::Ready,
            claimed_by: None,
        }
    }

    fn receipt() -> StageReceipt {
        StageReceipt {
            stage_id: "320".into(),
            claimed_by: "OCAF-SWARM-01".into(),
            source_commit: "a".repeat(40),
            test_command: "cargo test --locked".into(),
            test_result: "PASS".into(),
            artifact_digest: format!("sha256:{}", "11".repeat(32)),
            receipt_id: "receipt-320".into(),
            known_limits: vec![],
        }
    }

    fn evidence(key: &PKey<Private>) -> EvidenceReceipt {
        let mut evidence = EvidenceReceipt {
            stage_id: "320".into(),
            status: VerificationStatus::TestedLocal,
            source_commit: "a".repeat(40),
            artifact_sha256: [0x11; 32],
            deterministic_seed: 42,
            test_vector_sha256: [0x22; 32],
            passed_assertions: 7,
            proof: ProofContext::Local {
                test_command: "cargo test --locked".into(),
                log_sha256: [0x33; 32],
            },
            signature_ed25519: vec![0; 64],
        };
        let mut signer = Signer::new_without_digest(key).unwrap();
        evidence.signature_ed25519 = signer
            .sign_oneshot_to_vec(&evidence.signing_bytes())
            .unwrap();
        evidence
    }

    fn setup() -> (VerifiedStageLedger, PKey<Private>) {
        let private = PKey::generate_ed25519().unwrap();
        let public: [u8; 32] = private.raw_public_key().unwrap().try_into().unwrap();
        let authorities = EvidenceAuthorities::new(public, None, None, None).unwrap();
        (
            VerifiedStageLedger::new([record()], authorities).unwrap(),
            private,
        )
    }

    #[test]
    fn signed_local_evidence_completes_claimed_stage() {
        let (mut ledger, key) = setup();
        assert_eq!(ledger.claim_next("OCAF-SWARM-01").unwrap(), "320");
        ledger
            .complete_signed_local(receipt(), &evidence(&key))
            .unwrap();
        assert_eq!(
            ledger.get("320").unwrap().status,
            StageStatus::CompleteVerified
        );
    }

    #[test]
    fn forged_pass_or_wrong_binding_does_not_complete_stage() {
        let (mut ledger, key) = setup();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        let signed = evidence(&key);
        for mut bad in [receipt(), receipt(), receipt(), receipt()]
            .into_iter()
            .enumerate()
            .map(|(i, mut r)| {
                match i {
                    0 => r.artifact_digest = format!("sha256:{}", "ff".repeat(32)),
                    1 => r.source_commit = "b".repeat(40),
                    2 => r.test_command = "echo PASS".into(),
                    _ => r.test_result = "FAIL".into(),
                }
                r
            })
        {
            // Keep each negative attempt independent of the others.
            assert_eq!(
                ledger.complete_signed_local(bad.clone(), &signed),
                Err(VerifiedStageError::BindingMismatch)
            );
            bad.receipt_id.push_str("-tampered");
            assert_eq!(ledger.get("320").unwrap().status, StageStatus::Claimed);
        }
    }

    #[test]
    fn signed_evidence_tampering_is_rejected() {
        let (mut ledger, key) = setup();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        let mut signed = evidence(&key);
        signed.deterministic_seed += 1;
        assert_eq!(
            ledger.complete_signed_local(receipt(), &signed),
            Err(VerifiedStageError::Evidence(
                EvidenceError::InvalidSignature
            ))
        );
        assert_eq!(ledger.get("320").unwrap().status, StageStatus::Claimed);
    }

    #[test]
    fn higher_assurance_cannot_be_claimed_via_local_bridge() {
        let (mut ledger, key) = setup();
        ledger.claim_next("OCAF-SWARM-01").unwrap();
        let mut signed = evidence(&key);
        signed.status = VerificationStatus::ProductionApproved;
        assert_eq!(
            ledger.complete_signed_local(receipt(), &signed),
            Err(VerifiedStageError::InsufficientAssurance)
        );
        assert_eq!(ledger.get("320").unwrap().status, StageStatus::Claimed);
    }

    #[test]
    fn preverified_and_preclaimed_imports_are_rejected() {
        let key = PKey::generate_ed25519().unwrap();
        let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
        let mut stage = record();
        stage.status = StageStatus::CompleteVerified;
        let authority = EvidenceAuthorities::new(public, None, None, None).unwrap();
        assert!(matches!(
            VerifiedStageLedger::new([stage], authority),
            Err(StageLedgerError::InvalidGraph)
        ));
    }
}
