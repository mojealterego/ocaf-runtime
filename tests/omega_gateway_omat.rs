use ocaf_runtime::omega_gateway_omat::{register_staged, stage_call, GatewayPrompt, CAPABILITY, TOOL_ID};
use ocaf_runtime::omat_server::{OmatError, OmatPolicy, OmatRegistry};
use ocaf_runtime::stage043_omat_route::{omat_resource_id, AuthenticatedPeer};

fn peer() -> AuthenticatedPeer {
    AuthenticatedPeer { principal: "operator".into(), tenant: "tenant-a".into() }
}
fn policy() -> OmatPolicy {
    OmatPolicy {
        tenant_id: "tenant-a".into(), allowed_tools: [TOOL_ID.into()].into_iter().collect(),
        allowed_capabilities: [CAPABILITY.into()].into_iter().collect(),
        max_timeout_ms: 5000, max_output_bytes: 262144, allow_external_side_effects: false,
    }
}
fn prompt() -> GatewayPrompt {
    GatewayPrompt { input: "  żółć\n".into(), max_output_tokens: 128 }
}
#[test]
fn staged_registry_never_exposes_or_executes_tool() {
    let mut registry = OmatRegistry::default();
    assert!(register_staged(&mut registry,"tenant-a").is_ok());
    assert!(registry.discover_for("tenant-a",&policy()).unwrap().is_empty());
    let staged = stage_call(&peer(),"agent","req","idem",&prompt()).unwrap();
    assert_eq!(registry.execute(&staged.call,&policy()),Err(OmatError::SourceAccessDenied));
}
#[test]
fn request_is_bound_to_peer_and_existing_resource_digest() {
    let staged = stage_call(&peer(),"agent","req","idem",&prompt()).unwrap();
    assert_eq!(staged.topic,"ocaf/omat/tenant-a/requests/req");
    assert_eq!(staged.resource,omat_resource_id(&staged.call));
    assert_eq!(staged.call.arguments["input"],"  żółć\n");
    assert!(!staged.call.ostc_enabled);
}
#[test]
fn rejects_cross_topic_and_unbounded_or_extra_arguments() {
    assert!(stage_call(&peer(),"agent","../tenant-b","idem",&prompt()).is_err());
    assert!(stage_call(&peer(),"agent","req","idem",&GatewayPrompt { input:"x".repeat(16385),max_output_tokens:128 }).is_err());
    assert!(serde_json::from_value::<GatewayPrompt>(serde_json::json!({"input":"x","max_output_tokens":128,"allow_cloud":true})).is_err());
}
