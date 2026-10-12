//! Independent SQLite-backed effect receiver: policy tests run without PostgreSQL.
use std::time::Duration;

use ocaf_runtime::rpc::SignedFrame;
use ocaf_runtime::sink::{
    EffectSink, SinkAction, SinkKeys, SinkPermit, SinkRequest, SinkResponse, SinkStatus,
};
use serde_json::json;

const NOW: u64 = 1_800_000_000_000;
const CONTROLLER: [u8; 32] = [0x31; 32];
const WORKER: [u8; 32] = [0x32; 32];
const CLUSTER: &str = "unit-cluster";

fn fake_lease(job_id: &str, epoch: i64) -> ocaf_runtime::jobs::LeasedJob {
    ocaf_runtime::jobs::LeasedJob {
        id: job_id.to_owned(),
        tool: "unit_echo".into(),
        args: json!({}),
        attempt: 1,
        worker_id: "worker".into(),
        lease_token: uuid::Uuid::new_v4().to_string(),
        lease_expires_at_ms: NOW + 30_000,
        leader_epoch: epoch,
    }
}

fn sink(path: &std::path::Path) -> EffectSink {
    EffectSink::open(
        path,
        "remote-sink",
        SinkKeys::new("controller", CONTROLLER, "worker", WORKER)
            .unwrap()
            .with_cluster_id(CLUSTER)
            .unwrap(),
    )
    .unwrap()
}
fn permit(job: &str, epoch: i64, effect: &serde_json::Value) -> Vec<u8> {
    let token = SinkPermit::from_lease(
        "remote-sink",
        CLUSTER,
        &fake_lease(job, epoch),
        NOW + 30_000,
        effect,
    )
    .unwrap();
    SignedFrame::seal(&token, &CONTROLLER).unwrap()
}
async fn request(
    sink: &EffectSink,
    sender: &str,
    key: &[u8; 32],
    action: SinkAction,
) -> SinkResponse {
    let req = SinkRequest::new("remote-sink", sender, NOW, 5000, action).unwrap();
    let frame = SinkRequest::seal(&req, key).unwrap();
    let response = sink.handle_at(&frame, NOW).await.unwrap();
    SinkResponse::open(&response, key, &req).unwrap()
}
async fn activate(sink: &EffectSink, epoch: i64) -> SinkResponse {
    request(
        sink,
        "controller",
        &CONTROLLER,
        SinkAction::ActivateEpoch {
            cluster_id: CLUSTER.into(),
            epoch,
            expires_at_ms: NOW + 15_000,
        },
    )
    .await
}

#[tokio::test]
async fn durable_sink_fences_old_generations_and_rejects_conflicting_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let a = sink(&tmp.path().join("effects.db"));
    assert_eq!(activate(&a, 12).await.status, SinkStatus::Ok);
    let id = uuid::Uuid::new_v4().to_string();
    let effect = json!({"changed": true, "value": 7});
    let token = permit(&id, 12, &effect);
    let done = request(
        &a,
        "worker",
        &WORKER,
        SinkAction::Apply {
            permit: token.clone(),
            effect: effect.clone(),
        },
    )
    .await;
    assert_eq!(done.status, SinkStatus::Ok);
    assert_eq!(done.output["applied"], true);
    let original_timestamp = done.output["committed_at_ms"].clone();

    let b = sink(&tmp.path().join("effects.db"));
    let repeat = request(
        &b,
        "worker",
        &WORKER,
        SinkAction::Apply {
            permit: token.clone(),
            effect: effect.clone(),
        },
    )
    .await;
    assert_eq!(repeat.status, SinkStatus::Ok);
    assert_eq!(repeat.output["applied"], false);
    assert_eq!(repeat.output["committed_at_ms"], original_timestamp);

    assert_eq!(activate(&b, 13).await.status, SinkStatus::Ok);
    let stale = request(
        &a,
        "worker",
        &WORKER,
        SinkAction::Apply {
            permit: token.clone(),
            effect: effect.clone(),
        },
    )
    .await;
    assert_eq!(stale.status, SinkStatus::Denied);
    assert_eq!(activate(&a, 12).await.status, SinkStatus::Denied);

    let changed = request(
        &b,
        "worker",
        &WORKER,
        SinkAction::Apply {
            permit: permit(&id, 13, &json!({"changed": false})),
            effect: json!({"changed": false}),
        },
    )
    .await;
    assert_eq!(changed.status, SinkStatus::Conflict);
}

