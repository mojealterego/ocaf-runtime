//! Stage 0.43: authenticated, tenant-bound OMAt tool manifest admission.
//!
//! Signature verification occurs before any registry mutation. The caller must
//! obtain the expected tenant and the Ed25519 public key from an independently
//! authenticated, operator-managed trust store; never from the manifest itself.
use crate::omat_server::{OmatError, OmatRegistry, ToolManifest};
use openssl::pkey::{Id, PKey};
use openssl::sign::Verifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_SIGNED_MANIFEST_BYTES: usize = 1_048_576;
const MAX_MANIFEST_LIFETIME_SECONDS: i64 = 300;
const DOMAIN: &[u8] = b"OCAF-OMAT-SIGNED-TOOL-MANIFEST-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedToolManifest {
    pub manifest: ToolManifest,
    pub signer_id: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SignedManifestError {
    #[error("tenant does not match the authenticated trust anchor")]
    TenantMismatch,
    #[error("invalid manifest signing time window")]
    TimeWindow,
    #[error("invalid signer identifier or oversized envelope")]
    InvalidEnvelope,
    #[error("signed manifest verification failed")]
    Signature,
    #[error("signed manifest serialization failed")]
    Serialization,
    #[error("tool manifest rejected: {0}")]
    Registry(#[from] OmatError),
}

impl SignedToolManifest {
    /// Versioned canonical signature input, independent of JSON field order
    /// in the envelope; all policy-relevant manifest fields are signed.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, SignedManifestError> {
        let manifest =
            serde_json::to_vec(&self.manifest).map_err(|_| SignedManifestError::Serialization)?;
        if manifest.len() > MAX_SIGNED_MANIFEST_BYTES
            || self.signer_id.is_empty()
            || self.signer_id.len() > 128
            || !self
                .signer_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        {
            return Err(SignedManifestError::InvalidEnvelope);
        }
        let mut out = Vec::with_capacity(DOMAIN.len() + manifest.len() + 180);
        out.extend_from_slice(DOMAIN);
        append_field(&mut out, self.signer_id.as_bytes());
        append_field(&mut out, &manifest);
        out.extend_from_slice(&self.issued_at.to_be_bytes());
        out.extend_from_slice(&self.expires_at.to_be_bytes());
        Ok(out)
    }
}

fn append_field(destination: &mut Vec<u8>, data: &[u8]) {
    destination.extend_from_slice(&(data.len() as u64).to_be_bytes());
    destination.extend_from_slice(data);
}

/// Fail closed if any untrusted data, signature or time bound disagrees.
/// The trusted_tenant/trusted_key values must be supplied by a server-owned
/// principal mapping, never by the client-provided signed envelope.
pub fn register_signed_manifest(
    registry: &mut OmatRegistry,
    envelope: &SignedToolManifest,
    trusted_tenant: &str,
    trusted_key: &[u8; 32],
    now: i64,
) -> Result<String, SignedManifestError> {
    if envelope.manifest.tenant_id != trusted_tenant {
        return Err(SignedManifestError::TenantMismatch);
    }
    let lifetime = envelope
        .expires_at
        .checked_sub(envelope.issued_at)
        .ok_or(SignedManifestError::TimeWindow)?;
    if lifetime <= 0
        || lifetime > MAX_MANIFEST_LIFETIME_SECONDS
        || now < envelope.issued_at
        || now >= envelope.expires_at
    {
        return Err(SignedManifestError::TimeWindow);
    }
    if envelope.signature.len() != 64 {
        return Err(SignedManifestError::Signature);
    }
    let canonical = envelope.canonical_bytes()?;
    let key = PKey::public_key_from_raw_bytes(trusted_key, Id::ED25519)
        .map_err(|_| SignedManifestError::Signature)?;
    let mut verifier =
        Verifier::new_without_digest(&key).map_err(|_| SignedManifestError::Signature)?;
    if !verifier
        .verify_oneshot(&envelope.signature, &canonical)
        .map_err(|_| SignedManifestError::Signature)?
    {
        return Err(SignedManifestError::Signature);
    }
    // Registry uniqueness/capacity/schema checks occur only after auth.
    registry
        .register(envelope.manifest.clone())
        .map_err(SignedManifestError::Registry)
}
