//! Stage 0.43 OMAs signed agent admission. This is an opt-in authenticated
//! admission API; the legacy local JSON-lines STAGED-only CLI remains separate.
//!
//! The trusted Ed25519 public key and authenticated RPC sender must be bound
//! by the transport/operator configuration before invoking this method.
//! Never trust a public key supplied in the request payload.
use crate::omas_server::{AdmissionDecision, AgentRegistrationRequest, OmasServer};
use crate::omas_signed::{verify_and_consume, SignedOperation, SignedOperationError};
use std::path::Path;
use thiserror::Error;

const STAGE043_AGENT_REGISTER_OPCODE: u8 = 5;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignedAgentError {
    #[error("agent registration is not bound to the authenticated signed request")]
    Binding,
    #[error("agent registration Ed25519 signature rejected")]
    Signature,
    #[error("agent registration nonce replayed")]
    Replay,
    #[error("agent registration replay database unavailable")]
    Store,
    #[error("agent registration policy or clock bounds rejected")]
    Invalid,
}

pub fn admit_signed_registration(
    server: &OmasServer,
    request: AgentRegistrationRequest,
    operation: &SignedOperation,
    trusted_public_key: &[u8; 32],
    replay_db: &Path,
    now: i64,
) -> Result<AdmissionDecision, SignedAgentError> {
    let payload = serde_json::to_vec(&request).map_err(|_| SignedAgentError::Binding)?;
    let expected_namespace = format!("{}/agents", request.tenant);
    let expected_target = format!("{expected_namespace}/{}", request.agent_id);
    if operation.tenant != request.tenant
        || operation.namespace != expected_namespace
        || operation.target != expected_target
        || operation.opcode != STAGE043_AGENT_REGISTER_OPCODE
        || operation.rpc_sender.is_none()
        || operation.payload != payload
    {
        return Err(SignedAgentError::Binding);
    }
    // Identity, opcode, target, signed fuel, nonce and entire request payload
    // must verify before consuming a persistent replay nonce or mutating OMAs.
    verify_and_consume(operation, trusted_public_key, replay_db, now).map_err(
        |error| match error {
            SignedOperationError::Invalid => SignedAgentError::Invalid,
            SignedOperationError::Signature => SignedAgentError::Signature,
            SignedOperationError::Replay => SignedAgentError::Replay,
            SignedOperationError::Store => SignedAgentError::Store,
        },
    )?;
    Ok(server.admit_registration(request))
}
