#![cfg(feature = "zenoh")]
use ocaf_runtime::omas_signed::SignedOperation;
use ocaf_runtime::omat_server::{ExecutorKind, ToolManifest, ToolRisk};
use ocaf_runtime::stage043_omat_revoke_zenoh::{
    request_revoke_zenoh, serve_revoke_zenoh_until, RevokeAuthority, SignedRevokeFrame,
};
use ocaf_runtime::stage043_signed_manifest::SignedToolManifest;
use ocaf_runtime::stage043_store::{Stage043ToolStore, ToolRevocationRequest};
use openssl::pkey::{PKey, Private};
use openssl::sign::Signer;
use rusqlite::Connection;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn sign_op(
    key: &PKey<Private>,
    request: ToolRevocationRequest,
    now: i64,
    nonce: &str,
) -> SignedRevokeFrame {
    let mut operation = SignedOperation {
        tenant: request.tenant_id.clone(),
        namespace: format!("{}/tools", request.tenant_id),
        target: format!("{}/tools/control/revoke", request.tenant_id),
        opcode: 5,
        fuel: 1,
        issued_at: now,
        nonce: nonce.into(),
        payload: serde_json::to_vec(&request).unwrap(),
        rpc_sender: Some("operator-node".into()),
        signature: vec![],
    };
    operation.signature = Signer::new_without_digest(key)
        .unwrap()
        .sign_oneshot_to_vec(&operation.canonical_bytes())
        .unwrap();
    SignedRevokeFrame { request, operation }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_zenoh_revoke_is_durable_bound_and_replay_safe() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tools.db");
    Connection::open(&path).unwrap().close().unwrap();
    let store = Stage043ToolStore::open_existing(&path).unwrap();
    let mut manifest = SignedToolManifest {
        manifest: ToolManifest {
            tenant_id: "tenant_a".into(),
            tool_id: "echo".into(),
            version: "v1".into(),
            description: "safe".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            capabilities: BTreeSet::from(["read".into()]),
            risk: ToolRisk::ReadOnly,
            executor: ExecutorKind::Wasi,
            timeout_ms: 100,
            max_output_bytes: 1024,
            enabled: true,
        },
        signer_id: "operator".into(),
        issued_at: now - 1,
        expires_at: now + 120,
        signature: vec![],
    };
    manifest.signature = Signer::new_without_digest(&key)
        .unwrap()
        .sign_oneshot_to_vec(&manifest.canonical_bytes().unwrap())
        .unwrap();
    let digest = store
        .register_signed(&manifest, "tenant_a", &public, now)
        .unwrap();
    let authority = RevokeAuthority {
        tenant: "tenant_a".into(),
        sender: "operator-node".into(),
        public_key: public,
        store: store.clone(),
    };
    let mut config = zenoh::Config::default();
    config.insert_json5("mode", "\"peer\"").unwrap();
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    config
        .insert_json5("scouting/gossip/enabled", "false")
        .unwrap();
    let session = Arc::new(zenoh::open(config).await.unwrap());
    let selector = authority.selector();
    let serving = Arc::clone(&session);
    let task = tokio::spawn(async move {
        serve_revoke_zenoh_until(serving, authority, std::future::pending()).await
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    let request = ToolRevocationRequest {
        tenant_id: "tenant_a".into(),
        tool_id: "echo".into(),
        manifest_sha256: digest,
    };
    let invalid = sign_op(&key, request.clone(), now, "revoke_wrong_selector");
    assert!(
        request_revoke_zenoh(&session, "ocaf/omat/tenant_b/control/revoke", &invalid)
            .await
            .is_err()
    );
    assert!(store.get_active("tenant_a", "echo").unwrap().is_some());
    let mut forged = sign_op(&key, request.clone(), now, "revoke_forged_signature");
    forged.operation.signature[0] ^= 1;
    assert!(request_revoke_zenoh(&session, &selector, &forged)
        .await
        .is_err());
    assert!(store.get_active("tenant_a", "echo").unwrap().is_some());
    let frame = sign_op(&key, request, now, "revoke_valid_nonce_1");
    request_revoke_zenoh(&session, &selector, &frame)
        .await
        .unwrap();
    assert!(store.get_active("tenant_a", "echo").unwrap().is_none());
    assert!(request_revoke_zenoh(&session, &selector, &frame)
        .await
        .is_err());
    let reopened = Stage043ToolStore::open_existing(&path).unwrap();
    assert!(reopened.get_active("tenant_a", "echo").unwrap().is_none());
    task.abort();
}
