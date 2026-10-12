//! PostgreSQL transactional effect sink: the lease/epoch check and the
//! local durable effect are committed in the SAME database transaction.
use std::collections::HashMap;
use std::sync::Arc;

use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::jobs::LeasedJob;
use ocaf_runtime::rpc::{
    PeerPolicy, RpcAction, RpcGateway, RpcReply, RpcRequest, RpcStatus, SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::json;

const NOW: u64 = 1_800_000_000_000;
const OWNER_KEY: [u8; 32] = [0xA1; 32];
const WORKER_KEY: [u8; 32] = [0xA2; 32];

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    value
}

async fn gateway(dsn: &str, tool: &str, leader: Arc<PgLeader>) -> RpcGateway {
    RpcGateway::new(
        "effects-coord",
        OcafRuntime::new(0),
        HashMap::from([
            (
                "effects-owner".into(),
                PeerPolicy::new(OWNER_KEY, vec![tool.into()]).allow_job_submission(),
            ),
            (
                "effects-worker".into(),
                PeerPolicy::new(WORKER_KEY, vec![tool.into()]).allow_job_worker(),
            ),
        ]),
        16,
    )
    .unwrap()
    .with_postgres_replay(dsn, true)
    .await
    .unwrap()
    .with_postgres_job_db(dsn, true)
    .await
    .unwrap()
    .with_pg_leader(leader)
}

async fn send(gateway: &RpcGateway, worker: bool, action: RpcAction) -> RpcReply {
    let (sender, key) = if worker {
        ("effects-worker", &WORKER_KEY)
    } else {
        ("effects-owner", &OWNER_KEY)
    };
    let request = RpcRequest::new(sender, "effects-coord", NOW, 5000, action).unwrap();
    let frame = SignedFrame::seal(&request, key).unwrap();
    let output = gateway.handle_at(&frame, NOW).await.unwrap();
    SignedFrame::open(&output, key).unwrap()
}

async fn claimed(gateway: &RpcGateway, tool: &str) -> LeasedJob {
    send(
        gateway,
        false,
        RpcAction::SubmitJob {
            id: uuid::Uuid::new_v4().to_string(),
            tool: tool.into(),
            args: json!({"task": "fenced effect"}),
        },
    )
    .await;
    let reply = send(gateway, true, RpcAction::ClaimJob).await;
    assert_eq!(reply.status, RpcStatus::Ok);
    let lease: LeasedJob = serde_json::from_value(reply.output).unwrap();
    assert_eq!(lease.tool, tool);
    assert!(lease.leader_epoch > 0);
    lease
}

async fn effect_in_db(url: &str, job_id: &str) -> Option<String> {
    let (client, conn) = tokio_postgres::connect(url, tokio_postgres::NoTls)
        .await
        .unwrap();
    let driver = tokio::spawn(async move {
        let _ = conn.await;
    });
    let row = client
        .query_opt(
            "SELECT effect FROM ocaf_pg_fenced_effects_v1 WHERE job_id=$1",
            &[&job_id],
        )
        .await
        .unwrap();
    driver.abort();
    row.map(|r| r.get(0))
}

#[tokio::test]
async fn signed_sink_commits_once_and_never_replaces_a_different_payload() {
    let Some(url) = dsn() else { return };
    let cluster = format!("sink_{}", uuid::Uuid::new_v4().simple());
    let tool = format!("effect_{}", uuid::Uuid::new_v4().simple());
    let leader = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    assert!(leader.try_acquire().await.unwrap());
    let gateway = gateway(&url, &tool, Arc::clone(&leader)).await;
    let lease = claimed(&gateway, &tool).await;
    let effect = json!({"resource": "fenced-demo", "value": 7});

    let first = send(
        &gateway,
        true,
        RpcAction::ApplyFencedEffect {
            lease: lease.clone(),
            effect: effect.clone(),
        },
    )
    .await;
    assert_eq!(first.status, RpcStatus::Ok);
    assert_eq!(first.output["applied"], true);
    assert_eq!(first.output["job_id"], lease.id);
    assert_eq!(first.output["leader_epoch"], lease.leader_epoch);

    let duplicate = send(
        &gateway,
        true,
        RpcAction::ApplyFencedEffect {
            lease: lease.clone(),
            effect: effect.clone(),
        },
    )
    .await;
    assert_eq!(duplicate.status, RpcStatus::Ok);
    assert_eq!(duplicate.output["applied"], false);
    assert_eq!(
        duplicate.output["committed_at_ms"],
        first.output["committed_at_ms"]
    );

    let conflict = send(
        &gateway,
        true,
        RpcAction::ApplyFencedEffect {
            lease: lease.clone(),
            effect: json!({"resource": "fenced-demo", "value": 8}),
        },
    )
    .await;
    assert_eq!(conflict.status, RpcStatus::Failed);
    assert_eq!(
        effect_in_db(&url, &lease.id).await,
        Some(serde_json::to_string(&effect).unwrap())
    );
    leader.release().await.unwrap();
}

