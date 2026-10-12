//! PostgreSQL membership and replay admission across independent gateways.
use ocaf_runtime::cluster::directory::PgDirectory;
use ocaf_runtime::coordinator::NODE_LEASE_MS;
use ocaf_runtime::rpc::{
    PeerPolicy, RpcAction, RpcError, RpcGateway, RpcReply, RpcRequest, RpcStatus, SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use std::collections::HashMap;

const NOW: u64 = 1_800_000_000_000;
const KEY: [u8; 32] = [0xB4; 32];
fn dsn() -> Option<String> {
    let v = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(
        v.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "Postgres required in CI"
    );
    v
}
async fn gateway(dsn: &str, node_id: &str) -> RpcGateway {
    RpcGateway::new(
        node_id,
        OcafRuntime::new(0),
        HashMap::from([("remote-a".into(), PeerPolicy::new(KEY, vec!["echo".into()]))]),
        16,
    )
    .unwrap()
    .with_postgres_replay(dsn, true)
    .await
    .unwrap()
    .with_postgres_coordinator(dsn, true)
    .await
    .unwrap()
}
fn payload(recipient: &str, action: RpcAction) -> Vec<u8> {
    let req = RpcRequest::new("remote-a", recipient, NOW, 5000, action).unwrap();
    SignedFrame::seal(&req, &KEY).unwrap()
}
async fn send(gateway: &RpcGateway, bytes: &[u8], now: u64) -> Result<RpcReply, RpcError> {
    let bytes = gateway.handle_at(bytes, now).await?;
    SignedFrame::open(&bytes, &KEY)
}

#[tokio::test]
async fn replay_is_rejected_by_separate_gateways_and_survives_restart() {
    let Some(dsn) = dsn() else { return };
    let first = gateway(&dsn, "pg-replay-node").await;
    let second = gateway(&dsn, "pg-replay-node").await;
    let msg = payload("pg-replay-node", RpcAction::Discover);
    assert_eq!(send(&first, &msg, NOW).await.unwrap().status, RpcStatus::Ok);
    assert!(matches!(
        send(&second, &msg, NOW + 1).await,
        Err(RpcError::Replay)
    ));
    drop(first);
    let restarted = gateway(&dsn, "pg-replay-node").await;
    assert!(matches!(
        send(&restarted, &msg, NOW + 2).await,
        Err(RpcError::Replay)
    ));
}

#[tokio::test]
async fn signed_heartbeat_on_one_gateway_is_discoverable_on_another() {
    let Some(dsn) = dsn() else { return };
    let a = gateway(&dsn, "pg-members-node").await;
    let b = gateway(&dsn, "pg-members-node").await;
    let hb = payload(
        "pg-members-node",
        RpcAction::Heartbeat {
            tools: vec!["echo".into()],
        },
    );
    assert_eq!(send(&a, &hb, NOW).await.unwrap().status, RpcStatus::Ok);
    let lookup = payload(
        "pg-members-node",
        RpcAction::Resolve {
            tool: "echo".into(),
        },
    );
    let result = send(&b, &lookup, NOW + 2).await.unwrap();
    assert_eq!(result.output["node_id"], "remote-a");
}

#[tokio::test]
async fn membership_replaces_capabilities_and_expires_across_connections() {
    let Some(dsn) = dsn() else { return };
    let a = PgDirectory::connect(&dsn, true).await.unwrap();
    let b = PgDirectory::connect(&dsn, true).await.unwrap();
    let node = format!("node-{}", uuid::Uuid::new_v4().simple());
    let tool = format!("tool_{}", uuid::Uuid::new_v4().simple());
    a.heartbeat(&node, std::slice::from_ref(&tool), NOW)
        .await
        .unwrap();
    assert_eq!(
        b.resolve(&tool, NOW + NODE_LEASE_MS - 1)
            .await
            .unwrap()
            .unwrap()
            .node_id,
        node
    );
    assert_eq!(b.resolve(&tool, NOW + NODE_LEASE_MS).await.unwrap(), None);
    a.heartbeat(&node, &["another".into()], NOW + NODE_LEASE_MS + 1)
        .await
        .unwrap();
    assert_eq!(
        b.resolve(&tool, NOW + NODE_LEASE_MS + 1).await.unwrap(),
        None
    );
    assert_eq!(
        b.resolve("another", NOW + NODE_LEASE_MS + 1)
            .await
            .unwrap()
            .unwrap()
            .node_id,
        node
    );
}

#[tokio::test]
async fn postgres_duplicate_signed_frame_across_gateways_cannot_mutate_membership() {
    let Some(dsn) = dsn() else { return };
    let a = gateway(&dsn, "pg-replay-heartbeat").await;
    let b = gateway(&dsn, "pg-replay-heartbeat").await;
    let hb = payload(
        "pg-replay-heartbeat",
        RpcAction::Heartbeat {
            tools: vec!["echo".into()],
        },
    );
    assert_eq!(send(&a, &hb, NOW).await.unwrap().status, RpcStatus::Ok);
    assert!(matches!(
        send(&b, &hb, NOW + 1).await,
        Err(RpcError::Replay)
    ));
    let fresh = payload(
        "pg-replay-heartbeat",
        RpcAction::Resolve {
            tool: "echo".into(),
        },
    );
    assert_eq!(
        send(&b, &fresh, NOW + 2).await.unwrap().output["node_id"],
        "remote-a"
    );
}

#[tokio::test]
async fn malformed_capabilities_fail_closed_in_postgresql_store() {
    let Some(dsn) = dsn() else { return };
    let directory = PgDirectory::connect(&dsn, true).await.unwrap();
    assert!(directory
        .heartbeat("invalid/peer", &["echo".into()], NOW)
        .await
        .is_err());
    assert!(directory.heartbeat("valid-peer", &[], NOW).await.is_err());
    assert!(directory
        .heartbeat("valid-peer", &["echo".into(), "echo".into()], NOW)
        .await
        .is_err());
    assert!(directory.resolve("", NOW).await.is_err());
}

#[tokio::test]
async fn standby_gateway_fails_closed_before_accepting_signed_requests() {
    use ocaf_runtime::cluster::leadership::PgLeader;
    use std::sync::Arc;
    let Some(dsn) = dsn() else { return };
    let leader = Arc::new(
        PgLeader::connect(&dsn, "pg-leader-gate", true)
            .await
            .unwrap(),
    );
    let standby = Arc::new(
        PgLeader::connect(&dsn, "pg-leader-gate", true)
            .await
            .unwrap(),
    );
    assert!(leader.try_acquire().await.unwrap());
    assert!(!standby.try_acquire().await.unwrap());
    let active = gateway(&dsn, "pg-gated-node")
        .await
        .with_pg_leader(Arc::clone(&leader));
    let passive = gateway(&dsn, "pg-gated-node")
        .await
        .with_pg_leader(Arc::clone(&standby));
    let msg = payload("pg-gated-node", RpcAction::Discover);
    assert!(matches!(
        send(&passive, &msg, NOW).await,
        Err(RpcError::Unavailable)
    ));
    assert_eq!(
        send(&active, &msg, NOW).await.unwrap().status,
        RpcStatus::Ok
    );
    leader.release().await.unwrap();
    assert!(matches!(
        send(
            &active,
            &payload("pg-gated-node", RpcAction::Discover),
            NOW + 1
        )
        .await,
        Err(RpcError::Unavailable)
    ));
    assert!(standby.try_acquire().await.unwrap());
    assert_eq!(
        send(
            &passive,
            &payload("pg-gated-node", RpcAction::Discover),
            NOW + 2
        )
        .await
        .unwrap()
        .status,
        RpcStatus::Ok
    );
}
