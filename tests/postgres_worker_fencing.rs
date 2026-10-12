//! Real PostgreSQL regression: failover invalidates an old worker lease.
use std::collections::HashMap;
use std::sync::Arc;

use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::jobs::LeasedJob;
use ocaf_runtime::rpc::{
    PeerPolicy, RemoteJobOutcome, RpcAction, RpcGateway, RpcReply, RpcRequest, RpcStatus,
    SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::json;

const CLOCK: u64 = 1_800_000_000_000;
const OWNER_KEY: [u8; 32] = [0x47; 32];
const WORKER_KEY: [u8; 32] = [0x48; 32];

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    value
}

async fn gateway(url: &str, tool: &str, leader: Arc<PgLeader>) -> RpcGateway {
    RpcGateway::new(
        "fence-coord",
        OcafRuntime::new(0),
        HashMap::from([
            (
                "fence-owner".into(),
                PeerPolicy::new(OWNER_KEY, vec![tool.into()]).allow_job_submission(),
            ),
            (
                "fence-worker".into(),
                PeerPolicy::new(WORKER_KEY, vec![tool.into()]).allow_job_worker(),
            ),
        ]),
        16,
    )
    .unwrap()
    .with_postgres_job_db(url, true)
    .await
    .unwrap()
    .with_pg_leader(leader)
}

async fn request(gw: &RpcGateway, sender: &str, key: &[u8; 32], action: RpcAction) -> RpcReply {
    let msg = RpcRequest::new(sender, "fence-coord", CLOCK, 5_000, action).unwrap();
    let signed = SignedFrame::seal(&msg, key).unwrap();
    let response = gw.handle_at(&signed, CLOCK).await.unwrap();
    SignedFrame::open(&response, key).unwrap()
}

#[tokio::test]
async fn promoted_leader_rejects_old_generation_worker_report() {
    let Some(url) = dsn() else { return };
    let cluster = format!("epoch_work_{}", uuid::Uuid::new_v4().simple());
    let tool = format!("tool_{}", uuid::Uuid::new_v4().simple());
    let first = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    let second = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    assert!(first.try_acquire().await.unwrap());
    let active = gateway(&url, &tool, Arc::clone(&first)).await;
    let id = uuid::Uuid::new_v4().to_string();
    let submitted = request(
        &active,
        "fence-owner",
        &OWNER_KEY,
        RpcAction::SubmitJob {
            id: id.clone(),
            tool: tool.clone(),
            args: json!({"side_effect":"not yet"}),
        },
    )
    .await;
    assert_eq!(submitted.status, RpcStatus::Ok);
    let claim = request(&active, "fence-worker", &WORKER_KEY, RpcAction::ClaimJob).await;
    let lease: LeasedJob = serde_json::from_value(claim.output).unwrap();
    assert_eq!(lease.leader_epoch, first.fencing_epoch().await.unwrap());
    assert!(lease.leader_epoch > 0);

    let valid = request(
        &active,
        "fence-worker",
        &WORKER_KEY,
        RpcAction::ValidateLease {
            lease: lease.clone(),
        },
    )
    .await;
    assert_eq!(valid.status, RpcStatus::Ok);
    assert_eq!(valid.output["valid"], true);

    first.release().await.unwrap();
    assert!(second.try_acquire().await.unwrap());
    let promoted = gateway(&url, &tool, Arc::clone(&second)).await;
    let denied = request(
        &promoted,
        "fence-worker",
        &WORKER_KEY,
        RpcAction::ValidateLease {
            lease: lease.clone(),
        },
    )
    .await;
    assert_eq!(denied.status, RpcStatus::Failed);
    assert_eq!(denied.output["valid"], false);
    let rejected = request(
        &promoted,
        "fence-worker",
        &WORKER_KEY,
        RpcAction::FinishJob {
            lease,
            outcome: RemoteJobOutcome::Succeeded {
                output: json!({"forged":"success"}),
            },
        },
    )
    .await;
    assert_eq!(rejected.status, RpcStatus::Failed);
    let status = request(
        &promoted,
        "fence-owner",
        &OWNER_KEY,
        RpcAction::GetJob { id },
    )
    .await;
    assert_eq!(status.output["status"], "running");
    assert!(status.output["result"].is_null());
    second.release().await.unwrap();
}
