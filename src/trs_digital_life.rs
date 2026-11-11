//! Versioned data-only contract for eventual integration into Digital-Life.
//! Does not execute requests or grant authority. Admission and dispatch
//! MUST occur in a separately authenticated, explicitly authorized adapter.
use serde::{Deserialize, Serialize};

pub const DIGITAL_LIFE_BRIDGE_VERSION: u16 = 1;
const MAX_PAYLOAD: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeError {
    UnsupportedVersion,
    InvalidIdentity,
    ForbiddenOperation,
    PayloadTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeRequest {
    pub version: u16,
    pub tenant: String,
    pub sender: String,
    pub operation: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeResponse {
    pub version: u16,
    pub accepted: bool,
    pub payload: Vec<u8>,
}

pub enum Admission { Denied, Accepted }

fn valid_identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

impl BridgeRequest {
    /// Syntactic admission only. Never substitute this for transport
    /// authentication, authorization, replay prevention or proof checking.
    pub fn validate(&self) -> Result<(), BridgeError> {
        if self.version != DIGITAL_LIFE_BRIDGE_VERSION {
            return Err(BridgeError::UnsupportedVersion);
        }
        if !valid_identity(&self.tenant) || !valid_identity(&self.sender) {
            return Err(BridgeError::InvalidIdentity);
        }
        if self.payload.len() > MAX_PAYLOAD {
            return Err(BridgeError::PayloadTooLarge);
        }
        match self.operation.as_str() {
            "trs.geometry.fisher_rao" | "trs.geometry.hodge_laplacian"
            | "trs.math.sat_evaluate" | "trs.health" => Ok(()),
            _ => Err(BridgeError::ForbiddenOperation),
        }
    }
}
impl BridgeResponse {
    pub fn accepted(payload: Vec<u8>) -> Self {
        Self { version: DIGITAL_LIFE_BRIDGE_VERSION, accepted: true, payload }
    }
    pub fn rejected() -> Self {
        Self { version: DIGITAL_LIFE_BRIDGE_VERSION, accepted: false, payload: Vec::new() }
    }
}
