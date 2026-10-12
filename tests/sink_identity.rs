//! OCAF 0.22: signed sink authorization must bind to real lease identity
//! and to the exact cluster namespace (not merely an unscoped job UUID).
use ocaf_runtime::jobs::LeasedJob;
use ocaf_runtime::rpc::SignedFrame;
use ocaf_runtime::sink::{EffectSink, SinkAction, SinkKeys, SinkPermit, SinkRequest, SinkResponse, SinkStatus};
use serde_json::json;

const NOW: u64 = 1_800_000_000_000;
const CONTROLLER: [u8;32] = [0x71;32];
const WORKER: [u8;32] = [0x72;32];

fn lease() -> LeasedJob {
    LeasedJob {
        id: uuid::Uuid::new_v4().to_string(),
        tool: "effectful_tool".into(),
        args: json!({"resource":"vault"}),
        attempt: 1,
        worker_id: "workeraa".into(),
        lease_token: uuid::Uuid::new_v4().to_string(),
        lease_expires_at_ms: NOW + 14_000,
        leader_epoch: 27,
    }
}

#[test]
fn sink_permit_is_cryptographically_bound_to_tool_attempt_token_and_cluster() {
    let l = lease();
    let body = json!({"write":"demo"});
    let p = SinkPermit::from_lease(
        "demo-sink", "cluster-a", &l, NOW + 10_000, &body,
    ).unwrap();
    assert_eq!(p.cluster_id, "cluster-a");
    assert_eq!(p.tool, l.tool);
    assert_eq!(p.attempt, 1);
    assert_eq!(p.epoch, l.leader_epoch);
    assert_eq!(p.worker, l.worker_id);
    assert!(!p.lease_fingerprint.contains(&l.lease_token));
    assert!(SinkPermit::from_lease(
        "demo-sink", "cluster-a", &l, l.lease_expires_at_ms + 1, &body
    ).is_err());
    let mut bad = l.clone();
    bad.attempt = 0;
    assert!(SinkPermit::from_lease(
        "demo-sink", "cluster-a", &bad, NOW+10_000, &body
    ).is_err());
    bad = l.clone();
    bad.leader_epoch = 0;
    assert!(SinkPermit::from_lease(
        "demo-sink", "cluster-a", &bad, NOW+10_000, &body
    ).is_err());
}

#[tokio::test]
async fn isolated_receiver_rejects_correctly_signed_cross_cluster_and_unbound_tokens() {
    let tmp = tempfile::tempdir().unwrap();
    let keys = SinkKeys::new("controller", CONTROLLER, "workeraa", WORKER)
        .unwrap().with_cluster_id("cluster-a").unwrap();
    let sink = EffectSink::open(&tmp.path().join("journal.sqlite"), "demo-sink", keys).unwrap();
    let activate = SinkRequest::new(
        "demo-sink", "controller", NOW, 5000,
        SinkAction::ActivateEpoch {
            cluster_id: "cluster-a".into(),
            epoch: 27,
            expires_at_ms: NOW + 15_000,
        }
    ).unwrap();
    let signed = activate.seal(&CONTROLLER).unwrap();
    let out = sink.handle_at(&signed, NOW).await.unwrap();
    assert_eq!(SinkResponse::open(&out, &CONTROLLER, &activate).unwrap().status, SinkStatus::Ok);
    let l = lease();
    let effect = json!({"write":true});
    let wrong = SinkPermit::from_lease("demo-sink","cluster-b",&l,NOW+10_000,&effect).unwrap();
    let request = SinkRequest::new(
        "demo-sink", "workeraa", NOW, 5000,
        SinkAction::Apply {permit:SignedFrame::seal(&wrong, &CONTROLLER).unwrap(), effect:effect.clone()}
    ).unwrap();
    let reply = sink.handle_at(&request.seal(&WORKER).unwrap(), NOW).await.unwrap();
    assert_eq!(SinkResponse::open(&reply,&WORKER,&request).unwrap().status,SinkStatus::Denied);

    let permit = SinkPermit::from_lease("demo-sink","cluster-a",&l,NOW+10_000,&effect).unwrap();
    let signed_permit = SignedFrame::seal(&permit, &CONTROLLER).unwrap();
    let valid = SinkRequest::new(
        "demo-sink","workeraa",NOW,5000,
        SinkAction::Apply {permit:signed_permit.clone(),effect:effect.clone()}
    ).unwrap();
    let result = sink.handle_at(&valid.seal(&WORKER).unwrap(),NOW).await.unwrap();
    assert_eq!(SinkResponse::open(&result,&WORKER,&valid).unwrap().status,SinkStatus::Ok);

    // Tampering with signed lease fields breaks the controller HMAC.
    let mut tampered = permit;
    tampered.attempt = 2;
    let mutated = serde_json::to_vec(&tampered).unwrap();
    assert_ne!(mutated, serde_json::to_vec(&SignedFrame::open::<SinkPermit>(&signed_permit,&CONTROLLER).unwrap()).unwrap());
}
