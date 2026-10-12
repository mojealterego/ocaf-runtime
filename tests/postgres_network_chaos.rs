//! Deterministic packet-level PostgreSQL fault scenarios with a loopback-only
//! TCP proxy. CI runs against the real PostgreSQL service, not mocks.
use std::sync::Arc;
use std::time::{Duration, Instant};

use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::jobs::{JobSpec, JobStatus};
use serde_json::json;

#[path = "support/pg_fault_proxy.rs"]
mod pg_fault_proxy;
use pg_fault_proxy::PgFaultProxy;

fn database() -> Option<String> {
    let dsn = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(dsn.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    dsn
}

fn spec() -> JobSpec {
    JobSpec {
        id: uuid::Uuid::new_v4().to_string(),
        tool: format!("partition_{}", uuid::Uuid::new_v4().simple()),
        args: json!({"action": "external"}),
        retry_safe: false,
        max_attempts: 1,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn network_partition_drops_leadership_fences_worker_and_allows_fresh_election() {
    let Some(dsn) = database() else { return };
    let proxy = PgFaultProxy::bind(&dsn).await.unwrap();
    let proxied = proxy.proxied_dsn(&dsn).unwrap();
    let cluster = format!("net_partition_{}", uuid::Uuid::new_v4().simple());
    let old = Arc::new(PgLeader::connect(&proxied, &cluster, true).await.unwrap());
    let standby = Arc::new(PgLeader::connect(&dsn, &cluster, true).await.unwrap());
    assert!(old.try_acquire().await.unwrap());
    assert!(!standby.try_acquire().await.unwrap());
    let db1 = PgJobQueue::connect(&dsn, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&old));
    let job = spec();
    db1.submit_owned("owner", job.clone()).await.unwrap();
    let leased = db1
        .claim_permitted("worker", 0, 300_000, std::slice::from_ref(&job.tool))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(leased.leader_epoch, old.fencing_epoch().await.unwrap());

    let before = Instant::now();
    proxy.partition();
    let acquired = tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            if standby.try_acquire().await.unwrap() {
                break true;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("standby should take over after TCP session loss");
    assert!(acquired);
    assert!(before.elapsed() < Duration::from_secs(6));
    assert!(proxy.terminated_connections() >= 1);
    let db2 = PgJobQueue::connect(&dsn, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&standby));
    assert!(old.verify().await.is_err());
    assert!(!db2.validate_current_lease(&leased).await.unwrap());
    assert_eq!(db2.recover_expired(0).await.unwrap().uncertain, 1);
    assert_eq!(
        db2.get_owned("owner", &job.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Uncertain
    );
    assert!(db1.submit_owned("owner", spec()).await.is_err());
    assert!(db1.recover_expired(0).await.is_err());
    assert_eq!(db2.recover_expired(0).await.unwrap().uncertain, 0);

    proxy.heal();
    let fresh = PgLeader::connect(&proxied, &cluster, true).await.unwrap();
    assert!(!fresh.try_acquire().await.unwrap());
    let second_epoch = standby.fencing_epoch().await.unwrap();
    standby.release().await.unwrap();
    assert!(fresh.try_acquire().await.unwrap());
    assert!(fresh.fencing_epoch().await.unwrap() > second_epoch);
    fresh.release().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blackholed_postgres_handshake_times_out_without_publishing_an_epoch() {
    let Some(dsn) = database() else { return };
    let proxy = PgFaultProxy::bind(&dsn).await.unwrap();
    let proxied = proxy.proxied_dsn(&dsn).unwrap();
    let cluster = format!("net_blackhole_{}", uuid::Uuid::new_v4().simple());
    proxy.blackhole_new_connections();
    let before = Instant::now();
    let denied = PgLeader::connect(&proxied, &cluster, true).await;
    assert!(denied.is_err(), "network blackhole cannot create leader");
    assert!(before.elapsed() < Duration::from_secs(8));
    proxy.heal();
    let healthy = PgLeader::connect(&proxied, &cluster, true).await.unwrap();
    assert!(healthy.try_acquire().await.unwrap());
    assert_eq!(healthy.fencing_epoch().await.unwrap(), 1);
    healthy.release().await.unwrap();
}
