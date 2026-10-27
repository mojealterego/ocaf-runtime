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
const MAX_DESCRIPTION_BYTES: usize = 4 * 1024;
const MAX_CAPABILITIES: usize = 64;

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

/// Discovery never exposes free-form descriptions as instructions. The client
/// pins this digest when retrieving a schema, so a changed manifest cannot be
/// mistaken for the one that was selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub tenant_id: String,
    pub tool_id: String,
    pub version: String,
    pub manifest_sha256: String,
    pub risk: ToolRisk,
    pub capabilities: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSchemas {
    pub manifest_sha256: String,
    pub input_schema: Value,
    pub output_schema: Value,
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
    #[error("the tool manifest changed since discovery")]
    ManifestChanged,
    #[error("a changed manifest must use a new version")]
    VersionConflict,
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
    // This is a server-owned trust gate. A ToolCall's ostc_enabled flag is
    // untrusted client input and must never activate execution authority.
    // Only a future verified OSTc authority may set this (not implemented).
    trusted_ostc_attested: bool,
}

impl OmatRegistry {
    pub fn register(&mut self, manifest: ToolManifest) -> Result<String, OmatError> {
        validate_manifest(&manifest)?;
        let identity = (manifest.tenant_id.clone(), manifest.tool_id.clone());
        if self.tools.contains_key(&identity) {
            return Err(OmatError::DuplicateTool);
        }
        if self.tools.len() >= MAX_TOOLS {
            return Err(OmatError::RegistryFull);
        }
        let digest = manifest_digest(&manifest)?;
        self.tools.insert(identity, manifest);
        Ok(digest)
    }

    /// The registry owner may rotate a manifest only after checking the
    /// previously published digest. A changed schema under the same version
    /// is rejected. Publisher authentication belongs to the outer control
    /// plane and is not inferred from this in-process method.
    pub fn replace_if_digest(
        &mut self,
        expected_sha256: &str,
        manifest: ToolManifest,
    ) -> Result<String, OmatError> {
        validate_manifest(&manifest)?;
        let identity = (manifest.tenant_id.clone(), manifest.tool_id.clone());
        let current = self.tools.get(&identity).ok_or(OmatError::ToolNotFound)?;
        if manifest_digest(current)? != expected_sha256 {
            return Err(OmatError::ManifestChanged);
        }
        if current.version == manifest.version {
            return Err(OmatError::VersionConflict);
        }
        let digest = manifest_digest(&manifest)?;
        self.tools.insert(identity, manifest);
        Ok(digest)
    }

    /// Fail-closed revocation keeps the identity but disables discovery and
    /// admission. Re-enablement requires a new manifest version.
    pub fn revoke_if_digest(
        &mut self,
        tenant_id: &str,
        tool_id: &str,
        expected_sha256: &str,
    ) -> Result<String, OmatError> {
        let tool = self
            .tools
            .get_mut(&(tenant_id.to_owned(), tool_id.to_owned()))
            .ok_or(OmatError::ToolNotFound)?;
        if manifest_digest(tool)? != expected_sha256 {
            return Err(OmatError::ManifestChanged);
        }
        if !tool.enabled {
            return Err(OmatError::ToolDenied);
        }
        tool.enabled = false;
        manifest_digest(tool)
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
            .filter(|tool| visible_to_policy(tool, tenant_id, policy))
            .map(|tool| tool.tool_id.clone())
            .collect()
    }

    pub fn discover_for(
        &self,
        tenant_id: &str,
        policy: &OmatPolicy,
    ) -> Result<Vec<ToolDescriptor>, OmatError> {
        self.tools
            .values()
            .filter(|tool| visible_to_policy(tool, tenant_id, policy))
            .map(|tool| {
                Ok(ToolDescriptor {
                    tenant_id: tool.tenant_id.clone(),
                    tool_id: tool.tool_id.clone(),
                    version: tool.version.clone(),
                    manifest_sha256: manifest_digest(tool)?,
                    risk: tool.risk,
                    capabilities: tool.capabilities.clone(),
                })
            })
            .collect()
    }

