//! Validate the operator probe's real PostgreSQL exit status and its
//! strictly metadata-only stdout. No network monitoring listener is started.
use std::process::Command;
use ocaf_runtime::cluster::leadership::PgLeader;

#[tokio::test]
async fn probe_reports_missing_leadership_and_recovers_without_leaking_database_secrets() {
    let Some(dsn) = std::env::var("OCAF_TEST_PG_DSN").ok() else {
        assert!(std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
        return;
    };
    let cluster = format!("health_cli_{}", uuid::Uuid::new_v4().simple());
    let probe = || {
        Command::new(env!("CARGO_BIN_EXE_ocaf-cluster-health"))
            .env("OCAF_CLUSTER_PG_DSN", &dsn)
            .env("OCAF_CLUSTER_ID", &cluster)
            .env("OCAF_ALLOW_INSECURE_LOOPBACK", "1")
            .output()
            .unwrap()
    };
    // Create the tables used by the read-only query.
    ocaf_runtime::cluster::jobs::PgJobQueue::connect(&dsn, true).await.unwrap();
    ocaf_runtime::cluster::directory::PgDirectory::connect(&dsn, true).await.unwrap();
    let before = probe();
    assert!(!before.status.success());
    let report: serde_json::Value = serde_json::from_slice(&before.stdout).unwrap();
    assert_eq!(report["health"], "critical");
    let leader = PgLeader::connect(&dsn, &cluster, true).await.unwrap();
    assert!(leader.try_acquire().await.unwrap());
    let healthy = probe();
    assert!(healthy.status.success(), "stderr should be generic: {}", String::from_utf8_lossy(&healthy.stderr));
    let payload = String::from_utf8(healthy.stdout).unwrap();
    assert!(!payload.contains("ocaf_local_test_password"));
    assert!(!payload.contains("OCAF_CLUSTER_PG_DSN"));
    let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(parsed["snapshot"]["leader_sessions"],1);
    leader.release().await.unwrap();
}
