use ocaf_runtime::trs_digital_life::{dispatch_local, BridgeError, BridgeRequest, DIGITAL_LIFE_BRIDGE_VERSION};
use serde_json::json;
fn req(op:&str, payload:serde_json::Value)->BridgeRequest {
    BridgeRequest {version:DIGITAL_LIFE_BRIDGE_VERSION,tenant:"tenant".into(),sender:"agent".into(),operation:op.into(),payload:serde_json::to_vec(&payload).unwrap()}
}
#[test]
fn fisher_distance_executes_with_typed_result() {
    let response=dispatch_local(&req("trs.geometry.fisher_rao",json!({"p":[0.5,0.5],"q":[0.5,0.5]}))).unwrap();
    let result:serde_json::Value=serde_json::from_slice(&response.payload).unwrap();
    assert!(result["distance"].as_f64().unwrap()<1e-7);
}
#[test]
fn hodge_laplacian_executes() {
    let response=dispatch_local(&req("trs.geometry.hodge_laplacian",json!({"values":[1.0,2.0],"edges":[[0,1]]}))).unwrap();
    let result:serde_json::Value=serde_json::from_slice(&response.payload).unwrap();
    assert_eq!(result["values"],json!([-1.0,1.0]));
}
#[test]
fn sat_is_bounded_and_correct() {
    let response=dispatch_local(&req("trs.math.sat_evaluate",json!({"variables":1,"clauses":[[1,1,1]]}))).unwrap();
    let result:serde_json::Value=serde_json::from_slice(&response.payload).unwrap();
    assert_eq!(result["satisfying_count"],1);
    assert_eq!(dispatch_local(&req("trs.math.sat_evaluate",json!({"variables":17,"clauses":[]}))),Err(BridgeError::InvalidPayload));
}
#[test]
fn unknown_fields_and_malformed_payload_fail_closed() {
    assert_eq!(dispatch_local(&req("trs.geometry.fisher_rao",json!({"p":[0.5,0.5],"q":[0.5,0.5],"secret":1}))),Err(BridgeError::InvalidPayload));
    assert_eq!(dispatch_local(&req("trs.geometry.hodge_laplacian",json!({"values":[1.0],"edges":[[0,9]]}))),Err(BridgeError::InvalidPayload));
}
