use ocaf_runtime::jobs::{JobStatus, JobStore, LeasedJob};
use ocaf_runtime::rpc::{
    PeerPolicy, RemoteJobOutcome, RpcAction, RpcError, RpcGateway, RpcReply, RpcRequest, RpcStatus,
    SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::{json, Value};
use std::collections::HashMap;

const NOW: u64 = 1_800_000_000_000;
const SUBMIT_KEY: [u8; 32] = [0x21; 32];
const WORKER_KEY: [u8; 32] = [0x22; 32];
const BAD_KEY: [u8; 32] = [0x23; 32];

fn gateway(path: &std::path::Path) -> RpcGateway {
    let peers = HashMap::from([
        (
            "requester".into(),
            PeerPolicy::new(SUBMIT_KEY, vec!["echo".into()]).allow_job_submission(),
        ),
        (
            "worker".into(),
            PeerPolicy::new(WORKER_KEY, vec!["echo".into()]).allow_job_worker(),
        ),
        (
            "outsider".into(),
            PeerPolicy::new(BAD_KEY, vec!["image".into()]),
        ),
    ]);
    RpcGateway::new("coord", OcafRuntime::new(0), peers, 16)
        .unwrap()
        .with_job_db(path)
        .unwrap()
}

async fn call(
    gateway: &RpcGateway,
    sender: &str,
    key: &[u8; 32],
    action: RpcAction,
    now: u64,
) -> Result<RpcReply, RpcError> {
    let req = RpcRequest::new(sender, "coord", now, 5_000, action).unwrap();
    let bytes = SignedFrame::seal(&req, key).unwrap();
    let response = gateway.handle_at(&bytes, now).await?;
    SignedFrame::open(&response, key)
}

fn submit(id: &str) -> RpcAction {
    RpcAction::SubmitJob {
        id: id.into(),
        tool: "echo".into(),
        args: json!({"message":"over rpc"}),
    }
}

async fn claim(gateway: &RpcGateway, now: u64) -> LeasedJob {
    let reply = call(gateway, "worker", &WORKER_KEY, RpcAction::ClaimJob, now)
        .await
        .unwrap();
    assert_eq!(reply.status, RpcStatus::Ok);
    serde_json::from_value(reply.output).unwrap()
}

#[tokio::test]
async fn signed_submission_claim_completion_and_durable_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("queue.sqlite");
    let gateway = gateway(&path);
    let id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW)
            .await
            .unwrap()
            .status,
        RpcStatus::Ok
    );
    let lease = claim(&gateway, NOW + 10).await;
    assert_eq!(lease.id, id);
    assert_eq!(lease.worker_id, "worker");
    assert_eq!(lease.tool, "echo");
    let response = call(
        &gateway,
        "worker",
        &WORKER_KEY,
        RpcAction::FinishJob {
            lease,
            outcome: RemoteJobOutcome::Succeeded {
                output: json!({"echo":"done"}),
            },
        },
        NOW + 20,
    )
    .await
    .unwrap();
    assert_eq!(response.status, RpcStatus::Ok);
    let db = JobStore::open(&path).unwrap();
    let stored = db.get(&id).await.unwrap().unwrap();
    assert_eq!(stored.status, JobStatus::Succeeded);
    assert_eq!(stored.result, Some(json!({"echo":"done"})));
}

