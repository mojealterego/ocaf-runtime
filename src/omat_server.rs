//! OMAt: isolated tool-server contract.
//!
//! OMAt owns tool manifests and bounded admission.  It is deliberately
//! separate from OMAs (the agent server) and from Zenoh (transport).  This
//! module does not claim that an external tool backend exists: admission is
//! deterministic and the execution boundary fails closed until a registered
//! executor is supplied.

use crate::registry::compile_schema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const OMAT_PROTOCOL_VERSION: &str = "omat.v1";
const MAX_TOOLS: usize = 4_096;
const MAX_ID_BYTES: usize = 128;
const MAX_SCHEMA_BYTES: usize = 256 * 1024;
const MAX_ARGUMENT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRisk {
    ReadOnly,
    BoundedWrite,
    ExternalSideEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    Wasi,
    Firecracker,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolManifest {
    pub tenant_id: String,
    pub tool_id: String,
    pub version: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    pub risk: ToolRisk,
    pub executor: ExecutorKind,
    pub timeout_ms: u64,
    pub max_output_bytes: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OmatPolicy {
    pub tenant_id: String,
    pub allowed_tools: BTreeSet<String>,
    #[serde(default)]
    pub allowed_capabilities: BTreeSet<String>,
    pub max_timeout_ms: u64,
    pub max_output_bytes: u64,
    pub allow_external_side_effects: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub request_id: String,
    pub tenant_id: String,
    pub agent_id: String,
    pub tool_id: String,
    pub arguments: Value,
    pub idempotency_key: String,
    pub ostc_enabled: bool,
    pub source_access: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolReceipt {
    pub protocol_version: String,
    pub request_id: String,
    pub tenant_id: String,
    pub agent_id: String,
    pub tool_id: String,
    pub manifest_sha256: String,
    pub decision: String,
    pub execution_state: String,
    pub bounded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmatHealth {
    pub server_id: String,
    pub protocol_version: String,
    pub tool_count: usize,
    pub ready: bool,
    pub ostc_enabled: bool,
    pub source_access: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OmatError {
    #[error("invalid identifier")]
    InvalidIdentifier,
    #[error("tool registry is full")]
    RegistryFull,
    #[error("duplicate tool id")]
    DuplicateTool,
    #[error("tool not found")]
    ToolNotFound,
    #[error("tool belongs to another tenant")]
    TenantMismatch,
    #[error("tool is disabled or not allowed")]
    ToolDenied,
    #[error("tool capability is not allowed")]
    CapabilityDenied,
    #[error("tool timeout or output bound is denied")]
    ResourceDenied,
    #[error("external side effects are not permitted")]
    SideEffectDenied,
    #[error("source access must remain disabled while OSTc is disabled")]
    SourceAccessDenied,
    #[error("arguments exceed the bounded request size")]
    ArgumentsTooLarge,
    #[error("arguments do not satisfy the tool input schema")]
    InvalidArguments,
    #[error("executor is not configured")]
    ExecutorUnavailable,
}

#[derive(Debug, Clone, Default)]
pub struct OmatRegistry {
    tools: BTreeMap<(String, String), ToolManifest>,
}

impl OmatRegistry {
    pub fn register(&mut self, manifest: ToolManifest) -> Result<String, OmatError> {
        validate_manifest(&manifest)?;
        if self.tools.len() >= MAX_TOOLS {
            return Err(OmatError::RegistryFull);
        }
        let identity = (manifest.tenant_id.clone(), manifest.tool_id.clone());
        if self.tools.contains_key(&identity) {
            return Err(OmatError::DuplicateTool);
        }
        let digest = manifest_digest(&manifest)?;
        self.tools.insert(identity, manifest);
        Ok(digest)
    }

    /// A tool name is not a global identity: the tenant is always required.
    pub fn get_for(&self, tenant_id: &str, tool_id: &str) -> Option<&ToolManifest> {
        self.tools.get(&(tenant_id.to_owned(), tool_id.to_owned()))
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn list_for(&self, tenant_id: &str, policy: &OmatPolicy) -> Vec<String> {
        self.tools
            .values()
            .filter(|tool| {
                tool.enabled
                    && tool.tenant_id == tenant_id
                    && policy.tenant_id == tenant_id
                    && policy.allowed_tools.contains(&tool.tool_id)
                    && tool.timeout_ms > 0
                    && tool.timeout_ms <= policy.max_timeout_ms
                    && tool.max_output_bytes > 0
                    && tool.max_output_bytes <= policy.max_output_bytes
                    && (tool.risk != ToolRisk::ExternalSideEffect || policy.allow_external_side_effects)
                    && tool
                        .capabilities
                        .iter()
                        .all(|cap| policy.allowed_capabilities.contains(cap))
            })
            .map(|tool| tool.tool_id.clone())
            .collect()
    }

    /// Admit a call.  This validates all boundaries but never invokes a tool.
    pub fn admit(&self, call: &ToolCall, policy: &OmatPolicy) -> Result<ToolReceipt, OmatError> {
        validate_id(&call.request_id)?;
        validate_id(&call.tenant_id)?;
        validate_id(&call.agent_id)?;
        validate_id(&call.tool_id)?;
        validate_id(&call.idempotency_key)?;
        if !call.ostc_enabled || call.source_access != "disabled" {
            return Err(OmatError::SourceAccessDenied);
        }
        if policy.tenant_id != call.tenant_id {
            return Err(OmatError::TenantMismatch);
        }
        let tool = self.get_for(&call.tenant_id, &call.tool_id).ok_or(OmatError::ToolNotFound)?;
        authorize(tool, call, policy)?;
        let encoded = serde_json::to_vec(&call.arguments).map_err(|_| OmatError::InvalidArguments)?;
        if encoded.len() > MAX_ARGUMENT_BYTES {
            return Err(OmatError::ArgumentsTooLarge);
        }
        let validator = compile_schema(&tool.input_schema).map_err(|_| OmatError::InvalidArguments)?;
        if !validator.is_valid(&call.arguments) {
            return Err(OmatError::InvalidArguments);
        }
        Ok(ToolReceipt {
            protocol_version: OMAT_PROTOCOL_VERSION.into(),
            request_id: call.request_id.clone(),
            tenant_id: call.tenant_id.clone(),
            agent_id: call.agent_id.clone(),
            tool_id: call.tool_id.clone(),
            manifest_sha256: manifest_digest(tool)?,
            decision: "ALLOW_BOUNDED".into(),
            execution_state: "ADMITTED_NOT_EXECUTED".into(),
            bounded: true,
        })
    }

    /// Execution is an explicit fail-closed boundary until an executor with
    /// its own receipt is attached.  This prevents a registry from becoming
    /// an implicit remote-code-execution service.
    pub fn execute(&self, call: &ToolCall, policy: &OmatPolicy) -> Result<ToolReceipt, OmatError> {
        let _ = self.admit(call, policy)?;
        Err(OmatError::ExecutorUnavailable)
    }
}

pub fn health(server_id: &str, registry: &OmatRegistry) -> OmatHealth {
    OmatHealth {
        server_id: server_id.to_owned(),
        protocol_version: OMAT_PROTOCOL_VERSION.into(),
        tool_count: registry.len(),
        // The registry can admit tools, but there is no configured executor yet.
        // Readiness must never imply that a tool can actually execute.
        ready: false,
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}

fn authorize(tool: &ToolManifest, call: &ToolCall, policy: &OmatPolicy) -> Result<(), OmatError> {
    if !tool.enabled || tool.tenant_id != call.tenant_id || !policy.allowed_tools.contains(&tool.tool_id) {
        return Err(OmatError::ToolDenied);
    }
    if !tool
        .capabilities
        .iter()
        .all(|cap| policy.allowed_capabilities.contains(cap))
    {
        return Err(OmatError::CapabilityDenied);
    }
    if tool.timeout_ms == 0
        || tool.timeout_ms > policy.max_timeout_ms
        || tool.max_output_bytes == 0
        || tool.max_output_bytes > policy.max_output_bytes
    {
        return Err(OmatError::ResourceDenied);
    }
    if tool.risk == ToolRisk::ExternalSideEffect && !policy.allow_external_side_effects {
        return Err(OmatError::SideEffectDenied);
    }
    Ok(())
}

fn validate_manifest(tool: &ToolManifest) -> Result<(), OmatError> {
    validate_id(&tool.tenant_id)?;
    validate_id(&tool.tool_id)?;
    validate_id(&tool.version)?;
    if tool.description.is_empty()
        || tool.timeout_ms == 0
        || tool.max_output_bytes == 0
        || tool.input_schema.to_string().len() > MAX_SCHEMA_BYTES
        || tool.output_schema.to_string().len() > MAX_SCHEMA_BYTES
    {
        return Err(OmatError::ResourceDenied);
    }
    compile_schema(&tool.input_schema).map_err(|_| OmatError::InvalidArguments)?;
    compile_schema(&tool.output_schema).map_err(|_| OmatError::InvalidArguments)?;
    Ok(())
}

fn validate_id(value: &str) -> Result<(), OmatError> {
    if value.is_empty()
        || value.len() > MAX_ID_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
    {
        return Err(OmatError::InvalidIdentifier);
    }
    Ok(())
}

fn manifest_digest(tool: &ToolManifest) -> Result<String, OmatError> {
    let encoded = serde_json::to_vec(tool).map_err(|_| OmatError::InvalidArguments)?;
    let mut hasher = Sha256::new();
    hasher.update(encoded);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest(risk: ToolRisk) -> ToolManifest {
        ToolManifest {
            tenant_id: "tenant-a".into(),
            tool_id: "echo".into(),
            version: "1".into(),
            description: "bounded echo".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            capabilities: ["read".into()].into_iter().collect(),
            risk,
            executor: ExecutorKind::Wasi,
            timeout_ms: 100,
            max_output_bytes: 1024,
            enabled: true,
        }
    }

    fn policy() -> OmatPolicy {
        OmatPolicy {
            tenant_id: "tenant-a".into(),
            allowed_tools: ["echo".into()].into_iter().collect(),
            allowed_capabilities: ["read".into()].into_iter().collect(),
            max_timeout_ms: 200,
            max_output_bytes: 2048,
            allow_external_side_effects: false,
        }
    }

    fn call() -> ToolCall {
        ToolCall {
            request_id: "req-1".into(),
            tenant_id: "tenant-a".into(),
            agent_id: "agent-1".into(),
            tool_id: "echo".into(),
            arguments: json!({}),
            idempotency_key: "idem-1".into(),
            ostc_enabled: true,
            source_access: "disabled".into(),
        }
    }

    #[test]
    fn registers_lists_and_admits_tenant_scoped_tool() {
        let mut registry = OmatRegistry::default();
        assert!(registry.register(manifest(ToolRisk::ReadOnly)).unwrap().starts_with("sha256:"));
        assert_eq!(registry.list_for("tenant-a", &policy()), vec!["echo"]);
        assert_eq!(registry.admit(&call(), &policy()).unwrap().decision, "ALLOW_BOUNDED");
    }

    #[test]
    fn cross_tenant_is_denied() {
        let mut registry = OmatRegistry::default();
        registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        let mut request = call();
        request.tenant_id = "tenant-b".into();
        assert_eq!(registry.admit(&request, &policy()), Err(OmatError::TenantMismatch));
    }

    #[test]
    fn disabled_ostc_denies_even_valid_call() {
        let mut registry = OmatRegistry::default();
        registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        let mut request = call();
        request.ostc_enabled = false;
        assert_eq!(registry.admit(&request, &policy()), Err(OmatError::SourceAccessDenied));
    }

    #[test]
    fn side_effects_need_explicit_policy() {
        let mut registry = OmatRegistry::default();
        registry.register(manifest(ToolRisk::ExternalSideEffect)).unwrap();
        assert_eq!(registry.admit(&call(), &policy()), Err(OmatError::SideEffectDenied));
    }

    #[test]
    fn executor_fails_closed_after_admission() {
        let mut registry = OmatRegistry::default();
        registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        assert_eq!(registry.execute(&call(), &policy()), Err(OmatError::ExecutorUnavailable));
    }

    #[test]
    fn health_exposes_disabled_source_access() {
        let registry = OmatRegistry::default();
        let status = health("omat-1", &registry);
        assert!(!status.ready);
        assert!(!status.ostc_enabled);
        assert_eq!(status.source_access, "disabled");
    }
}
