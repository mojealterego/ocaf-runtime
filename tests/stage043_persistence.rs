//! Stage 0.43 persistence: verified manifests survive process restarts,
//! are isolated by tenant, and revoked entries never become executable.
use ocaf_runtime::omat_server::{ExecutorKind, ToolManifest, ToolRisk};
use ocaf_runtime::stage043_signed_manifest::SignedToolManifest;
use ocaf_runtime::stage043_store::{Stage043ToolStore, StoreError};
use openssl::pkey::PKey;
use openssl::sign::Signer;
use rusqlite::Connection;
use serde_json::json;
use std::collections::BTreeSet;

const NOW: i64 = 1_800_000_000;

fn signed(tenant: &str) -> (SignedToolManifest, [u8; 32]) {
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
    (envelope, public)
}

#[test]
fn verified_manifest_survives_restart_without_cross_tenant_disclosure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tools.sqlite");
    Connection::open(&path).unwrap().close().unwrap();
    let (signed, key) = signed("tenant-a");
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
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tools.sqlite");
    Connection::open(&path).unwrap().close().unwrap();
    let (signed, key) = signed("tenant-a");
    let store = Stage043ToolStore::open_existing(&path).unwrap();
    store
        .register_signed(&signed, "tenant-a", &key, NOW)
        .unwrap();
    store.revoke("tenant-a", "echo").unwrap();
    drop(store);
    let reopened = Stage043ToolStore::open_existing(&path).unwrap();
    assert!(reopened.get_active("tenant-a", "echo").unwrap().is_none());
    assert!(matches!(
        reopened.register_signed(&signed, "tenant-a", &key, NOW),
        Err(StoreError::Duplicate)
    ));
}

#[test]
fn modified_sqlite_manifest_digest_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tools.sqlite");
    Connection::open(&path).unwrap().close().unwrap();
    let (signed, key) = signed("tenant-a");
    let store = Stage043ToolStore::open_existing(&path).unwrap();
    store
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
