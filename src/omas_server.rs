//! OMAs: the separate agent server boundary.
//!
//! This module intentionally contains admission and health contracts only.  It
//! does not execute tools (that belongs to OMAt) and it cannot grant source
//! access (that belongs to the future OSTc integration).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OmasServerConfig {
    pub server_id: String,
    pub bind_address: String,
    pub max_agents: u32,
    pub ostc_enabled: bool,
    pub source_access: String,
    pub active_admission: String,
}

impl Default for OmasServerConfig {
    fn default() -> Self {
        Self {
            server_id: "omas-local".into(),
            bind_address: "127.0.0.1:7443".into(),
            max_agents: 111,
            ostc_enabled: false,
            source_access: "disabled".into(),
            active_admission: "DENY".into(),
        }
    }
}

impl OmasServerConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.server_id.is_empty() || self.server_id.len() > 128 {
            return Err("invalid server_id".into());
        }
        if self.bind_address.is_empty() || self.bind_address.len() > 256 {
            return Err("invalid bind_address".into());
        }
        if self.max_agents == 0 || self.max_agents > 10_000 {
            return Err("max_agents outside safe bounds".into());
        }
        if self.ostc_enabled {
            return Err("OMAs cannot enable OSTc; source protection is a future separate service".into());
        }
        if self.source_access != "disabled" {
            return Err("OMAs source_access must be disabled".into());
        }
        if self.active_admission != "DENY" {
            return Err("OMAs ACTIVE admission must remain DENY while OSTc is disabled".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentRegistrationRequest {
    pub request_id: String,
    pub agent_id: String,
    pub tenant: String,
    pub manifest_digest: String,
    pub declared_state: String,
    pub readiness: bool,
    pub ostc_enabled: bool,
    pub source_access: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdmissionDecision {
    pub request_id: String,
    pub agent_id: String,
    pub admitted: bool,
    pub lifecycle: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OmasHealth {
    pub service: &'static str,
    pub health: &'static str,
    pub registered_agents: usize,
    pub max_agents: u32,
    pub ostc_enabled: bool,
    pub active_admission: &'static str,
}

#[derive(Clone)]
pub struct OmasServer {
    config: OmasServerConfig,
    registered: Arc<Mutex<HashSet<String>>>,
    requests: Arc<Mutex<HashSet<String>>>,
}

impl OmasServer {
    pub fn new(config: OmasServerConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            registered: Arc::new(Mutex::new(HashSet::new())),
            requests: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub fn config(&self) -> &OmasServerConfig { &self.config }

    pub fn health(&self) -> OmasHealth {
        let registered_agents = self.registered.lock().map(|g| g.len()).unwrap_or(0);
        OmasHealth {
            service: "OMAs",
            health: "ready_for_staged_agents",
            registered_agents,
            max_agents: self.config.max_agents,
            ostc_enabled: false,
            active_admission: "DENY",
        }
    }

    pub fn admit_registration(&self, req: AgentRegistrationRequest) -> AdmissionDecision {
        let denied = |reason: &str| AdmissionDecision {
            request_id: req.request_id.clone(), agent_id: req.agent_id.clone(),
            admitted: false, lifecycle: "DRAFT".into(), reason: reason.into(),
        };
        if req.request_id.is_empty() || req.request_id.len() > 128 { return denied("invalid_request_id"); }
        if req.agent_id.is_empty() || req.agent_id.len() > 128 || !req.agent_id.is_ascii() { return denied("invalid_agent_id"); }
        if !crate::omas_policy::safe_key(&req.tenant) || req.tenant.contains('/') { return denied("invalid_tenant"); }
        if req.manifest_digest.is_empty() || !req.manifest_digest.starts_with("sha256:") { return denied("missing_manifest_digest"); }
        if req.ostc_enabled || req.source_access != "disabled" { return denied("ostc_disabled_source_access_denied"); }
        if req.declared_state == "ACTIVE" || req.readiness { return denied("active_denied_ostc_disabled"); }
        let mut requests = match self.requests.lock() { Ok(lock) => lock, Err(_) => return denied("server_state_unavailable") };
        if !requests.insert(req.request_id.clone()) { return denied("replay_request_id"); }
        drop(requests);
        let mut registered = match self.registered.lock() { Ok(lock) => lock, Err(_) => return denied("server_state_unavailable") };
        if registered.len() >= self.config.max_agents && !registered.contains(&req.agent_id) { return denied("agent_capacity_exhausted"); }
        registered.insert(req.agent_id.clone());
        AdmissionDecision { request_id: req.request_id, agent_id: req.agent_id, admitted: true, lifecycle: "STAGED".into(), reason: "staged_registration_admitted".into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn req(id: &str) -> AgentRegistrationRequest { AgentRegistrationRequest { request_id: id.into(), agent_id: "OCAF-AGENT-001".into(), tenant: "tenant_a".into(), manifest_digest: "sha256:abc".into(), declared_state: "STAGED".into(), readiness: false, ostc_enabled: false, source_access: "disabled".into() } }
    #[test] fn config_keeps_ostc_and_active_closed() { assert!(OmasServerConfig::default().validate().is_ok()); }
    #[test] fn staged_registration_is_admitted() { let s=OmasServer::new(OmasServerConfig::default()).unwrap(); assert!(s.admit_registration(req("r1")).admitted); }
    #[test] fn fake_ready_active_is_denied() { let s=OmasServer::new(OmasServerConfig::default()).unwrap(); let mut r=req("r2"); r.readiness=true; r.declared_state="ACTIVE".into(); assert!(!s.admit_registration(r).admitted); }
    #[test] fn replay_is_denied() { let s=OmasServer::new(OmasServerConfig::default()).unwrap(); assert!(s.admit_registration(req("r3")).admitted); assert_eq!(s.admit_registration(req("r3")).reason,"replay_request_id"); }
    #[test] fn source_access_is_denied() { let s=OmasServer::new(OmasServerConfig::default()).unwrap(); let mut r=req("r4"); r.source_access="granted".into(); assert!(!s.admit_registration(r).admitted); }
}