#[tokio::test]
async fn promoted_leader_fences_old_worker_effect_and_expired_job() {
    let Some(url) = dsn() else { return };
    let cluster = format!("sink_fence_{}", uuid::Uuid::new_v4().simple());
    let tool = format!("effect_{}", uuid::Uuid::new_v4().simple());
    let first_leader = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    let second_leader = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    assert!(first_leader.try_acquire().await.unwrap());
    let first = gateway(&url, &tool, Arc::clone(&first_leader)).await;
    let old = claimed(&first, &tool).await;
    first_leader.release().await.unwrap();

    assert!(second_leader.try_acquire().await.unwrap());
    let promoted = gateway(&url, &tool, Arc::clone(&second_leader)).await;
    let rejected = send(
        &promoted,
        true,
        RpcAction::ApplyFencedEffect {
            lease: old.clone(),
            effect: json!({"must_not_write": true}),
        },
    )
    .await;
    assert_eq!(rejected.status, RpcStatus::Failed);
    assert_eq!(effect_in_db(&url, &old.id).await, None);

    let live = claimed(&promoted, &tool).await;
    assert!(live.leader_epoch > old.leader_epoch);
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .unwrap();
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    assert_eq!(
        client
            .execute(
                "UPDATE ocaf_pg_jobs_v1
                 SET lease_expires_at_ms =
                   (EXTRACT(EPOCH FROM clock_timestamp())*1000)::BIGINT - 1
                 WHERE id=$1 AND status='running'",
                &[&live.id],
            )
            .await
            .unwrap(),
        1
    );
    let expired = send(
        &promoted,
        true,
        RpcAction::ApplyFencedEffect {
            lease: live.clone(),
            effect: json!({"must_not_write": "expired"}),
        },
    )
    .await;
    assert_eq!(expired.status, RpcStatus::Failed);
    assert_eq!(effect_in_db(&url, &live.id).await, None);
    driver.abort();
    second_leader.release().await.unwrap();
}

#[tokio::test]
async fn a_job_worker_without_explicit_effect_permission_cannot_write_sink() {
    use ocaf_runtime::rpc::RpcError;

    let Some(url) = dsn() else { return };
    let cluster = format!("acl_{}", uuid::Uuid::new_v4().simple());
    let tool = format!("effect_{}", uuid::Uuid::new_v4().simple());
    let leader = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    assert!(leader.try_acquire().await.unwrap());
    let authorized = gateway(&url, &tool, Arc::clone(&leader)).await;
    let lease = claimed(&authorized, &tool).await;

    let unprivileged = RpcGateway::new(
        "effects-coord",
        OcafRuntime::new(0),
        HashMap::from([(
            "effects-worker".to_string(),
            PeerPolicy::new(WORKER_KEY, vec![tool]).allow_job_worker(),
        )]),
        16,
    )
    .unwrap()
    .with_postgres_job_db(&url, true)
    .await
    .unwrap()
    .with_pg_leader(Arc::clone(&leader));

    let req = RpcRequest::new(
        "effects-worker",
        "effects-coord",
        NOW,
        5_000,
        RpcAction::ApplyFencedEffect {
            lease: lease.clone(),
            effect: json!({"unauthorized": true}),
        },
    )
    .unwrap();
    let signed = SignedFrame::seal(&req, &WORKER_KEY).unwrap();
    assert!(matches!(
        unprivileged.handle_at(&signed, NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert_eq!(effect_in_db(&url, &lease.id).await, None);
    leader.release().await.unwrap();
}
