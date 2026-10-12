//! Signed coordinator RPC with shared PostgreSQL job journal. CI runs a
//! real PostgreSQL service; each gateway owns a separate database session.
use ocaf_runtime::rpc::{
    PeerPolicy, RemoteJobOutcome, RpcAction, RpcGateway, RpcReply, RpcRequest, RpcStatus,
    SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::json;
use std::collections::HashMap;

const TIME: u64 = 1_800_000_000_000;
const REQUESTER_KEY: [u8; 32] = [0xA5; 32];
const WORKER_KEY: [u8; 32] = [0xB6; 32];

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(
        value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "CI must supply live PostgreSQL"
    );
    value
}

async fn gateway(url: &str) -> RpcGateway {
    let peers = HashMap::from([
        (
            "requester".into(),
            PeerPolicy::new(REQUESTER_KEY, vec!["echo".into()]).allow_job_submission(),
        ),
        (
            "worker".into(),
            PeerPolicy::new(WORKER_KEY, vec!["echo".into()]).allow_job_worker(),
        ),
    ]);
    RpcGateway::new("coord", OcafRuntime::new(0), peers, 32)
        .unwrap()
        .with_postgres_job_db(url, true)
        .await
        .unwrap()
}

async fn request(
    gateway: &RpcGateway,
    sender: &str,
    key: &[u8; 32],
    action: RpcAction,
) -> RpcReply {
    let msg = RpcRequest::new(sender, "coord", TIME, 5_000, action).unwrap();
    let signed = SignedFrame::seal(&msg, key).unwrap();
    let response = gateway.handle_at(&signed, TIME).await.unwrap();
    SignedFrame::open(&response, key).unwrap()
}

#[tokio::test]
async fn independent_gateways_round_trip_one_persistent_remote_job() {
    let Some(url) = dsn() else { return };
    let first = gateway(&url).await;
    let second = gateway(&url).await;
    let id = uuid::Uuid::new_v4().to_string();
    let submitted = request(
        &first,
        "requester",
        &REQUESTER_KEY,
        RpcAction::SubmitJob {
            id: id.clone(),
            tool: "echo".into(),
            args: json!({"message":"cluster"}),
        },
    )
    .await;
    assert_eq!(submitted.status, RpcStatus::Ok);
    let lease = request(&second, "worker", &WORKER_KEY, RpcAction::ClaimJob).await;
    assert_eq!(lease.status, RpcStatus::Ok);
    let lease: ocaf_runtime::jobs::LeasedJob = serde_json::from_value(lease.output).unwrap();
    assert_eq!(lease.id, id);
    let done = request(
        &second,
        "worker",
        &WORKER_KEY,
        RpcAction::FinishJob {
            lease,
            outcome: RemoteJobOutcome::Succeeded {
                output: json!({"ok":true}),
            },
        },
    )
    .await;
    assert_eq!(done.status, RpcStatus::Ok);
    let status = request(
        &first,
        "requester",
        &REQUESTER_KEY,
        RpcAction::GetJob { id },
    )
    .await;
    assert_eq!(status.status, RpcStatus::Ok);
    assert_eq!(status.output["status"], "succeeded");
    assert_eq!(status.output["result"], json!({"ok":true}));
}
