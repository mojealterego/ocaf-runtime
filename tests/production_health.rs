//! v0.25 operational acceptance tests: per-cluster isolation, fail-closed
//! PostgreSQL readiness, and no credential leakage in health reports.
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::cluster::observability::{
    AlertPolicy, ClusterAlertCode, ClusterHealth, PgClusterMonitor,
};
use ocaf_runtime::jobs::JobSpec;
use serde_json::json;
use std::sync::Arc;

fn dsn() -> Option<String> {
    let d = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(d.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    d
}
fn task() -> JobSpec {
    JobSpec {
        id: uuid::Uuid::new_v4().to_string(),
        tool: "health_echo".into(),
        args: json!({"authorization": "sensitive-private-credential"}),
        retry_safe: false,
        max_attempts: 1,
    }
}

#[tokio::test]
async fn cluster_monitor_counts_only_jobs_owned_by_the_requested_cluster() {
    let Some(url) = dsn() else { return };
    let name_a = format!("monitor_a_{}", uuid::Uuid::new_v4().simple());
    let name_b = format!("monitor_b_{}", uuid::Uuid::new_v4().simple());
    let l_a = Arc::new(PgLeader::connect(&url, &name_a, true).await.unwrap());
    let l_b = Arc::new(PgLeader::connect(&url, &name_b, true).await.unwrap());
    assert!(l_a.try_acquire().await.unwrap());
    assert!(l_b.try_acquire().await.unwrap());
    let q_a = PgJobQueue::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&l_a));
    let q_b = PgJobQueue::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&l_b));
    q_a.submit_owned("owner", task()).await.unwrap();
    q_b.submit_owned("owner", task()).await.unwrap();
    q_b.submit_owned("owner", task()).await.unwrap();
    let a = PgClusterMonitor::connect(&url, &name_a, true)
        .await
        .unwrap();
    let b = PgClusterMonitor::connect(&url, &name_b, true)
        .await
        .unwrap();
    let snapshot_a = a.snapshot().await.unwrap();
    let snapshot_b = b.snapshot().await.unwrap();
    assert_eq!(snapshot_a.jobs_pending, 1);
    assert_eq!(snapshot_b.jobs_pending, 2);
    assert_eq!(snapshot_a.leader_sessions, 1);
    assert_eq!(snapshot_b.leader_sessions, 1);
    assert!(!serde_json::to_string(&snapshot_a)
        .unwrap()
        .contains("sensitive-private-credential"));
    l_a.release().await.unwrap();
    let degraded = a.report(AlertPolicy::default()).await.unwrap();
    assert_eq!(degraded.health, ClusterHealth::Critical);
    assert!(degraded
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::NoLeader));
    l_b.release().await.unwrap();
}

#[tokio::test]
async fn operator_health_checks_fail_closed_after_leader_session_crash() {
    let Some(url) = dsn() else { return };
    let name = format!("monitor_fail_{}", uuid::Uuid::new_v4().simple());
    let leader = PgLeader::connect(&url, &name, true).await.unwrap();
    let monitor = PgClusterMonitor::connect(&url, &name, true).await.unwrap();
    assert_eq!(
        monitor.report(AlertPolicy::default()).await.unwrap().health,
        ClusterHealth::Critical
    );
    assert!(leader.try_acquire().await.unwrap());
    assert_eq!(monitor.snapshot().await.unwrap().leader_sessions, 1);
    let pid = leader.backend_pid().await.unwrap();
    let (conn, driver) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .unwrap();
    let driver = tokio::spawn(async move {
        let _ = driver.await;
    });
    assert!(conn
        .query_one("SELECT pg_terminate_backend($1)", &[&pid])
        .await
        .unwrap()
        .get::<_, bool>(0));
    let report = monitor.report(AlertPolicy::default()).await.unwrap();
    assert_eq!(report.health, ClusterHealth::Critical);
    assert!(report
        .alerts
        .iter()
        .any(|a| a.code == ClusterAlertCode::NoLeader));
    driver.abort();
}
