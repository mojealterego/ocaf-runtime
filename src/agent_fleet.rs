//! Individual agent and 33-swarm admission contracts.
//!
//! This module owns the common shape used by the 111 independent agent
//! packages.  It deliberately does not start a model, grant source access or
//! turn an assertion into readiness.  OMAs and OMAt remain separate services,
//! Zenoh remains transport, and OSTc is a future capability.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const REQUIRED_AGENT_COUNT: usize = 111;
pub const REQUIRED_SWARM_COUNT: usize = 33;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentLifecycle {
    Draft,
    Validated,
    Built,
    Tested,
    Signed,
    Registered,
    Staged,
    Active,
    Quarantined,
}

#[cfg(test)]
impl AgentLifecycle {
    fn rank(self) -> u8 {
        match self {
            Self::Draft => 0,
            Self::Validated => 1,
            Self::Built => 2,
            Self::Tested => 3,
            Self::Signed => 4,
            Self::Registered => 5,
            Self::Staged => 6,
            Self::Active => 7,
            Self::Quarantined => 255,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIdentity {
    pub agent_id: String,
    /// Human-facing original name. The technical agent_id remains immutable
    /// for routing and evidence references.
    pub display_name: String,
    pub swarm_id: String,
    pub specialization: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutonomyContract {
    pub model_backend: String,
    pub memory_namespace: String,
    pub planner: String,
    pub human_approval_for_side_effects: bool,
    pub max_steps: u64,
    pub max_cost_minor: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentManifest {
    pub identity: AgentIdentity,
    pub lifecycle: AgentLifecycle,
    pub autonomy: AutonomyContract,
    pub omas_endpoint: String,
    pub omat_endpoint: String,
    pub transport: String,
    pub ostc_enabled: bool,
    pub source_access: String,
    pub allowed_tools: BTreeSet<String>,
    pub artifact_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentEvidence {
    pub declared_state: AgentLifecycle,
    pub source_commit: String,
    pub artifact_digest: String,
    pub tests_passed: bool,
    pub signature_verified: bool,
    pub omas_ready: bool,
    pub omat_ready: bool,
    pub ostc_ready: bool,
    pub source_access_granted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentAdmissionError {
    #[error("agent identity is invalid: {0}")]
    InvalidIdentity(&'static str),
    #[error("agent must use Zenoh transport")]
    InvalidTransport,
    #[error("OMAs and OMAt endpoints must be separate")]
    SharedServiceEndpoint,
    #[error("OSTc must remain disabled in this release")]
    OstcEnabled,
    #[error("source access must remain disabled while OSTc is disabled")]
    SourceAccessEnabled,
    #[error("agent lifecycle is quarantined")]
    Quarantined,
    #[error("ACTIVE is denied while OSTc is disabled")]
    OstcDisabled,
    #[error("source access was not granted by OSTc")]
    SourceAccessDenied,
    #[error("readiness evidence declared an unsupported state")]
    FalseReadiness,
    #[error("artifact digest is missing or does not match the manifest")]
    ArtifactMismatch,
    #[error("required build evidence is incomplete")]
    IncompleteEvidence,
}

impl AgentManifest {
    pub fn validate(&self) -> Result<(), AgentAdmissionError> {
        let id = &self.identity.agent_id;
        if !valid_component(id, "OCAF-AGENT-") {
            return Err(AgentAdmissionError::InvalidIdentity("agent_id"));
        }
        if !valid_component(&self.identity.swarm_id, "G") || self.identity.swarm_id.len() != 3 {
            return Err(AgentAdmissionError::InvalidIdentity("swarm_id"));
        }
        if self.identity.display_name.trim().is_empty()
            || self.identity.specialization.trim().is_empty()
            || self.identity.version.trim().is_empty()
        {
            return Err(AgentAdmissionError::InvalidIdentity(
                "specialization/version",
            ));
        }
        if self.transport != "zenoh" {
            return Err(AgentAdmissionError::InvalidTransport);
        }
        if self.omas_endpoint == self.omat_endpoint {
            return Err(AgentAdmissionError::SharedServiceEndpoint);
        }
        if self.ostc_enabled {
            return Err(AgentAdmissionError::OstcEnabled);
        }
        if self.source_access != "disabled" {
            return Err(AgentAdmissionError::SourceAccessEnabled);
        }
        if self.lifecycle == AgentLifecycle::Quarantined {
            return Err(AgentAdmissionError::Quarantined);
        }
        Ok(())
    }

    /// Fail-closed ACTIVE admission.  Caller supplied readiness is evidence
    /// only after the manifest's OSTc and source-access gates have passed.
    pub fn admit_active(&self, evidence: &AgentEvidence) -> Result<(), AgentAdmissionError> {
        self.validate()?;
        if !self.ostc_enabled {
            return Err(AgentAdmissionError::OstcDisabled);
        }
        if self.source_access != "granted" || !evidence.source_access_granted {
            return Err(AgentAdmissionError::SourceAccessDenied);
        }
        if evidence.declared_state != AgentLifecycle::Active
            || self.lifecycle != AgentLifecycle::Active
        {
            return Err(AgentAdmissionError::FalseReadiness);
        }
        let digest = self
            .artifact_digest
            .as_deref()
            .ok_or(AgentAdmissionError::ArtifactMismatch)?;
        if evidence.artifact_digest != digest || evidence.source_commit.trim().is_empty() {
            return Err(AgentAdmissionError::ArtifactMismatch);
        }
        if !evidence.tests_passed
            || !evidence.signature_verified
            || !evidence.omas_ready
            || !evidence.omat_ready
            || !evidence.ostc_ready
        {
            return Err(AgentAdmissionError::IncompleteEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentFleetManifest {
    pub fleet_id: String,
    pub swarm_count: usize,
    pub agent_count: usize,
    pub agents: Vec<AgentManifest>,
}

impl AgentFleetManifest {
    pub fn validate(&self) -> Result<(), FleetValidationError> {
        if self.swarm_count != REQUIRED_SWARM_COUNT {
            return Err(FleetValidationError::SwarmCount(self.swarm_count));
        }
        if self.agent_count != REQUIRED_AGENT_COUNT || self.agents.len() != REQUIRED_AGENT_COUNT {
            return Err(FleetValidationError::AgentCount(self.agents.len()));
        }
        let mut ids = BTreeSet::new();
        let mut swarms = BTreeSet::new();
        for agent in &self.agents {
            agent.validate().map_err(FleetValidationError::Agent)?;
            if !ids.insert(agent.identity.agent_id.clone()) {
                return Err(FleetValidationError::DuplicateAgent(
                    agent.identity.agent_id.clone(),
                ));
            }
            swarms.insert(agent.identity.swarm_id.clone());
        }
        if swarms.len() != REQUIRED_SWARM_COUNT {
            return Err(FleetValidationError::DistinctSwarmCount(swarms.len()));
        }
        Ok(())
    }

    pub fn digest(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("fleet manifest serialization is infallible");
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    pub fn by_id(&self) -> BTreeMap<&str, &AgentManifest> {
        self.agents
            .iter()
            .map(|agent| (agent.identity.agent_id.as_str(), agent))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FleetValidationError {
    #[error("fleet requires exactly 33 swarms, got {0}")]
    SwarmCount(usize),
    #[error("fleet requires exactly 111 agents, got {0}")]
    AgentCount(usize),
    #[error("duplicate agent: {0}")]
    DuplicateAgent(String),
    #[error("agent invalid: {0}")]
    Agent(#[source] AgentAdmissionError),
    #[error("fleet must contain 33 distinct swarms, got {0}")]
    DistinctSwarmCount(usize),
}

fn valid_component(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() > prefix.len()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(lifecycle: AgentLifecycle) -> AgentManifest {
        AgentManifest {
            identity: AgentIdentity {
                agent_id: "OCAF-AGENT-001".into(),
                display_name: "Asterion".into(),
                swarm_id: "G01".into(),
                specialization: "runtime".into(),
                version: "0.1.0".into(),
            },
            lifecycle,
            autonomy: AutonomyContract {
                model_backend: "unassigned".into(),
                memory_namespace: "agent-001".into(),
                planner: "unassigned".into(),
                human_approval_for_side_effects: true,
                max_steps: 10,
                max_cost_minor: 0,
            },
            omas_endpoint: "omas://agent-server".into(),
            omat_endpoint: "omat://tool-server".into(),
            transport: "zenoh".into(),
            ostc_enabled: false,
            source_access: "disabled".into(),
            allowed_tools: BTreeSet::new(),
            artifact_digest: Some("sha256:agent".into()),
        }
    }

    #[test]
    fn manifest_is_individual_and_fail_closed() {
        let agent = manifest(AgentLifecycle::Staged);
        assert!(agent.validate().is_ok());
        let evidence = AgentEvidence {
            declared_state: AgentLifecycle::Active,
            source_commit: "fake".into(),
            artifact_digest: "sha256:agent".into(),
            tests_passed: true,
            signature_verified: true,
            omas_ready: true,
            omat_ready: true,
            ostc_ready: true,
            source_access_granted: true,
        };
        assert_eq!(
            agent.admit_active(&evidence),
            Err(AgentAdmissionError::OstcDisabled)
        );
    }

    #[test]
    fn shared_omas_omat_endpoint_is_rejected() {
        let mut agent = manifest(AgentLifecycle::Draft);
        agent.omat_endpoint = agent.omas_endpoint.clone();
        assert_eq!(
            agent.validate(),
            Err(AgentAdmissionError::SharedServiceEndpoint)
        );
    }

    #[test]
    fn lifecycle_rank_is_monotonic() {
        assert!(AgentLifecycle::Staged.rank() > AgentLifecycle::Tested.rank());
        assert!(AgentLifecycle::Active.rank() > AgentLifecycle::Staged.rank());
    }
}
