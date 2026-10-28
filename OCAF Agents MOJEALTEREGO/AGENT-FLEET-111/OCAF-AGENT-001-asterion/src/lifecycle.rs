//! Lifecycle admission for the current, local-only package.

use serde::Serialize;

use crate::security::{AgentConfig, ReadinessEvidence, SecurityError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Lifecycle {
    Draft,
    Staged,
    Active,
}

/// Local planning is allowed in DRAFT. Installation does not grant registration
/// or certification, and ACTIVE is always denied by the OSTc gate.
pub fn request_transition(
    config: &AgentConfig,
    target: Lifecycle,
    evidence: &ReadinessEvidence,
) -> Result<Lifecycle, SecurityError> {
    config.validate()?;
    match target {
        Lifecycle::Draft => Ok(Lifecycle::Draft),
        Lifecycle::Staged => Err(SecurityError::Invalid(
            "STAGED requires external registration",
        )),
        Lifecycle::Active => {
            config.admit_active(evidence)?;
            Ok(Lifecycle::Active)
        }
    }
}
