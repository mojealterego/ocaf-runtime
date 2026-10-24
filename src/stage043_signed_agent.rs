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

/// Admission boundary for a transport that has independently authenticated
/// the peer identity. The sender string MUST come from the transport session,
/// not from client JSON or the signed envelope.
pub fn admit_signed_registration_for_sender(
    server: &OmasServer,
    request: AgentRegistrationRequest,
    operation: &SignedOperation,
    authenticated_rpc_sender: &str,
    trusted_public_key: &[u8; 32],
    replay_db: &Path,
    now: i64,
) -> Result<AdmissionDecision, SignedAgentError> {
    if authenticated_rpc_sender.is_empty()
        || operation.rpc_sender.as_deref() != Some(authenticated_rpc_sender)
    {
        return Err(SignedAgentError::Binding);
    }
    admit_signed_registration(
        server,
        request,
        operation,
        trusted_public_key,
        replay_db,
        now,
    )
}

/// Signed, transport-principal-bound admission into the durable OMAs registry.
/// The caller MUST derive `authenticated_rpc_sender` from an authenticated
/// transport session, never from the untrusted request body.
///
/// The replay nonce is committed before the durable agent registration. A
/// subsequent store failure therefore fails closed and requires a fresh signed
/// request with a new nonce. This is deliberately not a distributed transaction.
pub fn admit_signed_registration_persistent_for_sender(
    store: &crate::stage043_agent_store::Stage043AgentStore,
    request: AgentRegistrationRequest,
    operation: &SignedOperation,
    authenticated_rpc_sender: &str,
    trusted_public_key: &[u8; 32],
    replay_db: &Path,
    now: i64,
) -> Result<AdmissionDecision, SignedAgentError> {
    let validator = OmasServer::new(store.config().clone()).map_err(|_| SignedAgentError::Invalid)?;
    let preliminary = admit_signed_registration_for_sender(
        &validator,
        request.clone(),
        operation,
        authenticated_rpc_sender,
        trusted_public_key,
        replay_db,
        now,
    )?;
    if !preliminary.admitted {
        return Ok(preliminary);
    }
    store.admit(request).map_err(|_| SignedAgentError::Store)
}
