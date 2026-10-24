use ocaf_runtime::stage043_omat_route::{AuthenticatedPeer, OmatRouteGuard, RouteError};
use std::collections::HashMap;

#[test]
fn omat_transport_denies_disconnected_and_cross_tenant_requests() {
    let mut peers = HashMap::new();
    peers.insert("node_a".into(), "tenant_a".into());
    let mut guard = OmatRouteGuard::new(peers).unwrap();
    let peer = AuthenticatedPeer { principal: "node_a".into(), tenant: "tenant_a".into() };
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    assert_eq!(guard.authorize(&peer, topic), Err(RouteError::Disconnected));
    guard.set_transport_connected(true);
    assert_eq!(guard.authorize(&peer, topic), Ok(()));
    assert_eq!(guard.authorize(&peer, "ocaf/omat/tenant_b/requests/req_1"), Err(RouteError::InvalidTopic));
    let forged = AuthenticatedPeer { principal: "node_a".into(), tenant: "tenant_b".into() };
    assert_eq!(guard.authorize(&forged, "ocaf/omat/tenant_b/requests/req_1"), Err(RouteError::Unauthorized));
    assert_eq!(guard.authorize(&peer, "ocaf/omat/tenant_a/requests/../other"), Err(RouteError::InvalidTopic));
    guard.set_transport_connected(false);
    assert_eq!(guard.authorize(&peer, topic), Err(RouteError::Disconnected));
}
