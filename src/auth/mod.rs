use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthContext {
    pub macaroon: String,
    pub preimage: String,
    pub invoice: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L402Challenge {
    pub payment_required: bool,
    pub invoice: String,
    pub cost_sats: u64,
    pub macaroon: String,
    pub nonce: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("payment required for tool: {0}")]
    PaymentRequired(String),
    #[error("invalid macaroon token")]
    InvalidMacaroon,
    #[error("invalid preimage")]
    InvalidPreimage,
    #[error("challenge rejected")]
    ChallengeRejected,
}

pub struct AuthPolicy;

impl AuthPolicy {
    pub fn challenge_for(tool_name: &str, cost_sats: u64) -> L402Challenge {
        L402Challenge {
            payment_required: true,
            invoice: format!("lnbc_{}_invoice", tool_name),
            cost_sats,
            macaroon: format!("macaroon_{}_token", tool_name),
            nonce: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn validate(
        tool_name: &str,
        auth: Option<&AuthContext>,
        required_cost: u64,
    ) -> Result<(), AuthError> {
        if auth.is_none() && required_cost > 0 {
            return Err(AuthError::PaymentRequired(tool_name.to_string()));
        }

        let auth = auth.ok_or_else(|| AuthError::PaymentRequired(tool_name.to_string()))?;

        if auth.macaroon.trim().is_empty() {
            return Err(AuthError::InvalidMacaroon);
        }

        if auth.preimage.trim().is_empty() {
            return Err(AuthError::InvalidPreimage);
        }

        if auth.invoice.trim().is_empty() || auth.nonce.trim().is_empty() {
            return Err(AuthError::ChallengeRejected);
        }

        if required_cost == 0 {
            return Ok(());
        }

        Ok(())
    }
}
