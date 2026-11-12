//! Durable TRS-SEAL admission for Digital-Life bridge requests.
//! HMAC authenticates integrity but does not encrypt payloads. Caller must
//! use an independently authenticated, encrypted transport and trusted epoch.
use crate::trs_digital_life::{dispatch_authorized, AuthenticatedPrincipal, BridgeError, BridgeRequest, BridgeResponse};
use crate::trs_seal::{Envelope, SealKeys, SealVerifier};
use crate::trs_seal_sqlite::{verify_and_record_authorized, DurableSealError};
use rusqlite::Connection;

#[derive(Debug)]
pub enum DurableBridgeError {
    Unauthorized,
    InvalidPayload,
    Bridge(BridgeError),
    Admission(DurableSealError),
}

/// The envelope carries the *entire* JSON request, authenticated by TRS-SEAL.
/// Pure computation is preflighted before durable admission. A crash after
/// admission can still lose the result (at-most-once, not exactly-once).
pub fn dispatch_sealed(
    conn: &mut Connection,
    keys: &SealKeys,
    principal: &AuthenticatedPrincipal,
    envelope: &Envelope,
) -> Result<BridgeResponse, DurableBridgeError> {
    if envelope.tenant != principal.tenant || envelope.sender != principal.sender {
        return Err(DurableBridgeError::Unauthorized);
    }
    if envelope.payload.len() > 70_000 {
        return Err(DurableBridgeError::InvalidPayload);
    }
    let request: BridgeRequest = serde_json::from_slice(&envelope.payload)
        .map_err(|_| DurableBridgeError::InvalidPayload)?;
    request.validate().map_err(DurableBridgeError::Bridge)?;
    if request.tenant != envelope.tenant || request.sender != envelope.sender {
        return Err(DurableBridgeError::Unauthorized);
    }
    if !principal.allowed_operations.iter().any(|op| op == &request.operation) {
        return Err(DurableBridgeError::Unauthorized);
    }
    // Authenticate the envelope before potentially expensive mathematical work.
    // This preliminary check is NOT the authoritative fencing/replay decision.
    let mut verifier = SealVerifier::new(envelope.fencing_epoch, 1)
        .map_err(|e| DurableBridgeError::Admission(DurableSealError::Seal(e)))?;
    verifier.verify(keys, envelope)
        .map_err(|e| DurableBridgeError::Admission(DurableSealError::Seal(e)))?;
    // Pure computations are preflighted so malformed mathematical input does
    // not burn a durable replay slot. No side effects are permitted here.
    let response = dispatch_authorized(principal, &request)
        .map_err(DurableBridgeError::Bridge)?;
    // SQLite pinned fencing epoch and replay uniqueness are authoritative.
    verify_and_record_authorized(conn, keys, &principal.tenant, &principal.sender, envelope)
        .map_err(DurableBridgeError::Admission)?;
    Ok(response)
}
