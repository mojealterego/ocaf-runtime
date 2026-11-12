use ocaf_runtime::trs_digital_life::{dispatch_authorized, AuthenticatedPrincipal, BridgeError, BridgeRequest, DIGITAL_LIFE_BRIDGE_VERSION};
fn request(tenant: &str, sender: &str, operation: &str) -> BridgeRequest {
    BridgeRequest { version: DIGITAL_LIFE_BRIDGE_VERSION, tenant: tenant.into(), sender: sender.into(), operation: operation.into(), payload: vec![] }
}
#[test]
fn principal_must_match_claimed_tenant_and_sender() {
    let principal = AuthenticatedPrincipal { tenant: "alpha".into(), sender: "worker".into(), allowed_operations: vec!["trs.health".into()] };
    assert!(dispatch_authorized(&principal, &request("alpha", "worker", "trs.health")).is_ok());
    assert_eq!(dispatch_authorized(&principal, &request("beta", "worker", "trs.health")), Err(BridgeError::Unauthorized));
    assert_eq!(dispatch_authorized(&principal, &request("alpha", "other", "trs.health")), Err(BridgeError::Unauthorized));
}
#[test]
fn operation_requires_explicit_capability() {
    let principal = AuthenticatedPrincipal { tenant: "alpha".into(), sender: "worker".into(), allowed_operations: vec![] };
    assert_eq!(dispatch_authorized(&principal, &request("alpha", "worker", "trs.health")), Err(BridgeError::Unauthorized));
}
#[test]
fn large_sat_result_is_rejected_before_enumeration() {
    let mut request = request("alpha", "worker", "trs.math.sat_evaluate");
    request.payload = br#"{"variables":16,"clauses":[]}"#.to_vec();
    assert_eq!(ocaf_runtime::trs_digital_life::dispatch_local(&request), Err(BridgeError::InvalidPayload));
}
