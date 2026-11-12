use ocaf_runtime::trs_digital_life::{AuthenticatedPrincipal, BridgeRequest, DIGITAL_LIFE_BRIDGE_VERSION};
use ocaf_runtime::trs_digital_life_durable::{dispatch_sealed, DurableBridgeError};
use ocaf_runtime::trs_seal::SealKeys;
use ocaf_runtime::trs_seal_sqlite::{advance_fencing_epoch, initialize};
use rusqlite::Connection;
#[test]
fn invalid_math_payload_does_not_consume_replay_slot() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 advance_fencing_epoch(&mut db,"tenant",7).unwrap();
 let keys=SealKeys::new(1,&[0x53;32]).unwrap();
 let principal=AuthenticatedPrincipal {tenant:"tenant".into(),sender:"worker".into(),allowed_operations:vec!["trs.geometry.fisher_rao".into()]};
 let mut request=BridgeRequest {version:DIGITAL_LIFE_BRIDGE_VERSION,tenant:"tenant".into(),sender:"worker".into(),operation:"trs.geometry.fisher_rao".into(),payload:br#"{"p":[0.5,0.5],"q":[0.0,1.0]}"#.to_vec()};
 let invalid=keys.sign("tenant","worker",7,1,&serde_json::to_vec(&request).unwrap()).unwrap();
 assert!(matches!(dispatch_sealed(&mut db,&keys,&principal,&invalid),Err(DurableBridgeError::Bridge(_))));
 request.payload=br#"{"p":[0.5,0.5],"q":[0.5,0.5]}"#.to_vec();
 let valid=keys.sign("tenant","worker",7,1,&serde_json::to_vec(&request).unwrap()).unwrap();
 assert!(dispatch_sealed(&mut db,&keys,&principal,&valid).is_ok());
}
#[test]
fn tampered_mac_does_not_trigger_computation_or_claim_slot() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 advance_fencing_epoch(&mut db,"tenant",7).unwrap();
 let keys=SealKeys::new(1,&[0x53;32]).unwrap();
 let principal=AuthenticatedPrincipal {tenant:"tenant".into(),sender:"worker".into(),allowed_operations:vec!["trs.health".into()]};
 let request=BridgeRequest {version:DIGITAL_LIFE_BRIDGE_VERSION,tenant:"tenant".into(),sender:"worker".into(),operation:"trs.health".into(),payload:vec![]};
 let valid=keys.sign("tenant","worker",7,1,&serde_json::to_vec(&request).unwrap()).unwrap();
 let mut invalid=valid.clone();invalid.tag[0]^=0x01;
 assert!(dispatch_sealed(&mut db,&keys,&principal,&invalid).is_err());
 assert!(dispatch_sealed(&mut db,&keys,&principal,&valid).is_ok());
}