#[tokio::test]
async fn invalid_worker_controller_and_tampered_permit_fail_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let store = sink(&tmp.path().join("sink.db"));
    let forbidden = request(
        &store,
        "worker",
        &WORKER,
        SinkAction::ActivateEpoch {
            cluster_id: CLUSTER.into(),
            epoch: 1,
            expires_at_ms: NOW + 5_000,
        },
    )
    .await;
    assert_eq!(forbidden.status, SinkStatus::Denied);
    let id = uuid::Uuid::new_v4().to_string();
    let data = json!({"original": true});
    let token = permit(&id, 1, &data);
    let not_activated = request(
        &store,
        "worker",
        &WORKER,
        SinkAction::Apply {
            permit: token.clone(),
            effect: data.clone(),
        },
    )
    .await;
    assert_eq!(not_activated.status, SinkStatus::Denied);
    assert_eq!(activate(&store, 1).await.status, SinkStatus::Ok);
    let swapped = request(
        &store,
        "worker",
        &WORKER,
        SinkAction::Apply {
            permit: token.clone(),
            effect: json!({"different": true}),
        },
    )
    .await;
    assert_eq!(swapped.status, SinkStatus::Denied);
    let valid = request(
        &store,
        "worker",
        &WORKER,
        SinkAction::Apply {
            permit: token,
            effect: data,
        },
    )
    .await;
    assert_eq!(valid.status, SinkStatus::Ok);
    let request = SinkRequest::new(
        "remote-sink",
        "controller",
        NOW,
        5000,
        SinkAction::ActivateEpoch {
            cluster_id: CLUSTER.into(),
            epoch: 2,
            expires_at_ms: NOW + 5000,
        },
    )
    .unwrap();
    let frame = SinkRequest::seal(&request, &CONTROLLER).unwrap();
    assert!(store.handle_at(&frame, NOW).await.is_ok());
    assert!(store.handle_at(&frame, NOW).await.is_err());
    let timeout =
        tokio::time::timeout(Duration::from_secs(1), request_fn_expired(&store, &request))
            .await
            .unwrap();
    assert!(timeout);
}
async fn request_fn_expired(store: &EffectSink, req: &SinkRequest) -> bool {
    let frame = SinkRequest::seal(req, &CONTROLLER).unwrap();
    store.handle_at(&frame, NOW + 31_000).await.is_err()
}

#[tokio::test]
async fn epoch_activation_expires_even_when_signed_worker_permit_is_still_valid() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("time_fence.sqlite");
    let store = sink(&file);
    let activation = SinkRequest::new(
        "remote-sink",
        "controller",
        NOW,
        5000,
        SinkAction::ActivateEpoch {
            cluster_id: CLUSTER.into(),
            epoch: 23,
            expires_at_ms: NOW + 1_000,
        },
    )
    .unwrap();
    let signed_activation = activation.seal(&CONTROLLER).unwrap();
    assert!(store.handle_at(&signed_activation, NOW).await.is_ok());
    drop(store);

    let reopened = sink(&file);
    assert!(matches!(
        reopened.handle_at(&signed_activation, NOW).await,
        Err(ocaf_runtime::sink::SinkError::Replay)
    ));
    let body = json!({"still_signed":"not_authorized"});
    let req = SinkRequest::new(
        "remote-sink",
        "worker",
        NOW + 1_100,
        5000,
        SinkAction::Apply {
            permit: permit(&uuid::Uuid::new_v4().to_string(), 23, &body),
            effect: body,
        },
    )
    .unwrap();
    let response = reopened
        .handle_at(&req.seal(&WORKER).unwrap(), NOW + 1_100)
        .await
        .unwrap();
    let signed = SinkResponse::open(&response, &WORKER, &req).unwrap();
    assert_eq!(signed.status, SinkStatus::Denied);
}

#[tokio::test]
async fn a_sink_database_cannot_be_relabelled_to_another_sink_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("identity.sqlite");
    let original = sink(&path);
    drop(original);
    assert!(matches!(
        EffectSink::open(
            &path,
            "different-sink",
            SinkKeys::new("controller", CONTROLLER, "worker", WORKER)
                .unwrap()
                .with_cluster_id(CLUSTER)
                .unwrap()
        ),
        Err(ocaf_runtime::sink::SinkError::Invalid)
    ));
}
