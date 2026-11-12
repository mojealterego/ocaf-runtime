//! Durable TRS-SEAL admission for Digital-Life bridge requests.
//! HMAC authenticates integrity but does not encrypt payloads. Caller must
//! use an independently authenticated, encrypted transport and trusted epoch.
use crate::trs_digital_life::{dispatch_authorized, AuthenticatedPrincipal, BridgeError, BridgeRequest, BridgeResponse};
use crate::trs_seal::{Envelope, SealKeys};
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
/// Admission is committed before computation; a crash after admission can
/// consume the request without a result (at-most-once, not exactly-once).
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
    verify_and_record_authorized(conn, keys, &principal.tenant, &principal.sender, envelope)
        .map_err(DurableBridgeError::Admission)?;
    dispatch_authorized(principal, &request).map_err(DurableBridgeError::Bridge)
}
