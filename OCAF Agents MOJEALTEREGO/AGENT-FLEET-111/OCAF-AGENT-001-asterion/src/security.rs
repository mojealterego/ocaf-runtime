//! Local configuration and immutable fail-closed gates.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{AGENT_ID, DISPLAY_NAME, GROUP_ID, SPECIALIZATION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    pub endpoint: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    pub agent_id: String,
    pub display_name: String,
    pub package_name: String,
    pub group_id: String,
    pub specialization: String,
    pub lifecycle: String,
    pub deployment_status: String,
    pub autonomy_mode: String,
    pub model_backend: String,
    pub memory_namespace: String,
    pub transport: String,
    pub ostc_enabled: bool,
    pub source_access: String,
    pub active_admission: String,
    pub max_steps: usize,
    pub max_input_bytes: usize,
    pub omas: ServiceConfig,
    pub omat: ServiceConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadinessEvidence {
    pub tests_passed: bool,
    pub signature_verified: bool,
    pub omas_ready: bool,
    pub omat_ready: bool,
    pub ostc_ready: bool,
    pub source_access_granted: bool,
}

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("invalid agent configuration: {0}")]
    Invalid(&'static str),
    #[error("invalid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("ACTIVE is denied while OSTc is disabled")]
    ActiveDenied,
}

impl AgentConfig {
    pub fn parse(source: &str) -> Result<Self, SecurityError> {
        let config: Self = toml::from_str(source)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), SecurityError> {
        if self.agent_id != AGENT_ID
            || self.display_name != DISPLAY_NAME
            || self.package_name != "ocaf-agent-001-asterion"
            || self.group_id != GROUP_ID
            || self.specialization != SPECIALIZATION
        {
            return Err(SecurityError::Invalid("identity"));
        }
        if self.lifecycle != "DRAFT" || self.deployment_status != "NOT_DEPLOYED" {
            return Err(SecurityError::Invalid("unverified lifecycle"));
        }
        if self.autonomy_mode != "bounded-local-stage-assistance"
            || self.model_backend != "UNASSIGNED"
            || self.memory_namespace != "ocaf-asterion-state"
        {
            return Err(SecurityError::Invalid("unsupported autonomy backend"));
        }
        if self.transport != "zenoh" {
            return Err(SecurityError::Invalid("transport"));
        }
        if self.ostc_enabled || self.source_access != "disabled" {
            return Err(SecurityError::Invalid(
                "OSTc and source access must remain disabled",
            ));
        }
        if self.active_admission != "DENY" {
            return Err(SecurityError::Invalid("ACTIVE admission must remain DENY"));
        }
        if !(1..=32).contains(&self.max_steps) {
            return Err(SecurityError::Invalid("max_steps must be 1..=32"));
        }
        if !(1..=1_048_576).contains(&self.max_input_bytes) {
            return Err(SecurityError::Invalid(
                "max_input_bytes must be 1..=1048576",
            ));
        }
        if self.omas.endpoint.is_empty()
            || self.omat.endpoint.is_empty()
            || self.omas.endpoint.len() > 256
            || self.omat.endpoint.len() > 256
            || self.omas.endpoint == self.omat.endpoint
        {
            return Err(SecurityError::Invalid(
                "OMAs and OMAt need separate endpoints",
            ));
        }
        if self.omas.enabled || self.omat.enabled {
            return Err(SecurityError::Invalid(
                "remote services are unavailable in this package",
            ));
        }
        Ok(())
    }

    /// Caller-provided readiness is never an authority to enable OSTc.
    pub fn admit_active(&self, _evidence: &ReadinessEvidence) -> Result<(), SecurityError> {
        self.validate()?;
        Err(SecurityError::ActiveDenied)
    }
}
