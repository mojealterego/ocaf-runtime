//! Certificate-private-key proof of possession for a pre-pinned RPC peer.
//!
//! This is application-layer certificate proof, NOT Zenoh TLS exporter/channel
//! binding. Operators must separately verify certificate chain, purpose,
//! revocation and transport peer identity; a certificate alone is not trusted.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Public};
use openssl::sign::{Signer, Verifier};
use openssl::x509::X509;
use sha2::{Digest, Sha256};

use super::{RpcError, RpcRequest};

const PROOF_DOMAIN: &[u8] = b"OCAF-RPC-CERT-POP-V1\0";
const MAX_CERT_BYTES: usize = 16 * 1024;
const MAX_PRIVATE_KEY_BYTES: usize = 16 * 1024;
const MAX_SIGNATURE_BASE64: usize = 8192;

#[derive(Clone)]
pub(crate) struct PinnedCertificate {
    public_key: PKey<Public>,
}

impl PinnedCertificate {
    pub(crate) fn from_pem(cert_pem: &[u8], sha256_pin: &str) -> Result<Self, RpcError> {
        if cert_pem.is_empty()
            || cert_pem.len() > MAX_CERT_BYTES
            || sha256_pin.len() != 64
            || !sha256_pin.bytes().all(|b| b.is_ascii_hexdigit())
            || sha256_pin.bytes().any(|b| b.is_ascii_uppercase())
        {
            return Err(RpcError::Malformed);
        }
        let cert = X509::from_pem(cert_pem).map_err(|_| RpcError::Malformed)?;
        let der = cert.to_der().map_err(|_| RpcError::Malformed)?;
        let calculated = format!("{:x}", Sha256::digest(&der));
        if calculated != sha256_pin {
            return Err(RpcError::Unauthorized);
        }
        let public_key = cert.public_key().map_err(|_| RpcError::Malformed)?;
        // Support only signer algorithms with conventional SHA-256 digest.
        use openssl::pkey::Id;
        if !matches!(public_key.id(), Id::EC | Id::RSA) {
            return Err(RpcError::Malformed);
        }
        Ok(Self { public_key })
    }

    pub(crate) fn verify(&self, request: &RpcRequest) -> Result<(), RpcError> {
        let signature = request
            .certificate_proof_b64
            .as_deref()
            .ok_or(RpcError::Unauthorized)?;
        if signature.is_empty() || signature.len() > MAX_SIGNATURE_BASE64 {
            return Err(RpcError::Unauthorized);
        }
        let raw = B64.decode(signature).map_err(|_| RpcError::Unauthorized)?;
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.public_key)
            .map_err(|_| RpcError::Unauthorized)?;
        verifier
            .update(&request.proof_payload()?)
            .map_err(|_| RpcError::Unauthorized)?;
        if !verifier.verify(&raw).map_err(|_| RpcError::Unauthorized)? {
            return Err(RpcError::Unauthorized);
        }
        Ok(())
    }
}

pub(crate) fn sign_request(
    request: &RpcRequest,
    key_pem: &[u8],
) -> Result<String, RpcError> {
    if key_pem.is_empty() || key_pem.len() > MAX_PRIVATE_KEY_BYTES {
        return Err(RpcError::Malformed);
    }
    let key = PKey::private_key_from_pem(key_pem).map_err(|_| RpcError::Malformed)?;
    use openssl::pkey::Id;
    if !matches!(key.id(), Id::EC | Id::RSA) {
        return Err(RpcError::Malformed);
    }
    let mut signer =
        Signer::new(MessageDigest::sha256(), &key).map_err(|_| RpcError::Malformed)?;
    signer
        .update(&request.proof_payload()?)
        .map_err(|_| RpcError::Malformed)?;
    let signature = signer.sign_to_vec().map_err(|_| RpcError::Malformed)?;
    Ok(B64.encode(signature))
}

impl RpcRequest {
    fn proof_payload(&self) -> Result<Vec<u8>, RpcError> {
        let mut unsigned = self.clone();
        unsigned.certificate_proof_b64 = None;
        let bytes = serde_json::to_vec(&unsigned).map_err(|_| RpcError::Malformed)?;
        let mut payload = Vec::with_capacity(PROOF_DOMAIN.len() + bytes.len());
        payload.extend_from_slice(PROOF_DOMAIN);
        payload.extend_from_slice(&bytes);
        Ok(payload)
    }

    /// Certify an individual UUID/TTL/action request with the private key
    /// corresponding to an explicitly pinned peer certificate.
    pub fn with_certificate_proof(mut self, key_pem: &[u8]) -> Result<Self, RpcError> {
        self.certificate_proof_b64 = Some(sign_request(&self, key_pem)?);
        Ok(self)
    }
}
