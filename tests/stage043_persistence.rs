//! Stage 0.43 persistence: verified manifests survive process restarts,
//! are isolated by tenant, and revoked entries never become executable.
use ocaf_runtime::omas_signed::{SignedOperation, SignedOperationError};
use ocaf_runtime::omat_server::{ExecutorKind, ToolManifest, ToolRisk};
use ocaf_runtime::stage043_signed_manifest::SignedToolManifest;
use ocaf_runtime::stage043_store::{Stage043ToolStore, StoreError, ToolRevocationRequest};
use openssl::pkey::{PKey, Private};
use openssl::sign::Signer;
use rusqlite::Connection;
use serde_json::json;
use std::collections::BTreeSet;

const NOW: i64 = 1_800_000_000;

fn signed(tenant: &str) -> (SignedToolManifest, PKey<Private>, [u8; 32]) {
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let mut envelope = SignedToolManifest {
        manifest: ToolManifest {
            tenant_id: tenant.into(),
            tool_id: "echo".into(),
            version: "v1".into(),
            description: "bounded".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            capabilities: BTreeSet::from(["read".into()]),
            risk: ToolRisk::ReadOnly,
            executor: ExecutorKind::Wasi,
            timeout_ms: 100,
            max_output_bytes: 1024,
            enabled: true,
        },
        signer_id: "trusted-operator".into(),
        issued_at: NOW - 1,
        expires_at: NOW + 100,
        signature: Vec::new(),
    };
    envelope.signature = Signer::new_without_digest(&key)
        .unwrap()
        .sign_oneshot_to_vec(&envelope.canonical_bytes().unwrap())
        .unwrap();
    (envelope, key, public)
}

fn revoke_request(tenant: &str, digest: &str) -> ToolRevocationRequest {
    ToolRevocationRequest {
        tenant_id: tenant.into(),
        tool_id: "echo".into(),
        manifest_sha256: digest.into(),
    }
}

fn sign_revoke(
    request: &ToolRevocationRequest,
    key: &PKey<Private>,
    nonce: &str,
) -> SignedOperation {
    let mut operation = SignedOperation {
        tenant: request.tenant_id.clone(),
        namespace: format!("{}/tools", request.tenant_id),
        target: format!("{}/tools/control/revoke", request.tenant_id),
        opcode: 5,
        fuel: 1,
        issued_at: NOW,
        nonce: nonce.into(),
        payload: serde_json::to_vec(request).unwrap(),
        rpc_sender: Some("operator-node".into()),
        signature: Vec::new(),
    };
    operation.signature = Signer::new_without_digest(key)
        .unwrap()
        .sign_oneshot_to_vec(&operation.canonical_bytes())
        .unwrap();
    operation
}

fn existing_store() -> (tempfile::TempDir, std::path::PathBuf, Stage043ToolStore) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tools.sqlite");
    Connection::open(&path).unwrap().close().unwrap();
    let store = Stage043ToolStore::open_existing(&path).unwrap();
    (root, path, store)
}

#[test]
fn verified_manifest_survives_restart_without_cross_tenant_disclosure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tools.sqlite");
    Connection::open(&path).unwrap().close().unwrap();
    let (signed, _, key) = signed("tenant-a");
    {
        let store = Stage043ToolStore::open_existing(&path).unwrap();
        store
            .register_signed(&signed, "tenant-a", &key, NOW)
            .unwrap();
    }
    let reopened = Stage043ToolStore::open_existing(&path).unwrap();
    assert!(reopened.get_active("tenant-a", "echo").unwrap().is_some());
    assert!(reopened.get_active("tenant-b", "echo").unwrap().is_none());
}

#[test]
fn revoked_manifest_cannot_be_replayed_after_restart() {
    let (_root, path, store) = existing_store();
    let (signed, signing_key, public) = signed("tenant-a");
    let digest = store
        .register_signed(&signed, "tenant-a", &public, NOW)
        .unwrap();
    let request = revoke_request("tenant-a", &digest);
    let operation = sign_revoke(&request, &signing_key, "revoke_restart_nonce_0001");
    store
        .revoke_signed(
            &request,
            &operation,
            "operator-node",
            "tenant-a",
            &public,
            NOW,
        )
        .unwrap();
    drop(store);
    let reopened = Stage043ToolStore::open_existing(&path).unwrap();
    assert!(reopened.get_active("tenant-a", "echo").unwrap().is_none());
    assert!(matches!(
        reopened.revoke_signed(
            &request,
            &operation,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Replay)
    ));
    assert!(matches!(
        reopened.register_signed(&signed, "tenant-a", &public, NOW),
        Err(StoreError::Duplicate)
    ));
}

