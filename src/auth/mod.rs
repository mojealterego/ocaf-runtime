use serde::{Deserialize, Serialize};
use std::fmt;

/// Credential material is sensitive. The runtime never logs its contents.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthContext {
    pub macaroon: String,
    pub preimage: String,
    pub invoice: String,
    pub nonce: String,
}

impl fmt::Debug for AuthContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthContext")
            .field("macaroon", &"[REDACTED]")
            .field("preimage", &"[REDACTED]")
            .field("invoice", &"[REDACTED]")
            .field("nonce", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("credential is required")]
    MissingCredential,
    #[error("no verified authorization provider is configured")]
    VerificationUnavailable,
    #[error("authorization was rejected")]
    Denied,
}

/// Production implementations MUST verify macaroon signatures, payment hashes,
/// invoice/payment context, caveats and replay policy before returning Ok(()).
/// The built-in verifier deliberately does not accept paid or permissioned calls.
#[async_trait::async_trait]
pub trait AuthVerifier: Send + Sync {
    async fn verify(
        &self,
        tool_name: &str,
        credential: &AuthContext,
        required_cost_sats: u64,
        permissions: &[String],
    ) -> Result<(), AuthError>;
}

pub struct DenyByDefaultVerifier;

#[async_trait::async_trait]
impl AuthVerifier for DenyByDefaultVerifier {
    async fn verify(
        &self,
        _tool_name: &str,
        _credential: &AuthContext,
        _required_cost_sats: u64,
        _permissions: &[String],
    ) -> Result<(), AuthError> {
        Err(AuthError::VerificationUnavailable)
    }
}

pub struct AuthPolicy;

impl AuthPolicy {
    pub async fn authorize(
        tool_name: &str,
        required_cost_sats: u64,
        permissions: &[String],
        auth: Option<&AuthContext>,
        verifier: &dyn AuthVerifier,
    ) -> Result<(), AuthError> {
        if required_cost_sats == 0 && permissions.is_empty() {
            return Ok(());
        }

        let credential = auth.ok_or(AuthError::MissingCredential)?;
        verifier
            .verify(tool_name, credential, required_cost_sats, permissions)
            .await
    }
}
