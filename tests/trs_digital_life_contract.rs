use ocaf_runtime::trs_digital_life::{Admission, BridgeError, BridgeRequest, BridgeResponse, DIGITAL_LIFE_BRIDGE_VERSION};

#[test]
fn version_and_identity_are_checked_before_dispatch() {
    let bad = BridgeRequest { version: 99, tenant: "a".into(), sender: "b".into(), operation: "health".into(), payload: vec![] };
    assert_eq!(bad.validate(), Err(BridgeError::UnsupportedVersion));
    let bad = BridgeRequest { version: DIGITAL_LIFE_BRIDGE_VERSION, tenant: "".into(), sender: "b".into(), operation: "health".into(), payload: vec![] };
    assert_eq!(bad.validate(), Err(BridgeError::InvalidIdentity));
}
#[test]
fn only_allowlisted_operations_cross_boundary() {
    let req = BridgeRequest { version: DIGITAL_LIFE_BRIDGE_VERSION, tenant: "tenant".into(), sender: "agent".into(), operation: "execute_shell".into(), payload: vec![] };
    assert_eq!(req.validate(), Err(BridgeError::ForbiddenOperation));
}
#[test]
fn bounded_payload_and_roundtrip_contract() {
    let req = BridgeRequest { version: DIGITAL_LIFE_BRIDGE_VERSION, tenant: "tenant".into(), sender: "agent".into(), operation: "trs.geometry.fisher_rao".into(), payload: vec![1, 2] };
    req.validate().unwrap();
    let encoded = serde_json::to_vec(&req).unwrap();
    let decoded: BridgeRequest = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(req, decoded);
    let reply = BridgeResponse::accepted(vec![3]);
    assert!(reply.accepted);
    assert_eq!(reply.version, DIGITAL_LIFE_BRIDGE_VERSION);
    let oversized = BridgeRequest { payload: vec![0; 65_537], ..req };
    assert_eq!(oversized.validate(), Err(BridgeError::PayloadTooLarge));
}
