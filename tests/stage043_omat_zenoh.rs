#![cfg(feature = "zenoh")]
use ocaf_runtime::omat_server::{OmatPolicy, OmatRegistry, ToolCall};
use ocaf_runtime::stage043_omat_zenoh::{
    request_omat_with_retry, request_omat_zenoh, serve_omat_zenoh, sign_omas_request,
    OmatZenohAdmission,
};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

fn call(tenant: &str) -> ToolCall {
    ToolCall {
        request_id: "req_1".into(),
        tenant_id: tenant.into(),
        agent_id: "agent_1".into(),
        tool_id: "tool_1".into(),
        arguments: serde_json::json!({}),
        idempotency_key: "idem_1".into(),
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}

fn admission() -> OmatZenohAdmission {
    let keys = HashMap::from([("omas_a".to_owned(), [7u8; 32])]);
    let tenants = HashMap::from([("omas_a".to_owned(), "tenant_a".to_owned())]);
    let policies = HashMap::from([(
        "tenant_a".to_owned(),
        OmatPolicy {
            tenant_id: "tenant_a".into(),
            allowed_tools: BTreeSet::new(),
            allowed_capabilities: BTreeSet::new(),
            max_timeout_ms: 1000,
            max_output_bytes: 1024,
            allow_external_side_effects: false,
        },
    )]);
    OmatZenohAdmission::new(keys, tenants, policies, OmatRegistry::default()).unwrap()
}

#[test]
fn signed_admission_rejects_forgery_cross_tenant_and_replay() {
    let mut server = admission();
    server.connected(true);
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    let signed = sign_omas_request(
        "omas_a",
        topic,
        "0123456789abcdef",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    // Validly authenticated requests reach the fail-closed tool policy.
    assert_eq!(
        server.handle(&signed).unwrap_err().to_string(),
        "OMAt request denied"
    );
    assert!(server
        .handle(&signed)
        .unwrap_err()
        .to_string()
        .contains("replay"));
    let forged = sign_omas_request(
        "omas_a",
        topic,
        "0123456789abcdee",
        call("tenant_a"),
        &[8u8; 32],
    )
    .unwrap();
    assert!(server
        .handle(&forged)
        .unwrap_err()
        .to_string()
        .contains("signature"));
    let foreign = sign_omas_request(
        "omas_a",
        topic,
        "0123456789abcded",
        call("tenant_b"),
        &[7u8; 32],
    )
    .unwrap();
    assert_eq!(
        server.handle(&foreign).unwrap_err().to_string(),
        "OMAt request denied"
    );
    server.connected(false);
    let disconnected = sign_omas_request(
        "omas_a",
        topic,
        "0123456789abcdec",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    assert_eq!(
        server.handle(&disconnected).unwrap_err().to_string(),
        "OMAt request denied"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_zenoh_loopback_rejects_and_fails_closed_after_server_shutdown() {
    let mut config = zenoh::Config::default();
    config.insert_json5("mode", "\"peer\"").unwrap();
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    config
        .insert_json5("scouting/gossip/enabled", "false")
        .unwrap();
    let session = Arc::new(zenoh::open(config).await.unwrap());
    let key = "ocaf/omat/tenant_a/requests/req_1";
    let server_session = Arc::clone(&session);
    let task =
        tokio::spawn(async move { serve_omat_zenoh(server_session, admission(), key).await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    let signed = sign_omas_request(
        "omas_a",
        key,
        "0123456789abcdef",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    let response = request_omat_zenoh(&session, key, signed)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        response.contains("rejected"),
        "unexpected Zenoh result: {response}"
    );
    task.abort();
    let _ = task.await;
    let signed = sign_omas_request(
        "omas_a",
        key,
        "0123456789abcdee",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    let response = request_omat_zenoh(&session, key, signed)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        response.contains("unavailable") || response.contains("timeout"),
        "unexpected disconnect result: {response}"
    );
}

#[test]
fn replay_survives_server_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replay.sqlite");
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    let signed = sign_omas_request(
        "omas_a",
        topic,
        "0123456789abcdef",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    {
        let mut first = admission().with_replay_database(&path).unwrap();
        first.connected(true);
        assert_eq!(
            first.handle(&signed).unwrap_err().to_string(),
            "OMAt request denied"
        );
    }
    let mut restarted = admission().with_replay_database(&path).unwrap();
    restarted.connected(true);
    assert!(restarted
        .handle(&signed)
        .unwrap_err()
        .to_string()
        .contains("replay"));
}

#[test]
fn security_metrics_track_denials_and_replays() {
    use std::sync::atomic::Ordering;
    let mut server = admission();
    let metrics = server.metrics();
    server.connected(true);
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    let signed = sign_omas_request(
        "omas_a",
        topic,
        "0123456789abcdef",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    assert!(server.handle(&signed).is_err());
    assert!(server.handle(&signed).is_err());
    assert_eq!(metrics.requests.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.rejections.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.replays.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.connection_events.load(Ordering::Relaxed), 1);
    assert!(metrics.prometheus().contains("ocaf_omat_replays_total 1"));
}

#[test]
fn security_metrics_count_rejections_and_replays() {
    use std::sync::atomic::Ordering;
    let mut server = admission();
    let metrics = server.metrics();
    server.connected(true);
    let topic = "ocaf/omat/tenant_a/requests/req_1";
    let signed = sign_omas_request(
        "omas_a",
        topic,
        "0123456789abcdef",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    assert!(server.handle(&signed).is_err());
    assert!(server.handle(&signed).is_err());
    assert_eq!(metrics.requests.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.rejections.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.replays.load(Ordering::Relaxed), 1);
    assert!(metrics
        .prometheus()
        .contains("ocaf_omat_requests_total 2\\n"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retry_rejects_invalid_budget_without_network_io() {
    let config = zenoh::Config::default();
    let session = zenoh::open(config).await.unwrap();
    let error = request_omat_with_retry(&session, "ocaf/omat/test", vec![], 0)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("1..=5"));
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_rotation_shutdown_releases_old_queryable() {
    use ocaf_runtime::stage043_omat_zenoh::serve_omat_zenoh_until;
    let mut config = zenoh::Config::default();
    config.insert_json5("mode", "\"peer\"").unwrap();
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    config
        .insert_json5("scouting/gossip/enabled", "false")
        .unwrap();
    let session = Arc::new(zenoh::open(config).await.unwrap());
    let key = "ocaf/omat/tenant_a/requests/req_1";
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let serving_session = Arc::clone(&session);
    let task = tokio::spawn(async move {
        serve_omat_zenoh_until(serving_session, admission(), key, async {
            let _ = shutdown_rx.await;
        })
        .await
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    let signed = sign_omas_request(
        "omas_a",
        key,
        "0123456789abcdef",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    assert!(request_omat_zenoh(&session, key, signed)
        .await
        .unwrap_err()
        .to_string()
        .contains("rejected"));
    shutdown_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .expect("graceful shutdown deadline")
        .unwrap()
        .unwrap();
    let signed = sign_omas_request(
        "omas_a",
        key,
        "0123456789abcdee",
        call("tenant_a"),
        &[7u8; 32],
    )
    .unwrap();
    let error = request_omat_zenoh(&session, key, signed)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("timeout") || error.contains("unavailable"));
}
