//! Signed evidence-receipt integrity and assurance gates.
//!
//! A valid Ed25519 signature proves only that a configured authority signed
//! these exact bytes. It does NOT independently prove a CI run passed, that a
//! TPM/TEE produced an attestation, or that SLSA provenance was validated.
//! Higher assurance requires a separate trusted external evidence validator.
//! No signing API is exported to agents; public keys come from the operator.

use openssl::pkey::{Id, PKey};
use openssl::sign::Verifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const DOMAIN: &[u8] = b"OCAF/EVIDENCE-RECEIPT/ED25519/v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    NotImplemented,
    Partial,
    ImplementedUnverified,
    TestedLocal,
    CiVerified,
    DeviceVerified,
    ProductionApproved,
}

impl VerificationStatus {
    fn wire_code(self) -> u8 {
        match self {
            Self::NotImplemented => 0,
            Self::Partial => 1,
            Self::ImplementedUnverified => 2,
            Self::TestedLocal => 3,
            Self::CiVerified => 4,
            Self::DeviceVerified => 5,
            Self::ProductionApproved => 6,
        }
    }

    pub fn is_verified(self) -> bool {
        matches!(
            self,
            Self::TestedLocal | Self::CiVerified | Self::DeviceVerified | Self::ProductionApproved
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProofContext {
    Specification,
    Local {
        test_command: String,
        log_sha256: [u8; 32],
    },
    Ci {
        run_url: String,
        runner_image_sha256: [u8; 32],
        log_sha256: [u8; 32],
    },
    Device {
        device_id: String,
        hardware_attestation_sha256: [u8; 32],
        log_sha256: [u8; 32],
    },
    Production {
        slsa_statement_sha256: [u8; 32],
        release_authorization_sha256: [u8; 32],
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReceipt {
    pub stage_id: String,
    pub status: VerificationStatus,
    /// Exact source commit (Git SHA-1 or SHA-256 object ID), not a branch name.
    pub source_commit: String,
    pub artifact_sha256: [u8; 32],
    pub deterministic_seed: u64,
    pub test_vector_sha256: [u8; 32],
    pub passed_assertions: u64,
    pub proof: ProofContext,
    pub signature_ed25519: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceError {
    InvalidAuthority,
    MissingAuthority,
    InvalidReceipt,
    InvalidSignature,
    ExternalAttestationRequired,
}

fn nonzero(digest: &[u8; 32]) -> bool {
    digest.iter().any(|byte| *byte != 0)
}

fn valid_label(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn valid_stage_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn field(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

impl EvidenceReceipt {
    fn validate(&self) -> Result<(), EvidenceError> {
        if !valid_stage_id(&self.stage_id)
            || !matches!(self.source_commit.len(), 40 | 64)
            || !self
                .source_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self.signature_ed25519.len() != 64
        {
            return Err(EvidenceError::InvalidReceipt);
        }
        if self.status.is_verified()
            && (!nonzero(&self.artifact_sha256)
                || !nonzero(&self.test_vector_sha256)
                || self.passed_assertions == 0)
        {
            return Err(EvidenceError::InvalidReceipt);
        }
        if !self.status.is_verified() && self.passed_assertions != 0 {
            return Err(EvidenceError::InvalidReceipt);
        }
        let proof_valid = match (&self.status, &self.proof) {
            (
                VerificationStatus::NotImplemented
                | VerificationStatus::Partial
                | VerificationStatus::ImplementedUnverified,
                ProofContext::Specification,
            ) => true,
            (
                VerificationStatus::TestedLocal,
                ProofContext::Local {
                    test_command,
                    log_sha256,
                },
            ) => valid_label(test_command, 512) && nonzero(log_sha256),
            (
                VerificationStatus::CiVerified,
                ProofContext::Ci {
                    run_url,
                    runner_image_sha256,
                    log_sha256,
                },
            ) => {
                run_url.starts_with("https://github.com/")
                    && run_url.contains("/actions/runs/")
                    && valid_label(run_url, 512)
                    && nonzero(runner_image_sha256)
                    && nonzero(log_sha256)
            }
            (
                VerificationStatus::DeviceVerified,
                ProofContext::Device {
                    device_id,
                    hardware_attestation_sha256,
                    log_sha256,
                },
            ) => {
                valid_label(device_id, 256)
                    && nonzero(hardware_attestation_sha256)
                    && nonzero(log_sha256)
            }
            (
                VerificationStatus::ProductionApproved,
                ProofContext::Production {
                    slsa_statement_sha256,
                    release_authorization_sha256,
                },
            ) => nonzero(slsa_statement_sha256) && nonzero(release_authorization_sha256),
            _ => false,
        };
        if !proof_valid {
            return Err(EvidenceError::InvalidReceipt);
        }
        Ok(())
    }

    /// Stable, length-delimited, domain-separated transcript. All status and
    /// assurance-specific fields are bound into the signature.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = DOMAIN.to_vec();
        for value in [
            self.stage_id.as_bytes(),
            &[self.status.wire_code()],
            self.source_commit.as_bytes(),
            &self.artifact_sha256,
            &self.deterministic_seed.to_be_bytes(),
            &self.test_vector_sha256,
            &self.passed_assertions.to_be_bytes(),
        ] {
            field(&mut bytes, value);
        }
        match &self.proof {
            ProofContext::Specification => field(&mut bytes, &[0]),
            ProofContext::Local {
                test_command,
                log_sha256,
            } => {
                field(&mut bytes, &[1]);
                field(&mut bytes, test_command.as_bytes());
                field(&mut bytes, log_sha256);
            }
            ProofContext::Ci {
                run_url,
                runner_image_sha256,
                log_sha256,
            } => {
                field(&mut bytes, &[2]);
                field(&mut bytes, run_url.as_bytes());
                field(&mut bytes, runner_image_sha256);
                field(&mut bytes, log_sha256);
            }
            ProofContext::Device {
                device_id,
                hardware_attestation_sha256,
                log_sha256,
            } => {
                field(&mut bytes, &[3]);
                field(&mut bytes, device_id.as_bytes());
                field(&mut bytes, hardware_attestation_sha256);
                field(&mut bytes, log_sha256);
            }
            ProofContext::Production {
                slsa_statement_sha256,
                release_authorization_sha256,
            } => {
                field(&mut bytes, &[4]);
                field(&mut bytes, slsa_statement_sha256);
                field(&mut bytes, release_authorization_sha256);
            }
        }
        bytes
    }
}

/// Keys are operator-provisioned, distinct Ed25519 public keys.
/// A key absent for a tier means the tier is NOT enabled.
pub struct EvidenceAuthorities {
    local: [u8; 32],
    ci: Option<[u8; 32]>,
    device: Option<[u8; 32]>,
    release: Option<[u8; 32]>,
}

impl EvidenceAuthorities {
    pub fn new(
        local: [u8; 32],
        ci: Option<[u8; 32]>,
        device: Option<[u8; 32]>,
        release: Option<[u8; 32]>,
    ) -> Result<Self, EvidenceError> {
        let mut seen = BTreeSet::new();
        for key in [Some(local), ci, device, release].into_iter().flatten() {
            if !nonzero(&key) || !seen.insert(key) {
                return Err(EvidenceError::InvalidAuthority);
            }
            if PKey::public_key_from_raw_bytes(&key, Id::ED25519).is_err() {
                return Err(EvidenceError::InvalidAuthority);
            }
        }
        Ok(Self {
            local,
            ci,
            device,
            release,
        })
    }

    fn key_for(&self, status: VerificationStatus) -> Option<&[u8; 32]> {
        match status {
            VerificationStatus::NotImplemented
            | VerificationStatus::Partial
            | VerificationStatus::ImplementedUnverified
            | VerificationStatus::TestedLocal => Some(&self.local),
            VerificationStatus::CiVerified => self.ci.as_ref(),
            VerificationStatus::DeviceVerified => self.device.as_ref(),
            VerificationStatus::ProductionApproved => self.release.as_ref(),
        }
    }
}

/// An external validator must be implemented in a trusted host, NEVER by the
/// requesting agent. Merely possessing a signed digest is not an attestation.
pub trait ExternalEvidenceValidator: Send + Sync {
    fn verify_ci(&self, receipt: &EvidenceReceipt) -> bool;
    fn verify_device(&self, receipt: &EvidenceReceipt) -> bool;
    fn verify_release(&self, receipt: &EvidenceReceipt) -> bool;
}

/// Safe default: no CI, hardware, or release claims can pass without an
/// independently configured verifier.
pub struct DenyExternalEvidence;

impl ExternalEvidenceValidator for DenyExternalEvidence {
    fn verify_ci(&self, _receipt: &EvidenceReceipt) -> bool {
        false
    }
    fn verify_device(&self, _receipt: &EvidenceReceipt) -> bool {
        false
    }
    fn verify_release(&self, _receipt: &EvidenceReceipt) -> bool {
        false
    }
}

pub struct EvidenceGate {
    authorities: EvidenceAuthorities,
}

impl EvidenceGate {
    pub fn new(authorities: EvidenceAuthorities) -> Self {
        Self { authorities }
    }

    pub fn verify(
        &self,
        receipt: &EvidenceReceipt,
        external: &dyn ExternalEvidenceValidator,
    ) -> Result<(), EvidenceError> {
        receipt.validate()?;
        let raw_key = self
            .authorities
            .key_for(receipt.status)
            .ok_or(EvidenceError::MissingAuthority)?;
        let key = PKey::public_key_from_raw_bytes(raw_key, Id::ED25519)
            .map_err(|_| EvidenceError::InvalidAuthority)?;
        let mut verifier =
            Verifier::new_without_digest(&key).map_err(|_| EvidenceError::InvalidAuthority)?;
        let accepted = verifier
            .verify_oneshot(&receipt.signature_ed25519, &receipt.signing_bytes())
            .map_err(|_| EvidenceError::InvalidSignature)?;
        if !accepted {
            return Err(EvidenceError::InvalidSignature);
        }
        let externally_verified = match receipt.status {
            VerificationStatus::CiVerified => external.verify_ci(receipt),
            VerificationStatus::DeviceVerified => external.verify_device(receipt),
            VerificationStatus::ProductionApproved => external.verify_release(receipt),
            _ => true,
        };
        if !externally_verified {
            return Err(EvidenceError::ExternalAttestationRequired);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::pkey::{PKey, Private};
    use openssl::sign::Signer;

    fn keys() -> (PKey<Private>, PKey<Private>, PKey<Private>, PKey<Private>) {
        (
            PKey::generate_ed25519().unwrap(),
            PKey::generate_ed25519().unwrap(),
            PKey::generate_ed25519().unwrap(),
            PKey::generate_ed25519().unwrap(),
        )
    }

    fn public(key: &PKey<Private>) -> [u8; 32] {
        key.raw_public_key().unwrap().try_into().unwrap()
    }

    fn gate(keys: &(PKey<Private>, PKey<Private>, PKey<Private>, PKey<Private>)) -> EvidenceGate {
        EvidenceGate::new(
            EvidenceAuthorities::new(
                public(&keys.0),
                Some(public(&keys.1)),
                Some(public(&keys.2)),
                Some(public(&keys.3)),
            )
            .unwrap(),
        )
    }

    fn receipt(status: VerificationStatus, proof: ProofContext) -> EvidenceReceipt {
        EvidenceReceipt {
            stage_id: "320".into(),
            status,
            source_commit: "a".repeat(40),
            artifact_sha256: [1; 32],
            deterministic_seed: 42,
            test_vector_sha256: [2; 32],
            passed_assertions: if status.is_verified() { 7 } else { 0 },
            proof,
            signature_ed25519: vec![0; 64],
        }
    }

    fn sign(receipt: &mut EvidenceReceipt, key: &PKey<Private>) {
        let mut signer = Signer::new_without_digest(key).unwrap();
        receipt.signature_ed25519 = signer
            .sign_oneshot_to_vec(&receipt.signing_bytes())
            .unwrap();
    }

    fn local() -> ProofContext {
        ProofContext::Local {
            test_command: "cargo test --locked".into(),
            log_sha256: [3; 32],
        }
    }

    #[test]
    fn local_receipt_is_authenticated_and_binds_all_fields() {
        let keys = keys();
        let gate = gate(&keys);
        let mut r = receipt(VerificationStatus::TestedLocal, local());
        sign(&mut r, &keys.0);
        assert_eq!(gate.verify(&r, &DenyExternalEvidence), Ok(()));
        let mut changed = r.clone();
        changed.deterministic_seed += 1;
        assert_eq!(
            gate.verify(&changed, &DenyExternalEvidence),
            Err(EvidenceError::InvalidSignature)
        );
        let mut changed = r.clone();
        changed.stage_id = "321".into();
        assert_eq!(
            gate.verify(&changed, &DenyExternalEvidence),
            Err(EvidenceError::InvalidSignature)
        );
        let mut changed = r;
        changed.source_commit = "b".repeat(40);
        assert_eq!(
            gate.verify(&changed, &DenyExternalEvidence),
            Err(EvidenceError::InvalidSignature)
        );
    }

    #[test]
    fn ci_signature_alone_does_not_promote_status() {
        let keys = keys();
        let gate = gate(&keys);
        let mut r = receipt(
            VerificationStatus::CiVerified,
            ProofContext::Ci {
                run_url: "https://github.com/mojealterego/ocaf-runtime/actions/runs/42".into(),
                runner_image_sha256: [4; 32],
                log_sha256: [5; 32],
            },
        );
        sign(&mut r, &keys.0);
        assert_eq!(
            gate.verify(&r, &DenyExternalEvidence),
            Err(EvidenceError::InvalidSignature)
        );
        sign(&mut r, &keys.1);
        assert_eq!(
            gate.verify(&r, &DenyExternalEvidence),
            Err(EvidenceError::ExternalAttestationRequired)
        );
    }

    #[test]
    fn device_and_production_require_independent_attestation() {
        let keys = keys();
        let gate = gate(&keys);
        let mut device = receipt(
            VerificationStatus::DeviceVerified,
            ProofContext::Device {
                device_id: "android-device-01".into(),
                hardware_attestation_sha256: [6; 32],
                log_sha256: [7; 32],
            },
        );
        sign(&mut device, &keys.2);
        assert_eq!(
            gate.verify(&device, &DenyExternalEvidence),
            Err(EvidenceError::ExternalAttestationRequired)
        );
        let mut production = receipt(
            VerificationStatus::ProductionApproved,
            ProofContext::Production {
                slsa_statement_sha256: [8; 32],
                release_authorization_sha256: [9; 32],
            },
        );
        sign(&mut production, &keys.3);
        assert_eq!(
            gate.verify(&production, &DenyExternalEvidence),
            Err(EvidenceError::ExternalAttestationRequired)
        );
        production.signature_ed25519.truncate(32);
        assert_eq!(
            gate.verify(&production, &DenyExternalEvidence),
            Err(EvidenceError::InvalidReceipt)
        );
    }

    #[test]
    fn incomplete_or_mismatched_proof_is_denied_before_signature_check() {
        let keys = keys();
        let gate = gate(&keys);
        let mut r = receipt(VerificationStatus::CiVerified, local());
        sign(&mut r, &keys.1);
        assert_eq!(
            gate.verify(&r, &DenyExternalEvidence),
            Err(EvidenceError::InvalidReceipt)
        );
        r.proof = ProofContext::Ci {
            run_url: "http://localhost/fake".into(),
            runner_image_sha256: [4; 32],
            log_sha256: [5; 32],
        };
        sign(&mut r, &keys.1);
        assert_eq!(
            gate.verify(&r, &DenyExternalEvidence),
            Err(EvidenceError::InvalidReceipt)
        );
        r.proof = ProofContext::Ci {
            run_url: "https://github.com/mojealterego/ocaf-runtime/actions/runs/42".into(),
            runner_image_sha256: [0; 32],
            log_sha256: [5; 32],
        };
        sign(&mut r, &keys.1);
        assert_eq!(
            gate.verify(&r, &DenyExternalEvidence),
            Err(EvidenceError::InvalidReceipt)
        );
    }

    #[test]
    fn independent_python_ed25519_vector_is_byte_exact() {
        use sha2::{Digest, Sha256};
        fn hex_bytes(value: &str) -> Vec<u8> {
            (0..value.len())
                .step_by(2)
                .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
                .collect()
        }
        // Generated with Python cryptography Ed25519 from seed 00..1f.
        // See docs/evidence/OCAF_EVIDENCE_ED25519_VECTOR_2026-11-15.json.
        let public: [u8; 32] =
            hex_bytes("03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8")
                .try_into()
                .unwrap();
        let gate = EvidenceGate::new(EvidenceAuthorities::new(public, None, None, None).unwrap());
        let mut r = receipt(VerificationStatus::TestedLocal, local());
        r.signature_ed25519 = hex_bytes(concat!(
            "64134d7db962f2d57c36246857c78f8227dbeba24a815bc124863f8a1ad2839",
            "1656d5259435bf3ad7d8c4f225eaefed0ad5253a8c3ed1a5e01f26ec39803560a"
        ));
        let expected =
            hex_bytes("2c29e3d404f0c748692072a42c02c7d8a4ec58e4a72dae4c1314ccec5ca2cebf");
        assert_eq!(
            Sha256::digest(r.signing_bytes()).as_slice(),
            expected.as_slice()
        );
        assert_eq!(gate.verify(&r, &DenyExternalEvidence), Ok(()));
    }

    #[test]
    fn authority_tiers_must_have_distinct_public_keys() {
        let keys = keys();
        assert!(matches!(
            EvidenceAuthorities::new(public(&keys.0), Some(public(&keys.0)), None, None),
            Err(EvidenceError::InvalidAuthority)
        ));
        let gate =
            EvidenceGate::new(EvidenceAuthorities::new(public(&keys.0), None, None, None).unwrap());
        let mut r = receipt(
            VerificationStatus::CiVerified,
            ProofContext::Ci {
                run_url: "https://github.com/mojealterego/ocaf-runtime/actions/runs/42".into(),
                runner_image_sha256: [4; 32],
                log_sha256: [5; 32],
            },
        );
        sign(&mut r, &keys.1);
        assert_eq!(
            gate.verify(&r, &DenyExternalEvidence),
            Err(EvidenceError::MissingAuthority)
        );
    }
}
