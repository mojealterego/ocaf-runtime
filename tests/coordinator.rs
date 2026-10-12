//! Durable coordinator and signed heartbeat/route lookup regression tests.
use std::collections::HashMap;

use ocaf_runtime::coordinator::{CoordinatorError, CoordinatorStore};
use ocaf_runtime::rpc::{
    PeerPolicy, RpcAction, RpcError, RpcGateway, RpcReply, RpcRequest, RpcStatus, SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::{json, Value};

const NOW: u64 = 1_800_000_000_000;
const A: [u8; 32] = [0xA1; 32];
const B: [u8; 32] = [0xB2; 32];
const C: [u8; 32] = [0xC3; 32];

fn peers() -> HashMap<String, PeerPolicy> {
    HashMap::from([
        ("worker-a".into(), PeerPolicy::new(A, vec!["echo".into(), "image".into()])),
        ("worker-b".into(), PeerPolicy::new(B, vec!["echo".into()])),
        ("client-c".into(), PeerPolicy::new(C, vec!["echo".into()])),
    ])
}

fn gateway(db: &std::path::Path) -> RpcGateway {
    RpcGateway::new("coordinator", OcafRuntime::new(0), peers(), 16)
        .unwrap()
        .with_coordinator_db(db)
        .unwrap()
}

fn request(node: &str, key: &[u8; 32], action: RpcAction) -> Vec<u8> {
    let req = RpcRequest::new(node, "coordinator", NOW, 5_000, action).unwrap();
    SignedFrame::seal(&req, key).unwrap()
}

async fn call(
    gateway: &RpcGateway,
    node: &str,
    key: &[u8; 32],
    action: RpcAction,
    now: u64,
) -> Result<RpcReply, RpcError> {
    let signed = request(node, key, action);
    let result = gateway.handle_at(&signed, now).await?;
    SignedFrame::open(&result, key)
}

#[tokio::test]
async fn signed_heartbeats_register_and_resolve_authorized_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let gateway = gateway(&dir.path().join("coordinator.sqlite"));
    let registered = call(
        &gateway,
        "worker-a",
        &A,
        RpcAction::Heartbeat {
            tools: vec!["echo".into(), "image".into()],
        },
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(registered.status, RpcStatus::Ok);
    assert_eq!(registered.output["node_id"], "worker-a");

    let routed = call(
        &gateway,
        "client-c",
        &C,
        RpcAction::Resolve {
            tool: "echo".into(),
        },
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(routed.status, RpcStatus::Ok);
    assert_eq!(routed.output["node_id"], "worker-a");
}

#[tokio::test]
async fn untrusted_capability_advertisement_is_rejected_before_storage() {
    let dir = tempfile::tempdir().unwrap();
    let gateway = gateway(&dir.path().join("coordinator.sqlite"));
    let forged = request(
        "worker-b",
        &B,
        RpcAction::Heartbeat {
            tools: vec!["admin:write".into()],
        },
    );
    assert!(matches!(
        gateway.handle_at(&forged, NOW).await,
        Err(RpcError::Unauthorized)
    ));
    let lookup = call(
        &gateway,
        "client-c",
        &C,
        RpcAction::Resolve {
            tool: "echo".into(),
        },
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(lookup.status, RpcStatus::Ok);
    assert_eq!(lookup.output, Value::Null);
}

#[tokio::test]
async fn direct_coordinator_store_expires_nodes_and_persists_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("members.sqlite");
    let first = CoordinatorStore::open(&path).unwrap();
    first
        .heartbeat("worker-a", &["echo".into()], NOW)
        .await
        .unwrap();
    drop(first);

    let restarted = CoordinatorStore::open(&path).unwrap();
    assert_eq!(
        restarted.resolve("echo", NOW + 14_999).await.unwrap().unwrap().node_id,
        "worker-a"
    );
    assert_eq!(restarted.resolve("echo", NOW + 15_000).await.unwrap(), None);
}

#[tokio::test]
async fn coordinator_routes_round_robin_among_live_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let store = CoordinatorStore::open(&dir.path().join("members.sqlite")).unwrap();
    store.heartbeat("worker-a", &["echo".into()], NOW).await.unwrap();
    store.heartbeat("worker-b", &["echo".into()], NOW).await.unwrap();
    let mut ids = vec![];
    for _ in 0..4 {
        ids.push(store.resolve("echo", NOW).await.unwrap().unwrap().node_id);
    }
    assert_eq!(ids, vec!["worker-a", "worker-b", "worker-a", "worker-b"]);
}

#[tokio::test]
async fn coordinator_rejects_invalid_capabilities_and_fails_closed_on_bad_storage() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        CoordinatorStore::open(&dir.path().join("missing").join("nodes.sqlite")),
        Err(CoordinatorError::Storage)
    ));
    let store = CoordinatorStore::open(&dir.path().join("members.sqlite")).unwrap();
    assert!(matches!(
        store.heartbeat("bad/node", &["echo".into()], NOW).await,
        Err(CoordinatorError::Invalid)
    ));
    assert!(matches!(
        store.heartbeat("worker-a", &["echo".into(), "echo".into()], NOW).await,
        Err(CoordinatorError::Invalid)
    ));
    assert!(matches!(
        store.heartbeat("worker-a", &[], NOW).await,
        Err(CoordinatorError::Invalid)
    ));
}

#[tokio::test]
async fn expired_heartbeats_cannot_resolve_even_if_old_data_was_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let gateway = gateway(&dir.path().join("members.sqlite"));
    let original = request(
        "worker-a",
        &A,
        RpcAction::Heartbeat {
            tools: vec!["echo".into()],
        },
    );
    gateway.handle_at(&original, NOW).await.unwrap();
    assert!(matches!(
        gateway.handle_at(&original, NOW + 1).await,
        Err(RpcError::Replay)
    ));
    let routed = call(
        &gateway,
        "client-c",
        &C,
        RpcAction::Resolve {
            tool: "echo".into(),
        },
        NOW + 15_000,
    )
    .await
    .unwrap();
    assert_eq!(routed.output, json!(null));
}

#[tokio::test]
async fn unauthorized_resolve_cannot_enumerate_disallowed_tools() {
    let dir = tempfile::tempdir().unwrap();
    let gateway = gateway(&dir.path().join("members.sqlite"));
    let denied = request(
        "client-c",
        &C,
        RpcAction::Resolve {
            tool: "image".into(),
        },
    );
    assert!(matches!(
        gateway.handle_at(&denied, NOW).await,
        Err(RpcError::Unauthorized)
    ));
}

#[tokio::test]
async fn heartbeat_renewal_replaces_capabilities_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let store = CoordinatorStore::open(&dir.path().join("members.sqlite")).unwrap();
    store
        .heartbeat("worker-a", &["echo".into(), "image".into()], NOW)
        .await
        .unwrap();
    store
        .heartbeat("worker-a", &["image".into()], NOW + 1000)
        .await
        .unwrap();
    assert!(store.resolve("echo", NOW + 1000).await.unwrap().is_none());
    assert_eq!(
        store.resolve("image", NOW + 1000).await.unwrap().unwrap().node_id,
        "worker-a"
    );
}
