#![cfg(feature = "zenoh")]
//! Stage 334: capability-bound OMAt ingress contract.
//! These tests deliberately do not assert successful execution: OSTc and
//! service executors remain independently fail-closed.
use ocaf_runtime::omat_server::{OmatPolicy, OmatRegistry, ToolCall};
use ocaf_runtime::sasos_capability::{
    CapabilityAction, CapabilityAuthority, CapabilityCaveat, CapabilityError, CapabilityVerbs,
};
use ocaf_runtime::stage043_omat_route::{
    dispatch_tool_call_with_capability, omat_resource_id, AuthenticatedPeer, DispatchError,
    CapabilityGrant, OmatRouteGuard, RouteError,
};
use ocaf_runtime::stage043_omat_zenoh::{
    sign_omas_request, sign_omas_request_with_capability, OmatZenohAdmission,
};
use std::collections::{BTreeSet, HashMap};

fn peer() -> AuthenticatedPeer {
    AuthenticatedPeer {
        principal: "omas_a".into(),
        tenant: "tenant_a".into(),
    }
}
fn call() -> ToolCall {
    ToolCall {
        request_id: "req_1".into(),
        tenant_id: "tenant_a".into(),
        agent_id: "agent_1".into(),
        tool_id: "tool_1".into(),
        arguments: serde_json::json!({"message":"hello"}),
        idempotency_key: "idem_1".into(),
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}
fn policy() -> OmatPolicy {
    OmatPolicy {
        tenant_id: "tenant_a".into(),
        allowed_tools: BTreeSet::new(),
        allowed_capabilities: BTreeSet::new(),
        max_timeout_ms: 1000,
        max_output_bytes: 1024,
        allow_external_side_effects: false,
    }
}
fn ticket(
    authority: &CapabilityAuthority,
    call: &ToolCall,
) -> ocaf_runtime::sasos_capability::CapabilityTicket {
    authority
        .mint(
            &peer(),
            &omat_resource_id(call),
            CapabilityCaveat {
                max_bytes: 1024,
                expires_at_unix: 4_000_000_000,
                verbs: CapabilityVerbs::EXECUTE,
            },
        )
        .unwrap()
}
fn admission() -> OmatZenohAdmission {
    OmatZenohAdmission::new(
        HashMap::from([("omas_a".into(), [7; 32])]),
        HashMap::from([("omas_a".into(), "tenant_a".into())]),
        HashMap::from([("tenant_a".into(), policy())]),
        OmatRegistry::default(),
    )
    .unwrap()
    .with_capability_authority(CapabilityAuthority::new([19; 32]).unwrap())
}
#[test]
fn capability_resource_binds_tenant_agent_and_tool() {
    let base = call();
    let mut other = base.clone();
    other.agent_id = "agent_2".into();
    assert_ne!(omat_resource_id(&base), omat_resource_id(&other));
    other = base.clone();
    other.tool_id = "tool_2".into();
    assert_ne!(omat_resource_id(&base), omat_resource_id(&other));
    other = base.clone();
    other.tenant_id = "tenant_b".into();
    assert_ne!(omat_resource_id(&base), omat_resource_id(&other));
}
#[test]
fn guarded_route_enforces_capability_before_ostc() {
    let authority = CapabilityAuthority::new([19; 32]).unwrap();
    let mut guard =
        OmatRouteGuard::new(HashMap::from([("omas_a".into(), "tenant_a".into())])).unwrap();
    let call = call();
    let ticket = ticket(&authority, &call);
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    let result = dispatch_tool_call_with_capability(
        &guard,
        &peer(),
        topic,
        &call,
        &policy(),
        &OmatRegistry::default(),
        CapabilityGrant { authority: &authority, ticket: &ticket },
    );
    assert_eq!(result, Err(DispatchError::Route(RouteError::Disconnected)));
    guard.set_transport_connected(true);
    let result = dispatch_tool_call_with_capability(
        &guard,
        &peer(),
        topic,
        &call,
        &policy(),
        &OmatRegistry::default(),
        CapabilityGrant { authority: &authority, ticket: &ticket },
    );
    assert_eq!(
        result,
        Err(DispatchError::Tool(
            ocaf_runtime::omat_server::OmatError::SourceAccessDenied
        ))
    );
    let mut swapped = call.clone();
    swapped.agent_id = "agent_2".into();
    assert_eq!(
        dispatch_tool_call_with_capability(
            &guard,
            &peer(),
            topic,
            &swapped,
            &policy(),
            &OmatRegistry::default(),
            CapabilityGrant { authority: &authority, ticket: &ticket },
        ),
        Err(DispatchError::Capability(CapabilityError::Denied))
    );
    let wrong = authority
        .mint(
            &peer(),
            &omat_resource_id(&call),
            CapabilityCaveat {
                max_bytes: 1024,
                expires_at_unix: 4_000_000_000,
                verbs: CapabilityVerbs::READ,
            },
        )
        .unwrap();
    assert_eq!(
        dispatch_tool_call_with_capability(
            &guard,
            &peer(),
            topic,
            &call,
            &policy(),
            &OmatRegistry::default(),
            CapabilityGrant { authority: &authority, ticket: &wrong },
        ),
        Err(DispatchError::Capability(CapabilityError::Denied))
    );
    let short = authority
        .mint(
            &peer(),
            &omat_resource_id(&call),
            CapabilityCaveat {
                max_bytes: 1,
                expires_at_unix: 4_000_000_000,
                verbs: CapabilityVerbs::EXECUTE,
            },
        )
        .unwrap();
    assert_eq!(
        dispatch_tool_call_with_capability(
            &guard,
            &peer(),
            topic,
            &call,
            &policy(),
            &OmatRegistry::default(),
            CapabilityGrant { authority: &authority, ticket: &short },
        ),
        Err(DispatchError::Capability(CapabilityError::Denied))
    );
    assert_ne!(CapabilityAction::Read, CapabilityAction::Execute);
}
#[test]
fn signed_ingress_rejects_missing_tampered_and_cross_agent_capabilities() {
    let authority = CapabilityAuthority::new([19; 32]).unwrap();
    let call = call();
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    let mut server = admission();
    server.connected(true);
    let legacy =
        sign_omas_request("omas_a", topic, "legacynonce123456", call.clone(), &[7; 32]).unwrap();
    assert_eq!(
        server.handle(&legacy).unwrap_err().to_string(),
        "OMAt request denied"
    );
    let cap = ticket(&authority, &call);
    let signed = sign_omas_request_with_capability(
        "omas_a",
        topic,
        "validnonce1234567",
        call.clone(),
        cap.clone(),
        &[7; 32],
    )
    .unwrap();
    assert_eq!(
        server.handle(&signed).unwrap_err().to_string(),
        "OMAt request denied"
    );
    assert!(server
        .handle(&signed)
        .unwrap_err()
        .to_string()
        .contains("replay"));

    let mut forged: serde_json::Value = serde_json::from_slice(
        &sign_omas_request_with_capability(
            "omas_a",
            topic,
            "tamperednonce1234",
            call.clone(),
            cap.clone(),
            &[7; 32],
        )
        .unwrap(),
    )
    .unwrap();
    forged["capability"]["resource_id"] = serde_json::json!("other");
    assert!(server
        .handle(&serde_json::to_vec(&forged).unwrap())
        .unwrap_err()
        .to_string()
        .contains("signature"));

    let mut swapped = call;
    swapped.agent_id = "agent_2".into();
    let signed = sign_omas_request_with_capability(
        "omas_a",
        topic,
        "agentnonce1234567",
        swapped,
        cap,
        &[7; 32],
    )
    .unwrap();
    assert_eq!(
        server.handle(&signed).unwrap_err().to_string(),
        "OMAt request denied"
    );
}