    pub fn schemas_for(
        &self,
        tenant_id: &str,
        tool_id: &str,
        policy: &OmatPolicy,
        expected_sha256: &str,
    ) -> Result<ToolSchemas, OmatError> {
        if tenant_id != policy.tenant_id {
            return Err(OmatError::TenantMismatch);
        }
        let tool = self
            .get_for(tenant_id, tool_id)
            .ok_or(OmatError::ToolNotFound)?;
        if !visible_to_policy(tool, tenant_id, policy) {
            return Err(OmatError::ToolDenied);
        }
        let digest = manifest_digest(tool)?;
        if digest != expected_sha256 {
            return Err(OmatError::ManifestChanged);
        }
        Ok(ToolSchemas {
            manifest_sha256: digest,
            input_schema: tool.input_schema.clone(),
            output_schema: tool.output_schema.clone(),
        })
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
        let tool = self
            .get_for(&call.tenant_id, &call.tool_id)
            .ok_or(OmatError::ToolNotFound)?;
        authorize(tool, call, policy)?;
        let encoded =
            serde_json::to_vec(&call.arguments).map_err(|_| OmatError::InvalidArguments)?;
        if encoded.len() > MAX_ARGUMENT_BYTES {
            return Err(OmatError::ArgumentsTooLarge);
        }
        let validator =
            compile_schema(&tool.input_schema).map_err(|_| OmatError::InvalidArguments)?;
        if !validator.is_valid(&call.arguments) {
            return Err(OmatError::InvalidArguments);
        }
        if !self.trusted_ostc_attested {
            return Err(OmatError::SourceAccessDenied);
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
    if !tool.enabled
        || tool.tenant_id != call.tenant_id
        || !policy.allowed_tools.contains(&tool.tool_id)
    {
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

fn visible_to_policy(tool: &ToolManifest, tenant_id: &str, policy: &OmatPolicy) -> bool {
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
}

fn validate_manifest(tool: &ToolManifest) -> Result<(), OmatError> {
    validate_id(&tool.tenant_id)?;
    validate_id(&tool.tool_id)?;
    validate_id(&tool.version)?;
    if tool.description.is_empty()
        || tool.description.len() > MAX_DESCRIPTION_BYTES
        || tool.capabilities.len() > MAX_CAPABILITIES
        || tool
            .capabilities
            .iter()
            .any(|cap| validate_id(cap).is_err())
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
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
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
    fn registers_lists_and_rejects_spoofed_ostc_tenant_scoped_tool() {
        let mut registry = OmatRegistry::default();
        assert!(registry
            .register(manifest(ToolRisk::ReadOnly))
            .unwrap()
            .starts_with("sha256:"));
        assert_eq!(registry.list_for("tenant-a", &policy()), vec!["echo"]);
        assert_eq!(
            registry.admit(&call(), &policy()),
            Err(OmatError::SourceAccessDenied)
        );
    }

    #[test]
    fn cross_tenant_is_denied() {
        let mut registry = OmatRegistry::default();
        registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        let mut request = call();
        request.tenant_id = "tenant-b".into();
        assert_eq!(
            registry.admit(&request, &policy()),
            Err(OmatError::TenantMismatch)
        );
    }

    #[test]
    fn disabled_ostc_denies_even_valid_call() {
        let mut registry = OmatRegistry::default();
        registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        let mut request = call();
        request.ostc_enabled = false;
        assert_eq!(
            registry.admit(&request, &policy()),
            Err(OmatError::SourceAccessDenied)
        );
    }

    #[test]
    fn side_effects_need_explicit_policy() {
        let mut registry = OmatRegistry::default();
        registry
            .register(manifest(ToolRisk::ExternalSideEffect))
            .unwrap();
        assert_eq!(
            registry.admit(&call(), &policy()),
            Err(OmatError::SideEffectDenied)
        );
    }

    #[test]
    fn executor_fails_closed_after_admission() {
        let mut registry = OmatRegistry::default();
        registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        assert_eq!(
            registry.execute(&call(), &policy()),
            Err(OmatError::SourceAccessDenied)
        );
    }

    #[test]
    fn health_exposes_disabled_source_access() {
        let registry = OmatRegistry::default();
        let status = health("omat-1", &registry);
        assert!(!status.ready);
        assert!(!status.ostc_enabled);
        assert_eq!(status.source_access, "disabled");
    }

    #[test]
    fn discovery_pins_schema_and_rejects_rug_pull() {
        let mut registry = OmatRegistry::default();
        let original = registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        let result = registry.discover_for("tenant-a", &policy()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].manifest_sha256, original);
        let discovered = serde_json::to_string(&result[0]).unwrap();
        assert!(!discovered.contains("bounded echo"));
        let first_schema = registry
            .schemas_for("tenant-a", "echo", &policy(), &original)
            .unwrap();
        assert_eq!(first_schema.input_schema, json!({"type":"object"}));

        let mut modified = manifest(ToolRisk::ReadOnly);
        modified.input_schema = json!({"type":"object","required":["message"]});
        assert_eq!(
            registry.replace_if_digest(&original, modified.clone()),
            Err(OmatError::VersionConflict)
        );
        modified.version = "2".into();
        assert_eq!(
            registry.replace_if_digest("sha256:stale", modified.clone()),
            Err(OmatError::ManifestChanged)
        );
        let second = registry.replace_if_digest(&original, modified).unwrap();
        assert_ne!(second, original);
        assert_eq!(
            registry.schemas_for("tenant-a", "echo", &policy(), &original),
            Err(OmatError::ManifestChanged)
        );
        assert_eq!(
            registry
                .schemas_for("tenant-a", "echo", &policy(), &second)
                .unwrap()
                .input_schema,
            json!({"type":"object","required":["message"]})
        );
        assert_eq!(
            registry.revoke_if_digest("tenant-a", "echo", &original),
            Err(OmatError::ManifestChanged)
        );
        let revoked = registry
            .revoke_if_digest("tenant-a", "echo", &second)
            .unwrap();
        assert!(registry
            .discover_for("tenant-a", &policy())
            .unwrap()
            .is_empty());
        assert_eq!(
            registry.schemas_for("tenant-a", "echo", &policy(), &second),
            Err(OmatError::ToolDenied)
        );
        assert_eq!(
            registry.admit(&call(), &policy()),
            Err(OmatError::ToolDenied)
        );

        let mut resumed = manifest(ToolRisk::ReadOnly);
        resumed.version = "3".into();
        let resumed_digest = registry.replace_if_digest(&revoked, resumed).unwrap();
        assert_eq!(
            registry.discover_for("tenant-a", &policy()).unwrap()[0].manifest_sha256,
            resumed_digest
        );
    }

    #[test]
    fn discovery_cannot_cross_tenant_or_bypass_policy() {
        let mut registry = OmatRegistry::default();
        let digest = registry.register(manifest(ToolRisk::ReadOnly)).unwrap();
        let mut other_policy = policy();
        other_policy.tenant_id = "tenant-b".into();
        assert!(registry
            .discover_for("tenant-a", &other_policy)
            .unwrap()
            .is_empty());
        assert_eq!(
            registry.schemas_for("tenant-a", "echo", &other_policy, &digest),
            Err(OmatError::TenantMismatch)
        );
        let mut denied = policy();
        denied.allowed_capabilities.clear();
        assert!(registry
            .discover_for("tenant-a", &denied)
            .unwrap()
            .is_empty());
        assert_eq!(
            registry.schemas_for("tenant-a", "echo", &denied, &digest),
            Err(OmatError::ToolDenied)
        );
    }

    #[test]
    fn unbounded_manifest_metadata_is_rejected_at_registration() {
        let mut registry = OmatRegistry::default();
        let mut large = manifest(ToolRisk::ReadOnly);
        large.description = "x".repeat(MAX_DESCRIPTION_BYTES + 1);
        assert_eq!(registry.register(large), Err(OmatError::ResourceDenied));
        let mut oversized_caps = manifest(ToolRisk::ReadOnly);
        oversized_caps.capabilities = (0..=MAX_CAPABILITIES)
            .map(|index| format!("cap_{index}"))
            .collect();
        assert_eq!(
            registry.register(oversized_caps),
            Err(OmatError::ResourceDenied)
        );
        let mut invalid_cap = manifest(ToolRisk::ReadOnly);
        invalid_cap.capabilities.insert("not a capability".into());
        assert_eq!(
            registry.register(invalid_cap),
            Err(OmatError::ResourceDenied)
        );
    }
}
