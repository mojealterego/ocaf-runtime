//! Operator-signed WASI admission contracts.
//!
//! The manifest is authenticated with a detached Ed25519 signature over the
//! *exact* UTF-8 JSON bytes, prefixed by a domain-separation label. The
//! trust-anchor PEM must be pinned by the operator, not submitted by a peer.
use std::time::{SystemTime, UNIX_EPOCH};

use openssl::pkey::{Id, PKey};
use openssl::sign::Verifier;
use serde::{Deserialize, Serialize};

use super::{rejected, SandboxPolicy};
use crate::executor::ToolError;
use crate::registry::ToolManifest;

const MANIFEST_DOMAIN: &[u8] = b"OCAF-WASI-MANIFEST-V1\0";
const MAX_MANIFEST_BYTES: usize = 24 * 1024;
const MAX_PUBLIC_KEY_BYTES: usize = 2048;
const MAX_SIGNATURE_BYTES: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedWasiManifest {
    pub schema_version: u8,
    pub tenant_id: String,
    pub agent_id: String,
    pub tool: ToolManifest,
    pub module_sha256_hex: String,
    pub expires_unix_secs: u64,
    pub policy: SandboxPolicy,
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || b"_-.".contains(&ch))
}

impl SignedWasiManifest {
    pub(super) fn validate(&self, module_sha256: [u8; 32]) -> Result<(), ToolError> {
        if self.schema_version != 1
            || !valid_identity(&self.tenant_id)
            || !valid_identity(&self.agent_id)
            || !valid_identity(&self.tool.name)
            || !self.tool.enabled
            || self.tool.timeout_ms == 0
            || self.tool.timeout_ms > self.policy.timeout_ms
            || !self.tool.permissions.is_empty()
            || self.tool.cost_sats != 0
            || !self.module_sha256_hex.bytes().all(|b| b.is_ascii_hexdigit())
            || self.module_sha256_hex.len() != 64
            || self.module_sha256_hex.bytes().any(|b| b.is_ascii_uppercase())
            || self.module_sha256_hex != hex_sha256(module_sha256)
        {
            return Err(rejected("signed WASI manifest policy rejected"));
        }
        self.policy.validate()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| rejected("system clock is unavailable"))?
            .as_secs();
        if self.expires_unix_secs <= now
            || self.expires_unix_secs.saturating_sub(now) > 31 * 24 * 60 * 60
        {
            return Err(rejected("signed WASI manifest expiry rejected"));
        }
        Ok(())
    }

    pub fn principal(&self) -> String {
        format!("{}:{}", self.tenant_id, self.agent_id)
    }
}

fn hex_sha256(bytes: [u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut result = String::with_capacity(64);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").expect("format to String");
    }
    result
}

/// Verify a detached signature and a time-bounded, immutable module/policy
/// binding. The tool manifest is part of the signed payload, so the caller
/// must register exactly the verified tool definition.
pub fn verify_signed_manifest(
    json: &[u8],
    detached_signature: &[u8],
    trusted_public_key_pem: &[u8],
    approved_module_sha256: [u8; 32],
) -> Result<SignedWasiManifest, ToolError> {
    if json.is_empty()
        || json.len() > MAX_MANIFEST_BYTES
        || detached_signature.len() != 64
        || trusted_public_key_pem.is_empty()
        || trusted_public_key_pem.len() > MAX_PUBLIC_KEY_BYTES
    {
        return Err(rejected("WASI manifest signature envelope rejected"));
    }
    let public = PKey::public_key_from_pem(trusted_public_key_pem)
        .map_err(|_| rejected("invalid WASI publisher trust anchor"))?;
    if public.id() != Id::ED25519 {
        return Err(rejected("WASI publisher must use Ed25519"));
    }
    let mut verifier = Verifier::new_without_digest(&public)
        .map_err(|_| rejected("cannot initialize WASI signature verification"))?;
    let mut payload = Vec::with_capacity(MANIFEST_DOMAIN.len() + json.len());
    payload.extend_from_slice(MANIFEST_DOMAIN);
    payload.extend_from_slice(json);
    let ok = verifier
        .verify_oneshot(detached_signature, &payload)
        .map_err(|_| rejected("invalid WASI signature"))?;
    if !ok {
        return Err(rejected("WASI manifest signature mismatch"));
    }
    let manifest: SignedWasiManifest =
        serde_json::from_slice(json).map_err(|_| rejected("invalid signed WASI manifest JSON"))?;
    manifest.validate(approved_module_sha256)?;
    Ok(manifest)
}

/// Test or operator signing helper. Private key material is never stored
/// in a manifest or this runtime's executor.
pub fn manifest_signing_payload(json: &[u8]) -> Result<Vec<u8>, ToolError> {
    if json.is_empty() || json.len() > MAX_MANIFEST_BYTES {
        return Err(rejected("invalid WASI signing payload length"));
    }
    let mut payload = Vec::with_capacity(MANIFEST_DOMAIN.len() + json.len());
    payload.extend_from_slice(MANIFEST_DOMAIN);
    payload.extend_from_slice(json);
    Ok(payload)
}