#[tokio::test]
async fn unauthorized_worker_or_submitter_cannot_access_queue() {
    let dir = tempfile::tempdir().unwrap();
    let gateway = gateway(&dir.path().join("queue.sqlite"));
    let id = uuid::Uuid::new_v4().to_string();
    assert!(matches!(
        call(&gateway, "outsider", &BAD_KEY, submit(&id), NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert!(matches!(
        call(&gateway, "requester", &SUBMIT_KEY, RpcAction::ClaimJob, NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert!(matches!(
        call(&gateway, "worker", &WORKER_KEY, submit(&id), NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert!(matches!(
        call(&gateway, "outsider", &BAD_KEY, RpcAction::ClaimJob, NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert!(JobStore::open(&dir.path().join("queue.sqlite"))
        .unwrap()
        .get(&id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn replayed_job_id_cannot_change_arguments_or_repeat_completed_work() {
    let dir = tempfile::tempdir().unwrap();
    let gateway = gateway(&dir.path().join("queue.sqlite"));
    let id = uuid::Uuid::new_v4().to_string();
    call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW)
        .await
        .unwrap();
    let duplicate = call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW + 1)
        .await
        .unwrap();
    assert_eq!(duplicate.status, RpcStatus::Ok);
    let changed = RpcAction::SubmitJob {
        id: id.clone(),
        tool: "echo".into(),
        args: json!({"message":"altered"}),
    };
    assert_eq!(
        call(&gateway, "requester", &SUBMIT_KEY, changed, NOW + 2)
            .await
            .unwrap()
            .status,
        RpcStatus::Failed
    );
    let lease = claim(&gateway, NOW + 3).await;
    call(
        &gateway,
        "worker",
        &WORKER_KEY,
        RpcAction::FinishJob {
            lease,
            outcome: RemoteJobOutcome::Succeeded { output: json!(1) },
        },
        NOW + 4,
    )
    .await
    .unwrap();
    assert_eq!(
        call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW + 5)
            .await
            .unwrap()
            .status,
        RpcStatus::Ok
    );
    assert_eq!(
        call(
            &gateway,
            "worker",
            &WORKER_KEY,
            RpcAction::ClaimJob,
            NOW + 6
        )
        .await
        .unwrap()
        .output,
        Value::Null
    );
}

#[tokio::test]
async fn cross_identity_completion_is_rejected_and_lease_is_fenced() {
    let dir = tempfile::tempdir().unwrap();
    let gateway = gateway(&dir.path().join("queue.sqlite"));
    let id = uuid::Uuid::new_v4().to_string();
    call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW)
        .await
        .unwrap();
    let lease = claim(&gateway, NOW + 1).await;
    let unauthorized = call(
        &gateway,
        "requester",
        &SUBMIT_KEY,
        RpcAction::FinishJob {
            lease: lease.clone(),
            outcome: RemoteJobOutcome::Succeeded {
                output: json!({"forged":true}),
            },
        },
        NOW + 2,
    )
    .await;
    assert!(matches!(unauthorized, Err(RpcError::Unauthorized)));
    let mut forged = lease.clone();
    forged.lease_token = uuid::Uuid::new_v4().to_string();
    let denied = call(
        &gateway,
        "worker",
        &WORKER_KEY,
        RpcAction::FinishJob {
            lease: forged,
            outcome: RemoteJobOutcome::Succeeded {
                output: json!({"forged":true}),
            },
        },
        NOW + 3,
    )
    .await
    .unwrap();
    assert_eq!(denied.status, RpcStatus::Failed);
    let valid = call(
        &gateway,
        "worker",
        &WORKER_KEY,
        RpcAction::FinishJob {
            lease,
            outcome: RemoteJobOutcome::Uncertain,
        },
        NOW + 4,
    )
    .await
    .unwrap();
    assert_eq!(valid.status, RpcStatus::Ok);
    assert_eq!(
        JobStore::open(&dir.path().join("queue.sqlite"))
            .unwrap()
            .get(&id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Uncertain
    );
}

#[tokio::test]
async fn worker_only_claims_tools_covered_by_its_peer_acl() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("queue.sqlite");
    let gateway = gateway(&path);
    // A different trusted service submitted a job not executable by this worker.
    let store = JobStore::open(&path).unwrap();
    store
        .submit(ocaf_runtime::jobs::JobSpec {
            id: uuid::Uuid::new_v4().to_string(),
            tool: "image".into(),
            args: json!({}),
            retry_safe: false,
            max_attempts: 1,
        })
        .await
        .unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW)
        .await
        .unwrap();
    let lease = claim(&gateway, NOW + 1).await;
    assert_eq!(lease.id, id);
    assert_eq!(
        store.get(&id).await.unwrap().unwrap().status,
        JobStatus::Running
    );
}

#[tokio::test]
async fn crash_expiration_marks_unsafe_remote_work_uncertain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("queue.sqlite");
    let gateway = gateway(&path);
    let id = uuid::Uuid::new_v4().to_string();
    call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW)
        .await
        .unwrap();
    let lease = claim(&gateway, NOW + 1).await;
    drop(gateway);
    let recovered = self::gateway(&path);
    let next = call(
        &recovered,
        "worker",
        &WORKER_KEY,
        RpcAction::ClaimJob,
        NOW + 61_000,
    )
    .await
    .unwrap();
    assert_eq!(next.output, Value::Null);
    let stored = JobStore::open(&path)
        .unwrap()
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, JobStatus::Uncertain);
    // No silent replay after ambiguous remote execution.
    assert_eq!(stored.attempts, 1);
    assert!(!lease.lease_token.is_empty());
}

#[tokio::test]
async fn only_original_submitter_can_read_remote_job_status_and_result() {
    let dir = tempfile::tempdir().unwrap();
    let peers = HashMap::from([
        (
            "requester".into(),
            PeerPolicy::new(SUBMIT_KEY, vec!["echo".into()]).allow_job_submission(),
        ),
        (
            "requester-b".into(),
            PeerPolicy::new(BAD_KEY, vec!["echo".into()]).allow_job_submission(),
        ),
        (
            "worker".into(),
            PeerPolicy::new(WORKER_KEY, vec!["echo".into()]).allow_job_worker(),
        ),
    ]);
    let gateway = RpcGateway::new("coord", OcafRuntime::new(0), peers, 16)
        .unwrap()
        .with_job_db(&dir.path().join("jobs.sqlite"))
        .unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    call(&gateway, "requester", &SUBMIT_KEY, submit(&id), NOW)
        .await
        .unwrap();
    let other = call(
        &gateway,
        "requester-b",
        &BAD_KEY,
        RpcAction::GetJob { id: id.clone() },
        NOW + 1,
    )
    .await
    .unwrap();
    assert_eq!(other.status, RpcStatus::Ok);
    assert_eq!(other.output, Value::Null);

    // Knowing the job UUID and the original payload does not grant ownership.
    let collision = call(&gateway, "requester-b", &BAD_KEY, submit(&id), NOW + 2)
        .await
        .unwrap();
    assert_eq!(collision.status, RpcStatus::Failed);

    let lease = claim(&gateway, NOW + 3).await;
    call(
        &gateway,
        "worker",
        &WORKER_KEY,
        RpcAction::FinishJob {
            lease,
            outcome: RemoteJobOutcome::Succeeded {
                output: json!({"result":"secret"}),
            },
        },
        NOW + 4,
    )
    .await
    .unwrap();

    let mine = call(
        &gateway,
        "requester",
        &SUBMIT_KEY,
        RpcAction::GetJob { id: id.clone() },
        NOW + 5,
    )
    .await
    .unwrap();
    assert_eq!(mine.status, RpcStatus::Ok);
    assert_eq!(mine.output["status"], "succeeded");
    assert_eq!(mine.output["result"]["result"], "secret");

    let outsider = call(
        &gateway,
        "outsider",
        &BAD_KEY,
        RpcAction::GetJob { id },
        NOW + 6,
    )
    .await;
    assert!(matches!(outsider, Err(RpcError::Unauthorized)));
}
