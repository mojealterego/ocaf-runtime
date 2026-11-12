use ocaf_runtime::trs_digital_life::{AuthenticatedPrincipal, BridgeRequest, DIGITAL_LIFE_BRIDGE_VERSION};
use ocaf_runtime::trs_digital_life_durable::{dispatch_sealed, DurableBridgeError};
use ocaf_runtime::trs_seal::SealKeys;
use ocaf_runtime::trs_seal_sqlite::{advance_fencing_epoch, initialize};
use rusqlite::Connection;
fn request() -> BridgeRequest {
    BridgeRequest {version:DIGITAL_LIFE_BRIDGE_VERSION,tenant:"tenant-a".into(),sender:"worker".into(),operation:"trs.health".into(),payload:vec![]}
}
fn principal() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal {tenant:"tenant-a".into(),sender:"worker".into(),allowed_operations:vec!["trs.health".into()]}
}
#[test]
fn sealed_dispatch_rejects_replay_across_connections() {
    let file=tempfile::NamedTempFile::new().unwrap();
    let keys=SealKeys::new(1,&[0x44;32]).unwrap();
    let payload=serde_json::to_vec(&request()).unwrap();
    let env=keys.sign("tenant-a","worker",7,1,&payload).unwrap();
    {
        let mut conn=Connection::open(file.path()).unwrap();
        initialize(&conn).unwrap();
        advance_fencing_epoch(&mut conn,"tenant-a",7).unwrap();
        assert!(dispatch_sealed(&mut conn,&keys,&principal(),&env).is_ok());
    }
    let mut conn=Connection::open(file.path()).unwrap();
    assert!(matches!(dispatch_sealed(&mut conn,&keys,&principal(),&env),Err(DurableBridgeError::Admission(_))));
}
#[test]
fn mismatched_request_identity_is_rejected_before_admission() {
    let mut conn=Connection::open_in_memory().unwrap();
    initialize(&conn).unwrap();
    advance_fencing_epoch(&mut conn,"tenant-a",7).unwrap();
    let keys=SealKeys::new(1,&[0x44;32]).unwrap();
    let mut wrong=request();wrong.sender="attacker".into();
    let bad=keys.sign("tenant-a","worker",7,1,&serde_json::to_vec(&wrong).unwrap()).unwrap();
    assert!(matches!(dispatch_sealed(&mut conn,&keys,&principal(),&bad),Err(DurableBridgeError::Unauthorized)));
    let good=keys.sign("tenant-a","worker",7,1,&serde_json::to_vec(&request()).unwrap()).unwrap();
    assert!(dispatch_sealed(&mut conn,&keys,&principal(),&good).is_ok());
}
