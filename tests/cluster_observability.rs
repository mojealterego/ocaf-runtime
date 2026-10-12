//! Real PostgreSQL operational snapshot and fail-closed leader detection.
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::cluster::observability::{
    AlertPolicy, ClusterAlertCode, ClusterHealth, ClusterSnapshot, PgClusterMonitor,
};
use ocaf_runtime::jobs::JobSpec;
use serde_json::json;

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(
        value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "PostgreSQL integration tests must not silently skip in CI"
    );
    value
}

#[test]
fn alert_thresholds_and_leadership_are_deterministic_and_bounded() {
    let snapshot = ClusterSnapshot {
        observed_at_ms: 1_800_000_000_000,
        leader_sessions: 0,
        agents_live: 0,
        agents_expired: 1,
        jobs_pending: 100,
        jobs_running: 3,
        jobs_succeeded: 10,
        jobs_failed: 2,
        jobs_uncertain: 1,
        jobs_running_expired: 1,
    };
    let policy = AlertPolicy {
        pending_threshold: 100,
        uncertain_threshold: 1,
        expired_agents_threshold: 1,
    };
    let report = policy.evaluate(snapshot.clone());
    assert_eq!(report.health, ClusterHealth::Critical);
    assert!(report
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::NoLeader));
    assert!(report
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::PendingBacklog));
    assert!(report
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::UncertainJobs));
    assert!(report
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::ExpiredRunningJobs));
    assert!(report
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::NoLiveAgents));
    assert!(report
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::ExpiredAgents));
    assert!(!serde_json::to_string(&report).unwrap().contains("password"));

    let healthy = ClusterSnapshot {
        leader_sessions: 1,
        agents_live: 2,
        agents_expired: 0,
        jobs_pending: 0,
        jobs_running: 0,
        jobs_succeeded: 3,
        jobs_failed: 0,
        jobs_uncertain: 0,
        jobs_running_expired: 0,
        ..snapshot
    };
    let report = policy.evaluate(healthy);
    assert_eq!(report.health, ClusterHealth::Healthy);
    assert!(report.alerts.is_empty());
    assert!(AlertPolicy::new(0, 1, 1).is_err());
    assert!(AlertPolicy::new(1, 0, 1).is_err());
}

#[tokio::test]
async fn monitor_reads_real_db_clock_counts_and_postgres_advisory_leader() {
    let Some(dsn) = dsn() else { return };
    let cluster = format!("observe_{}", uuid::Uuid::new_v4().simple());
    let monitor = PgClusterMonitor::connect(&dsn, &cluster, true)
        .await
        .unwrap();
    // Live metrics must not invent a leader before advisory lock acquisition.
    let empty = monitor.snapshot().await.unwrap();
    assert_eq!(empty.leader_sessions, 0);
    assert!(empty.observed_at_ms > 1_700_000_000_000);

    let leader = PgLeader::connect(&dsn, &cluster, true).await.unwrap();
    assert!(leader.try_acquire().await.unwrap());
    let current = monitor.snapshot().await.unwrap();
    assert_eq!(current.leader_sessions, 1);

    let queue = PgJobQueue::connect(&dsn, true).await.unwrap();
    let job = JobSpec {
        id: uuid::Uuid::new_v4().to_string(),
        tool: format!("health_{}", uuid::Uuid::new_v4().simple()),
        args: json!({"private_key":"DO_NOT_EMIT_SECRETS"}),
        retry_safe: false,
        max_attempts: 1,
    };
    queue.submit_owned("health-test", job).await.unwrap();
    let after = monitor.snapshot().await.unwrap();
    assert!(after.jobs_pending >= 1);
    assert!(
        after.jobs_pending
            + after.jobs_running
            + after.jobs_succeeded
            + after.jobs_failed
            + after.jobs_uncertain
            >= 1
    );
    let encoded = serde_json::to_string(&after).unwrap();
    assert!(!encoded.contains("DO_NOT_EMIT_SECRETS"));
    assert!(!encoded.contains("health-test"));

    leader.release().await.unwrap();
    assert_eq!(monitor.snapshot().await.unwrap().leader_sessions, 0);
}

#[tokio::test]
async fn monitor_rejects_plaintext_remote_postgres_and_bad_cluster_names() {
    let err = PgClusterMonitor::connect(
        "host=public.example.com user=guest sslmode=disable",
        "monitor",
        true,
    )
    .await
    .err()
    .expect("remote insecure database denied");
    assert!(err.to_string().contains("transport"));
    let err = PgClusterMonitor::connect(
        "host=127.0.0.1 user=guest sslmode=disable",
        "not/a/cluster",
        true,
    )
    .await
    .err()
    .expect("invalid cluster denied before network call");
    assert!(err.to_string().contains("cluster"));
}

#[tokio::test]
async fn per_node_health_uses_database_clock_and_no_capability_or_secret_payload() {
    let Some(dsn) = dsn() else { return };
    let cluster = format!("observe_node_{}", uuid::Uuid::new_v4().simple());
    let monitor = PgClusterMonitor::connect(&dsn, &cluster, true)
        .await
        .unwrap();
    let directory = ocaf_runtime::cluster::directory::PgDirectory::connect(&dsn, true)
        .await
        .unwrap();
    let node = format!("node_{}", uuid::Uuid::new_v4().simple());
    let now_ms = ocaf_runtime::rpc::now_ms().unwrap();
    directory
        .heartbeat(&node, &["private_tool".to_string()], now_ms)
        .await
        .unwrap();
    let observed = monitor.node_health(&node).await.unwrap().unwrap();
    assert_eq!(observed.node_id, node);
    assert!(observed.alive);
    assert!(observed.expires_at_ms > observed.observed_at_ms);
    assert!(!serde_json::to_string(&observed)
        .unwrap()
        .contains("private_tool"));
    assert!(monitor.node_health("invalid/node/id").await.is_err());
    assert!(monitor
        .node_health(&format!("missing_{}", uuid::Uuid::new_v4().simple()))
        .await
        .unwrap()
        .is_none());
}
