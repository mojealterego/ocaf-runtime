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
    InvalidPayload,
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

/// Pure local mathematical dispatch; never call directly on untrusted network
/// input without independent transport identity and capability authorization.
/// Every operation has a bounded, strictly typed payload schema.
pub fn dispatch_local(request: &BridgeRequest) -> Result<BridgeResponse, BridgeError> {
    request.validate()?;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FisherInput { p: Vec<f64>, q: Vec<f64> }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct HodgeInput { values: Vec<f64>, edges: Vec<(usize, usize)> }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SatInput { variables: usize, clauses: Vec<[i32; 3]> }
    let payload = match request.operation.as_str() {
        "trs.health" => {
            if !request.payload.is_empty() && request.payload != b"{}" {
                return Err(BridgeError::InvalidPayload);
            }
            serde_json::json!({"status":"ready","version":DIGITAL_LIFE_BRIDGE_VERSION})
        }
        "trs.geometry.fisher_rao" => {
            let input: FisherInput = serde_json::from_slice(&request.payload)
                .map_err(|_| BridgeError::InvalidPayload)?;
            if input.p.is_empty() || input.p.len() > 4096 || input.q.len() != input.p.len() {
                return Err(BridgeError::InvalidPayload);
            }
            let distance = crate::trs_geometry::fisher_rao_distance(&input.p, &input.q)
                .map_err(|_| BridgeError::InvalidPayload)?;
            serde_json::json!({"distance":distance})
        }
        "trs.geometry.hodge_laplacian" => {
            let input: HodgeInput = serde_json::from_slice(&request.payload)
                .map_err(|_| BridgeError::InvalidPayload)?;
            if input.values.is_empty() || input.values.len() > 4096 || input.edges.len() > 8192 {
                return Err(BridgeError::InvalidPayload);
            }
            let values = crate::trs_geometry::hodge_zero_laplacian(&input.values, &input.edges)
                .map_err(|_| BridgeError::InvalidPayload)?;
            serde_json::json!({"values":values})
        }
        "trs.math.sat_evaluate" => {
            let input: SatInput = serde_json::from_slice(&request.payload)
                .map_err(|_| BridgeError::InvalidPayload)?;
            // Enumerating assignments is exponential. Cap to 2^16.
            if input.variables == 0 || input.variables > 16 || input.clauses.len() > 4096 {
                return Err(BridgeError::InvalidPayload);
            }
            let clauses: Vec<crate::trs_math::Clause3> =
                input.clauses.into_iter().map(crate::trs_math::Clause3).collect();
            let result = crate::trs_math::enumerate_3sat(input.variables, &clauses)
                .map_err(|_| BridgeError::InvalidPayload)?;
            serde_json::json!({
                "minimum_energy":result.minimum_energy,
                "satisfying_count":result.satisfying_count,
                "minimizers":result.minimizers
            })
        }
        _ => return Err(BridgeError::ForbiddenOperation),
    };
    let encoded = serde_json::to_vec(&payload).map_err(|_| BridgeError::InvalidPayload)?;
    if encoded.len() > MAX_PAYLOAD {
        return Err(BridgeError::PayloadTooLarge);
    }
    Ok(BridgeResponse::accepted(encoded))
}
