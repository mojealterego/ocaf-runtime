use ocaf_runtime::cluster::directory::PgDirectory;
use ocaf_runtime::cluster::jobs::PgJobQueue;
use serde_json::Value;
use std::process::Command;

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(
        value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "PostgreSQL CI service is required"
    );
    value
}

#[tokio::test]
async fn operator_cli_prints_bounded_secret_free_json_and_prometheus() {
    let Some(url) = dsn() else { return };
    // The read-only CLI is never permitted to initialize/mutate schemas.
    PgDirectory::connect(&url, true).await.unwrap();
    PgJobQueue::connect(&url, true).await.unwrap();

    let cluster = format!("health_cli_{}", uuid::Uuid::new_v4().simple());
    let bin = env!("CARGO_BIN_EXE_ocaf-cluster-health");
    let json = Command::new(bin)
        .args(["--json", "--no-fail"])
        .env("OCAF_CLUSTER_PG_DSN", &url)
        .env("OCAF_CLUSTER_ID", &cluster)
        .env("OCAF_ALLOW_INSECURE_LOOPBACK", "1")
        .output()
        .unwrap();
    assert!(json.status.success(), "{:?}", json.stderr);
    let metrics: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(metrics["health"], "critical");
    assert_eq!(metrics["snapshot"]["leader_sessions"], 0);
    assert!(metrics["snapshot"]["observed_at_ms"].as_u64().unwrap() > 1_700_000_000_000);
    let stdout = String::from_utf8(json.stdout).unwrap();
    assert!(!stdout.contains("ocaf_local_test_password"));
    assert!(!stdout.contains("private_key"));

    let prom = Command::new(bin)
        .args(["--prometheus", "--no-fail"])
        .env("OCAF_CLUSTER_PG_DSN", &url)
        .env("OCAF_CLUSTER_ID", &cluster)
        .env("OCAF_ALLOW_INSECURE_LOOPBACK", "1")
        .output()
        .unwrap();
    assert!(prom.status.success(), "{:?}", prom.stderr);
    let text = String::from_utf8(prom.stdout).unwrap();
    assert!(text.contains("ocaf_cluster_leader_sessions 0"));
    assert!(text.contains("ocaf_cluster_jobs_pending "));
    assert!(text.contains("ocaf_cluster_health_level 2"));
    assert!(!text.contains("ocaf_local_test_password"));

    // Without --no-fail the operator can hook up exit-status alerting.
    let critical = Command::new(bin)
        .args(["--json"])
        .env("OCAF_CLUSTER_PG_DSN", &url)
        .env("OCAF_CLUSTER_ID", &cluster)
        .env("OCAF_ALLOW_INSECURE_LOOPBACK", "1")
        .output()
        .unwrap();
    assert_eq!(critical.status.code(), Some(2));
}

#[test]
fn operator_cli_rejects_unrecognized_flags_before_opening_database() {
    let result = Command::new(env!("CARGO_BIN_EXE_ocaf-cluster-health"))
        .arg("--listen-public")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unknown option"));
}