#[test]
fn revoke_rejects_wrong_sender_key_tenant_payload_and_nonce() {
    let (_root, _path, store) = existing_store();
    let (manifest, signing_key, public) = signed("tenant-a");
    let digest = store
        .register_signed(&manifest, "tenant-a", &public, NOW)
        .unwrap();
    let request = revoke_request("tenant-a", &digest);
    let operation = sign_revoke(&request, &signing_key, "revoke_binding_nonce_001");

    assert!(matches!(
        store.revoke_signed(&request, &operation, "other-node", "tenant-a", &public, NOW),
        Err(StoreError::Binding)
    ));
    let mut changed_sender = operation.clone();
    changed_sender.rpc_sender = Some("other-node".into());
    assert!(matches!(
        store.revoke_signed(
            &request,
            &changed_sender,
            "other-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Operation(SignedOperationError::Signature))
    ));
    let mut downgraded = operation.clone();
    downgraded.rpc_sender = None;
    assert!(matches!(
        store.revoke_signed(
            &request,
            &downgraded,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Binding)
    ));

    let unrelated_key = PKey::generate_ed25519().unwrap();
    let unrelated_public: [u8; 32] = unrelated_key.raw_public_key().unwrap().try_into().unwrap();
    assert!(matches!(
        store.revoke_signed(
            &request,
            &operation,
            "operator-node",
            "tenant-a",
            &unrelated_public,
            NOW
        ),
        Err(StoreError::Operation(SignedOperationError::Signature))
    ));
    let forged = sign_revoke(&request, &unrelated_key, "revoke_binding_nonce_001");
    assert!(matches!(
        store.revoke_signed(&request, &forged, "operator-node", "tenant-a", &public, NOW),
        Err(StoreError::Operation(SignedOperationError::Signature))
    ));

    let cross_tenant = revoke_request("tenant-b", &digest);
    let cross_tenant_op = sign_revoke(&cross_tenant, &signing_key, "revoke_binding_nonce_001");
    assert!(matches!(
        store.revoke_signed(
            &cross_tenant,
            &cross_tenant_op,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Binding)
    ));
    let mut other_tool = request.clone();
    other_tool.tool_id = "other".into();
    let other_tool_op = sign_revoke(&other_tool, &signing_key, "revoke_binding_nonce_002");
    assert!(matches!(
        store.revoke_signed(
            &other_tool,
            &other_tool_op,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store.revoke_signed(
            &other_tool,
            &other_tool_op,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Replay)
    ));
    let mut tampered_payload = operation.clone();
    tampered_payload.payload.push(b' ');
    assert!(matches!(
        store.revoke_signed(
            &request,
            &tampered_payload,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Binding)
    ));
    let mut tampered_nonce = operation.clone();
    tampered_nonce.nonce = "revoke_binding_nonce_002".into();
    assert!(matches!(
        store.revoke_signed(
            &request,
            &tampered_nonce,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Operation(SignedOperationError::Signature))
    ));
    assert!(store.get_active("tenant-a", "echo").unwrap().is_some());
    store
        .revoke_signed(
            &request,
            &operation,
            "operator-node",
            "tenant-a",
            &public,
            NOW,
        )
        .unwrap();
}

#[test]
fn revoke_rejects_expired_future_and_stale_digest_then_accepts_exact_digest() {
    let (_root, path, store) = existing_store();
    let (manifest, signing_key, public) = signed("tenant-a");
    let digest = store
        .register_signed(&manifest, "tenant-a", &public, NOW)
        .unwrap();
    let request = revoke_request("tenant-a", &digest);
    let operation = sign_revoke(&request, &signing_key, "revoke_freshness_nonce_01");

    for issued_at in [NOW - 301, NOW + 301] {
        let mut out_of_window = operation.clone();
        out_of_window.issued_at = issued_at;
        out_of_window.signature = Signer::new_without_digest(&signing_key)
            .unwrap()
            .sign_oneshot_to_vec(&out_of_window.canonical_bytes())
            .unwrap();
        assert!(matches!(
            store.revoke_signed(
                &request,
                &out_of_window,
                "operator-node",
                "tenant-a",
                &public,
                NOW
            ),
            Err(StoreError::Operation(SignedOperationError::Invalid))
        ));
    }
    let stale = revoke_request("tenant-a", &format!("sha256:{}", "0".repeat(64)));
    let stale_operation = sign_revoke(&stale, &signing_key, "revoke_freshness_nonce_02");
    assert!(matches!(
        store.revoke_signed(
            &stale,
            &stale_operation,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::StaleDigest)
    ));
    let reopened = Stage043ToolStore::open_existing(&path).unwrap();
    assert!(matches!(
        reopened.revoke_signed(
            &stale,
            &stale_operation,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Replay)
    ));
    let mut malformed = request.clone();
    malformed.manifest_sha256 = digest.to_uppercase();
    let malformed_operation = sign_revoke(&malformed, &signing_key, "revoke_freshness_nonce_01");
    assert!(matches!(
        store.revoke_signed(
            &malformed,
            &malformed_operation,
            "operator-node",
            "tenant-a",
            &public,
            NOW
        ),
        Err(StoreError::Binding)
    ));
    store
        .revoke_signed(
            &request,
            &operation,
            "operator-node",
            "tenant-a",
            &public,
            NOW,
        )
        .unwrap();
    assert!(store.get_active("tenant-a", "echo").unwrap().is_none());
}

#[test]
fn concurrent_signed_revocation_commits_one_tombstone_and_one_nonce() {
    let (_root, path, store) = existing_store();
    let (manifest, signing_key, public) = signed("tenant-a");
    let digest = store
        .register_signed(&manifest, "tenant-a", &public, NOW)
        .unwrap();
    let request = revoke_request("tenant-a", &digest);
    let operation = sign_revoke(&request, &signing_key, "revoke_concurrent_nonce_01");
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let store = store.clone();
            let request = request.clone();
            let operation = operation.clone();
            std::thread::spawn(move || {
                store.revoke_signed(
                    &request,
                    &operation,
                    "operator-node",
                    "tenant-a",
                    &public,
                    NOW,
                )
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(StoreError::Replay)))
            .count(),
        3
    );
    let connection = Connection::open(&path).unwrap();
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM stage043_revoke_nonce", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn modified_sqlite_manifest_digest_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tools.sqlite");
    Connection::open(&path).unwrap().close().unwrap();
    let (signed, signing_key, key) = signed("tenant-a");
    let store = Stage043ToolStore::open_existing(&path).unwrap();
    let digest = store
        .register_signed(&signed, "tenant-a", &key, NOW)
        .unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE stage043_tools SET digest = 'sha256:tampered' WHERE tenant = ?1",
            rusqlite::params!["tenant-a"],
        )
        .unwrap();
    assert!(matches!(
        store.get_active("tenant-a", "echo"),
        Err(StoreError::Integrity)
    ));
    // The signed CAS cannot hide a corrupt stored digest.
    let request = revoke_request("tenant-a", &digest);
    let operation = sign_revoke(&request, &signing_key, "revoke_corrupt_nonce_0001");
    assert!(matches!(
        store.revoke_signed(&request, &operation, "operator-node", "tenant-a", &key, NOW),
        Err(StoreError::Integrity)
    ));
    assert!(matches!(
        store.revoke_signed(&request, &operation, "operator-node", "tenant-a", &key, NOW),
        Err(StoreError::Replay)
    ));
}

#[cfg(unix)]
#[test]
fn database_symlink_is_not_followed() {
    let root = tempfile::tempdir().unwrap();
    let actual = root.path().join("real.sqlite");
    let link = root.path().join("link.sqlite");
    Connection::open(&actual).unwrap().close().unwrap();
    std::os::unix::fs::symlink(&actual, &link).unwrap();
    assert!(Stage043ToolStore::open_existing(&link).is_err());
}
