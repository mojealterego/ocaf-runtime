//! The client signs the exact Execute request before HMAC and certificate PoP.
use ocaf_runtime::omas_signed::{verify_and_consume, SignedOperationError};
use ocaf_runtime::rpc::{RpcAction, RpcClient};
use ocaf_runtime::runtime::ToolCallRequest;
use openssl::pkey::PKey;
use rusqlite::Connection;
use serde_json::json;

const KEY: [u8; 32] = [17; 32];

#[test]
fn client_signs_exact_sender_tenant_call_and_replay_is_denied() {
    let signer = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = signer.raw_public_key().unwrap().try_into().unwrap();
    let private = signer.private_key_to_pem_pkcs8().unwrap();
    let client = RpcClient::new("agent-a", "coord", KEY)
        .unwrap()
        .with_omas_signer("tenant-ci", &private, 2_000_000)
        .unwrap();
    let (request, _) = client
        .prepare(
            RpcAction::Execute {
                tool: "ocaf_ci_noop".into(),
                args: json!({"probe": true}),
            },
            5_000,
        )
        .unwrap();
    let operation = request.omas_operation.unwrap();
    assert_eq!(operation.rpc_sender.as_deref(), Some("agent-a"));
    assert_eq!(operation.tenant, "tenant-ci");
    assert_eq!(operation.target, "tenant-ci/tools/ocaf_ci_noop");
    let call: ToolCallRequest = serde_json::from_slice(&operation.payload).unwrap();
    assert_eq!(call.call_id, request.request_id);
    assert_eq!(call.tool, "ocaf_ci_noop");
    assert_eq!(call.args, json!({"probe": true}));
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("replay.sqlite");
    Connection::open(&db).unwrap().close().unwrap();
    let now = operation.issued_at;
    assert!(verify_and_consume(&operation, &public, &db, now).is_ok());
    assert!(matches!(
        verify_and_consume(&operation, &public, &db, now),
        Err(SignedOperationError::Replay)
    ));
    let mut tampered = operation;
    tampered.rpc_sender = Some("agent-b".into());
    assert!(matches!(
        verify_and_consume(&tampered, &public, &db, now),
        Err(SignedOperationError::Signature)
    ));
}

#[test]
fn malformed_or_missing_signer_does_not_create_an_omas_operation() {
    let base = RpcClient::new("agent-a", "coord", KEY).unwrap();
    let (unsigned, _) = base
        .prepare(
            RpcAction::Execute {
                tool: "safe".into(),
                args: json!({}),
            },
            5_000,
        )
        .unwrap();
    assert!(unsigned.omas_operation.is_none());
    let signer = PKey::generate_ed25519().unwrap();
    let private = signer.private_key_to_pem_pkcs8().unwrap();
    assert!(RpcClient::new("agent-a", "coord", KEY)
        .unwrap()
        .with_omas_signer("../another-tenant", &private, 100)
        .is_err());
    assert!(RpcClient::new("agent-a", "coord", KEY)
        .unwrap()
        .with_omas_signer("tenant-ci", b"invalid private key", 100)
        .is_err());
}
