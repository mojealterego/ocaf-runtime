//! Independent processes exercising the opt-in external multi-host probe
//! against an isolated PostgreSQL 16 instance on the CI runner.
#![cfg(feature = "ha-probe")]

use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

fn postgres() -> Option<String> {
    let result = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(result.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    result
}

fn command(cluster: &str, dsn: &str) -> Command {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ocaf-ha-probe"));
    child
        .env("OCAF_HA_PROBE_ACK", "isolated-test-only")
        .env("OCAF_HA_PROBE_CLUSTER_ID", cluster)
        .env("OCAF_CLUSTER_PG_DSN", dsn)
        .env_remove("OCAF_CLUSTER_PG_DSN_FILE")
        .env("OCAF_ALLOW_INSECURE_LOOPBACK", "1")
        .stderr(Stdio::piped());
    child
}

async fn attempt(
    cluster: &str,
    dsn: &str,
    expect: &str,
    min_epoch: Option<i64>,
) -> serde_json::Value {
    let mut command = command(cluster, dsn);
    command.args(["--mode", "attempt", "--expect", expect]);
    if let Some(epoch) = min_epoch {
        command.args(["--min-epoch", &epoch.to_string()]);
    }
    let output = timeout(Duration::from_secs(10), command.output())
        .await
        .expect("bounded probe attempt")
        .expect("start probe attempt");
    assert!(
        output.status.success(),
        "probe attempt failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("one JSON record")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_processes_reject_concurrent_leader_and_advance_epoch() {
    let Some(dsn) = postgres() else { return };
    let cluster = format!("ocaf33_{}", uuid::Uuid::new_v4().simple());
    let mut hold = command(&cluster, &dsn);
    hold.args(["--mode", "hold", "--hold-ms", "2500"]);
    hold.stdout(Stdio::piped());
    let mut child = hold.spawn().expect("start independent leader process");
    let stdout = child.stdout.take().expect("probe stdout");
    let mut lines = BufReader::new(stdout).lines();
    let start = timeout(Duration::from_secs(10), lines.next_line())
        .await
        .expect("bounded startup")
        .expect("read JSON")
        .expect("hold acquired event");
    let event: serde_json::Value = serde_json::from_str(&start).expect("valid event");
    assert_eq!(event["outcome"], "acquired");
    assert_eq!(event["cluster_id"], cluster);
    let epoch = event["epoch"].as_i64().expect("epoch");
    assert!(epoch > 0);

    let denial = attempt(&cluster, &dsn, "denied", None).await;
    assert_eq!(denial["outcome"], "denied");
    assert!(denial["epoch"].is_null());

    let exit = timeout(Duration::from_secs(8), child.wait())
        .await
        .expect("bounded release")
        .expect("hold completion");
    assert!(exit.success());
    let release = timeout(Duration::from_secs(2), lines.next_line())
        .await
        .expect("release event timeout")
        .expect("release event read")
        .expect("release event");
    let released: serde_json::Value = serde_json::from_str(&release).expect("release JSON");
    assert_eq!(released["outcome"], "released");
    assert_eq!(released["epoch"], epoch);

    let election = attempt(&cluster, &dsn, "acquired", Some(epoch + 1)).await;
    assert_eq!(election["outcome"], "acquired");
    assert!(election["epoch"].as_i64().expect("second epoch") > epoch);
}

#[tokio::test]
async fn refuses_missing_operator_ack_without_touching_database() {
    let cluster = format!("ocaf33_{}", uuid::Uuid::new_v4().simple());
    let mut child = Command::new(env!("CARGO_BIN_EXE_ocaf-ha-probe"));
    child
        .env_remove("OCAF_HA_PROBE_ACK")
        .env("OCAF_HA_PROBE_CLUSTER_ID", cluster)
        .args(["--mode", "attempt", "--expect", "denied"]);
    let output = child.output().await.expect("run guard");
    assert!(!output.status.success());
}

#[tokio::test]
async fn refuses_non_test_namespace_before_opening_database() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ocaf-ha-probe"));
    child
        .env("OCAF_HA_PROBE_ACK", "isolated-test-only")
        .env("OCAF_HA_PROBE_CLUSTER_ID", "production")
        .args(["--mode", "attempt", "--expect", "denied"]);
    let output = child.output().await.expect("run guard");
    assert!(!output.status.success());
}
