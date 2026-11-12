//! In-process replay and resource budget for a trusted authenticated session.
//! NOT durable across restarts, NOT a distributed replay authority. Transport
//! must authenticate sequence numbers and bind session identity to the channel.
use crate::trs_digital_life::{dispatch_authorized, AuthenticatedPrincipal, BridgeError, BridgeRequest, BridgeResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError { InvalidConfig, Replay, Exhausted, Bridge(BridgeError) }

pub struct AuthorizedSession {
    principal: AuthenticatedPrincipal,
    highest_sequence: u64,
    remaining: u64,
}
impl AuthorizedSession {
    pub fn new(principal: AuthenticatedPrincipal, max_requests: u64) -> Result<Self, SessionError> {
        if max_requests == 0 || max_requests > 1_000_000 {
            return Err(SessionError::InvalidConfig);
        }
        Ok(Self { principal, highest_sequence: 0, remaining: max_requests })
    }
    pub fn dispatch(&mut self, sequence: u64, request: &BridgeRequest) -> Result<BridgeResponse, SessionError> {
        if self.remaining == 0 { return Err(SessionError::Exhausted); }
        if sequence == 0 || sequence <= self.highest_sequence {
            return Err(SessionError::Replay);
        }
        // Reject invalid/unauthorized requests without consuming a sequence.
        let response = dispatch_authorized(&self.principal, request).map_err(SessionError::Bridge)?;
        self.highest_sequence = sequence;
        self.remaining -= 1;
        Ok(response)
    }
    pub fn remaining(&self) -> u64 { self.remaining }
}
