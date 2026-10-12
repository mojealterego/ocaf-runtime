//! Independent SQLite-backed effect receiver: policy tests run without PostgreSQL.
use std::time::Duration;

use ocaf_runtime::sink::{
    EffectSink, SinkAction, SinkKeys, SinkPermit, SinkRequest, SinkResponse, SinkStatus,
};
use ocaf_runtime::rpc::SignedFrame;
use serde_json::json;

const NOW: u64 = 1_800_000_000_000;
const CONTROLLER: [u8; 32] = [0x31; 32];
const WORKER: [u8; 32] = [0x32; 32];

fn sink(path: &std::path::Path) -> EffectSink {
    EffectSink::open(
        path,
        "remote-sink",
        SinkKeys::new("controller", CONTROLLER, "worker", WORKER).unwrap(),
    ).unwrap()
}
fn permit(job: &str, epoch: i64, effect: &serde_json::Value) -> Vec<u8> {
    let token = SinkPermit::new(
        "remote-sink", "worker", job, epoch,
        NOW + 30_000, effect,
    ).unwrap();
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
            epoch,
            expires_at_ms: NOW + 15_000,
        },
    ).await
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
        &a, "worker", &WORKER,
        SinkAction::Apply {permit: token.clone(), effect: effect.clone()},
    ).await;
    assert_eq!(done.status, SinkStatus::Ok);
    assert_eq!(done.output["applied"], true);
    let original_timestamp = done.output["committed_at_ms"].clone();

    let b = sink(&tmp.path().join("effects.db"));
    let repeat = request(
        &b, "worker", &WORKER,
        SinkAction::Apply {permit: token.clone(), effect: effect.clone()},
    ).await;
    assert_eq!(repeat.status, SinkStatus::Ok);
    assert_eq!(repeat.output["applied"], false);
    assert_eq!(repeat.output["committed_at_ms"], original_timestamp);

    assert_eq!(activate(&b, 13).await.status, SinkStatus::Ok);
    let stale = request(
        &a, "worker", &WORKER,
        SinkAction::Apply {permit: token.clone(), effect: effect.clone()},
    ).await;
    assert_eq!(stale.status, SinkStatus::Denied);
    assert_eq!(activate(&a, 12).await.status, SinkStatus::Denied);

    let changed = request(
        &b, "worker", &WORKER,
        SinkAction::Apply {
            permit: permit(&id, 13, &json!({"changed": false})),
            effect: json!({"changed": false}),
        },
    ).await;
    assert_eq!(changed.status, SinkStatus::Conflict);
}

#[tokio::test]
async fn invalid_worker_controller_and_tampered_permit_fail_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let store = sink(&tmp.path().join("sink.db"));
    let forbidden = request(
        &store, "worker", &WORKER,
        SinkAction::ActivateEpoch {epoch: 1, expires_at_ms: NOW + 5_000},
    ).await;
    assert_eq!(forbidden.status, SinkStatus::Denied);
    let id = uuid::Uuid::new_v4().to_string();
    let data = json!({"original": true});
    let token = permit(&id, 1, &data);
    let not_activated = request(
        &store, "worker", &WORKER,
        SinkAction::Apply {permit: token.clone(), effect: data.clone()},
    ).await;
    assert_eq!(not_activated.status, SinkStatus::Denied);
    assert_eq!(activate(&store, 1).await.status, SinkStatus::Ok);
    let swapped = request(
        &store, "worker", &WORKER,
        SinkAction::Apply {permit: token.clone(), effect: json!({"different": true})},
    ).await;
    assert_eq!(swapped.status, SinkStatus::Denied);
    let valid = request(
        &store, "worker", &WORKER,
        SinkAction::Apply {permit: token, effect: data},
    ).await;
    assert_eq!(valid.status, SinkStatus::Ok);
    let request = SinkRequest::new(
        "remote-sink", "controller", NOW, 5000,
        SinkAction::ActivateEpoch {epoch: 2, expires_at_ms: NOW + 5000},
    ).unwrap();
    let frame = SinkRequest::seal(&request, &CONTROLLER).unwrap();
    assert!(store.handle_at(&frame, NOW).await.is_ok());
    assert!(store.handle_at(&frame, NOW).await.is_err());
    let timeout = tokio::time::timeout(
        Duration::from_secs(1),
        request_fn_expired(&store, &request),
    ).await.unwrap();
    assert!(timeout);
}
async fn request_fn_expired(store: &EffectSink, req: &SinkRequest) -> bool {
    let frame = SinkRequest::seal(req, &CONTROLLER).unwrap();
    store.handle_at(&frame, NOW + 31_000).await.is_err()
}
