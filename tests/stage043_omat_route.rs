use ocaf_runtime::omat_server::{OmatPolicy, OmatRegistry, ToolCall};
use ocaf_runtime::stage043_omat_route::{
    dispatch_tool_call, AuthenticatedPeer, DispatchError, OmatRouteGuard, RouteError,
};
use std::collections::BTreeSet;
use std::collections::HashMap;

#[test]
fn omat_transport_denies_disconnected_and_cross_tenant_requests() {
    let mut peers = HashMap::new();
    peers.insert("node_a".into(), "tenant_a".into());
    let mut guard = OmatRouteGuard::new(peers).unwrap();
    let peer = AuthenticatedPeer {
        principal: "node_a".into(),
        tenant: "tenant_a".into(),
    };
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    assert_eq!(guard.authorize(&peer, topic), Err(RouteError::Disconnected));
    guard.set_transport_connected(true);
    assert_eq!(guard.authorize(&peer, topic), Ok(()));
    assert_eq!(
        guard.authorize(&peer, "ocaf/omat/tenant_b/requests/req_1"),
        Err(RouteError::InvalidTopic)
    );
    let forged = AuthenticatedPeer {
        principal: "node_a".into(),
        tenant: "tenant_b".into(),
    };
    assert_eq!(
        guard.authorize(&forged, "ocaf/omat/tenant_b/requests/req_1"),
        Err(RouteError::Unauthorized)
    );
    assert_eq!(
        guard.authorize(&peer, "ocaf/omat/tenant_a/requests/../other"),
        Err(RouteError::InvalidTopic)
    );
    guard.set_transport_connected(false);
    assert_eq!(guard.authorize(&peer, topic), Err(RouteError::Disconnected));
}

#[test]
fn tool_dispatch_binds_authenticated_peer_to_request_id_and_tenant() {
    let mut peers = HashMap::new();
    peers.insert("node_a".into(), "tenant_a".into());
    let mut guard = OmatRouteGuard::new(peers).unwrap();
    let peer = AuthenticatedPeer {
        principal: "node_a".into(),
        tenant: "tenant_a".into(),
    };
    let call = ToolCall {
        request_id: "req_1".into(),
        tenant_id: "tenant_a".into(),
        agent_id: "agent_1".into(),
        tool_id: "tool_1".into(),
        arguments: serde_json::json!({}),
        idempotency_key: "idem_1".into(),
        ostc_enabled: false,
        source_access: "disabled".into(),
    };
    let policy = OmatPolicy {
        tenant_id: "tenant_a".into(),
        allowed_tools: BTreeSet::new(),
        allowed_capabilities: BTreeSet::new(),
        max_timeout_ms: 1000,
        max_output_bytes: 1024,
        allow_external_side_effects: false,
    };
    let registry = OmatRegistry::default();
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    assert_eq!(
        dispatch_tool_call(&guard, &peer, topic, &call, &policy, &registry),
        Err(DispatchError::Route(RouteError::Disconnected))
    );
    guard.set_transport_connected(true);
    assert_eq!(
        dispatch_tool_call(
            &guard,
            &peer,
            "ocaf/omat/tenant_a/requests/req_2",
            &call,
            &policy,
            &registry
        ),
        Err(DispatchError::Route(RouteError::InvalidTopic))
    );
    let mut forged = call.clone();
    forged.tenant_id = "tenant_b".into();
    assert_eq!(
        dispatch_tool_call(&guard, &peer, topic, &forged, &policy, &registry),
        Err(DispatchError::Route(RouteError::Unauthorized))
    );
    assert_eq!(
        dispatch_tool_call(&guard, &peer, topic, &call, &policy, &registry),
        Err(DispatchError::Tool(ocaf_runtime::omat_server::OmatError::SourceAccessDenied))
    );
}
