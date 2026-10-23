//! Stage 0.43 black-box security regression suite for native OMAs and OMAt.
//! No network, database, paid service, or OSTc activation is needed.
use ocaf_runtime::omas_server::{AgentRegistrationRequest, OmasServer, OmasServerConfig};
use ocaf_runtime::omat_server::{
    health, ExecutorKind, OmatError, OmatPolicy, OmatRegistry, ToolCall, ToolManifest, ToolRisk,
};
use serde_json::json;
use std::collections::BTreeSet;

fn agent(tenant: &str, request_id: &str) -> AgentRegistrationRequest {
    AgentRegistrationRequest {
        request_id: request_id.into(),
        agent_id: "OCAF-AGENT-001".into(),
        tenant: tenant.into(),
        manifest_digest: format!("sha256:{}", "a".repeat(64)),
        declared_state: "STAGED".into(),
        readiness: false,
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}

fn tool(tenant: &str, risk: ToolRisk) -> ToolManifest {
    ToolManifest {
        tenant_id: tenant.into(),
        tool_id: "echo".into(),
        version: "1".into(),
        description: "bounded test tool".into(),
        input_schema: json!({"type":"object", "additionalProperties":false}),
        output_schema: json!({"type":"object"}),
        capabilities: ["read".to_string()].into_iter().collect(),
        risk,
        executor: ExecutorKind::Wasi,
        timeout_ms: 100,
        max_output_bytes: 1024,
        enabled: true,
    }
}

fn policy(tenant: &str) -> OmatPolicy {
    OmatPolicy {
        tenant_id: tenant.into(),
        allowed_tools: ["echo".to_string()].into_iter().collect(),
        allowed_capabilities: BTreeSet::from(["read".to_string()]),
        max_timeout_ms: 200,
        max_output_bytes: 2048,
        allow_external_side_effects: false,
    }
}

#[test]
fn omas_tracks_the_same_agent_id_in_two_tenants_independently() {
    let server = OmasServer::new(OmasServerConfig::default()).unwrap();
    assert!(
        server
            .admit_registration(agent("tenant_a", "request-a"))
            .admitted
    );
    assert!(
        server
            .admit_registration(agent("tenant_b", "request-b"))
            .admitted
    );
    assert_eq!(
        server.health().registered_agents,
        2,
        "tenant identities must not alias"
    );
}

#[test]
fn omas_rejects_truncated_or_non_hex_manifest_hashes() {
    let server = OmasServer::new(OmasServerConfig::default()).unwrap();
    let mut malformed = agent("tenant_a", "digest-short");
    malformed.manifest_digest = "sha256:abc".into();
    assert!(!server.admit_registration(malformed).admitted);

    let mut non_hex = agent("tenant_a", "digest-non-hex");
    non_hex.manifest_digest = format!("sha256:{}", "Z".repeat(64));
    assert!(!server.admit_registration(non_hex).admitted);
    assert_eq!(server.health().registered_agents, 0);
}

#[test]
fn omas_rejects_unknown_lifecycle_values() {
    let server = OmasServer::new(OmasServerConfig::default()).unwrap();
    let mut req = agent("tenant_a", "state-unknown");
    req.declared_state = "UNKNOWN".into();
    assert!(!server.admit_registration(req).admitted);
}

#[test]
fn omat_supports_same_tool_id_for_distinct_tenants() {
    let mut registry = OmatRegistry::default();
    let a = registry.register(tool("tenant_a", ToolRisk::ReadOnly));
    let b = registry.register(tool("tenant_b", ToolRisk::ReadOnly));
    assert!(a.is_ok());
    assert!(
        b.is_ok(),
        "a tenant must not be blocked by another tenant's name"
    );
    assert_eq!(registry.len(), 2);
    assert_eq!(
        registry.list_for("tenant_a", &policy("tenant_a")),
        vec!["echo"]
    );
    assert_eq!(
        registry.list_for("tenant_b", &policy("tenant_b")),
        vec!["echo"]
    );
    assert!(registry.get_for("tenant_a", "echo").is_some());
    assert!(registry.get_for("tenant_b", "echo").is_some());
    assert!(registry.get_for("tenant_c", "echo").is_none());
}

#[test]
fn omat_hides_disallowed_side_effect_tools_from_discovery() {
    let mut registry = OmatRegistry::default();
    registry
        .register(tool("tenant_a", ToolRisk::ExternalSideEffect))
        .unwrap();
    assert!(registry
        .list_for("tenant_a", &policy("tenant_a"))
        .is_empty());
}

#[test]
fn omat_does_not_claim_ready_without_an_executor() {
    let mut registry = OmatRegistry::default();
    registry
        .register(tool("tenant_a", ToolRisk::ReadOnly))
        .unwrap();
    assert!(!health("omat-local", &registry).ready);
}

#[test]
fn omat_duplicate_manifest_remains_denied_within_a_tenant() {
    let mut registry = OmatRegistry::default();
    registry
        .register(tool("tenant_a", ToolRisk::ReadOnly))
        .unwrap();
    assert!(registry
        .register(tool("tenant_a", ToolRisk::ReadOnly))
        .is_err());
}

#[test]
fn omat_rejects_client_spoofing_of_server_side_ostc_readiness() {
    let mut registry = OmatRegistry::default();
    registry.register(tool("tenant_a", ToolRisk::ReadOnly)).unwrap();
    let call = ToolCall {
        request_id: "spoof-1".into(),
        tenant_id: "tenant_a".into(),
        agent_id: "agent-1".into(),
        tool_id: "echo".into(),
        arguments: json!({}),
        idempotency_key: "spoof-idem".into(),
        ostc_enabled: true, // This is untrusted caller data, not a server attestation.
        source_access: "disabled".into(),
    };
    assert_eq!(
        registry.admit(&call, &policy("tenant_a")),
        Err(OmatError::SourceAccessDenied)
    );
    assert_eq!(
        registry.execute(&call, &policy("tenant_a")),
        Err(OmatError::SourceAccessDenied)
    );
}
