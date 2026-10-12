//! PostgreSQL-backed high availability: stale leases must be quarantined
//! immediately after promotion, without touching another cluster.
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::jobs::{JobSpec, JobStatus};
use serde_json::json;
use std::sync::Arc;

fn dsn() -> Option<String> {
    let dsn = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(dsn.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    dsn
}

#[tokio::test]
async fn new_leader_immediately_quarantines_running_predecessor_lease() {
    let Some(url) = dsn() else { return };
    let cluster = format!("ha_a_{}", uuid::Uuid::new_v4().simple());
    let first = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    let second = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    assert!(first.try_acquire().await.unwrap());
    let db1 = PgJobQueue::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&first));
    let id = uuid::Uuid::new_v4().to_string();
    let tool = format!("ha_effect_{}", uuid::Uuid::new_v4().simple());
    db1.submit_owned(
        "owner",
        JobSpec {
            id: id.clone(),
            tool: tool.clone(),
            args: json!({}),
            retry_safe: false,
            max_attempts: 1,
        },
    )
    .await
    .unwrap();
    let lease = db1
        .claim_permitted("worker", 0, 300_000, &[tool])
        .await
        .unwrap()
        .unwrap();
    assert!(db1.validate_current_lease(&lease).await.unwrap());
    first.release().await.unwrap();
    assert!(second.try_acquire().await.unwrap());
    let db2 = PgJobQueue::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&second));
    assert!(!db2.validate_current_lease(&lease).await.unwrap());
    let summary = db2.recover_expired(0).await.unwrap();
    assert_eq!(
        summary.uncertain, 1,
        "promotion must quarantine unexpired stale leases"
    );
    assert_eq!(
        db2.get_owned("owner", &id).await.unwrap().unwrap().status,
        JobStatus::Uncertain
    );
    assert_eq!(db2.recover_expired(0).await.unwrap().uncertain, 0);
    assert!(
        db1.recover_expired(0).await.is_err(),
        "stale leader cannot mutate recovery"
    );
    second.release().await.unwrap();
}

#[tokio::test]
async fn independent_clusters_cannot_claim_or_recover_each_others_jobs() {
    let Some(url) = dsn() else { return };
    let cluster_a = format!("ha_ns_a_{}", uuid::Uuid::new_v4().simple());
    let cluster_b = format!("ha_ns_b_{}", uuid::Uuid::new_v4().simple());
    let a = Arc::new(PgLeader::connect(&url, &cluster_a, true).await.unwrap());
    let b = Arc::new(PgLeader::connect(&url, &cluster_b, true).await.unwrap());
    assert!(a.try_acquire().await.unwrap());
    assert!(b.try_acquire().await.unwrap());
    let qa = PgJobQueue::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&a));
    let qb = PgJobQueue::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&b));
    let id = uuid::Uuid::new_v4().to_string();
    let tool = format!("ha_ns_tool_{}", uuid::Uuid::new_v4().simple());
    qa.submit_owned(
        "owner",
        JobSpec {
            id: id.clone(),
            tool: tool.clone(),
            args: json!({}),
            retry_safe: false,
            max_attempts: 1,
        },
    )
    .await
    .unwrap();
    assert!(qb.get_owned("owner", &id).await.unwrap().is_none());
    assert!(qb
        .claim_permitted("worker", 0, 30_000, &[tool.clone()])
        .await
        .unwrap()
        .is_none());
    assert_eq!(qb.recover_expired(0).await.unwrap().uncertain, 0);
    let own = qa
        .claim_permitted("worker", 0, 300_000, &[tool])
        .await
        .unwrap()
        .unwrap();
    assert!(qa.validate_current_lease(&own).await.unwrap());
    assert!(qb.get_owned("owner", &id).await.unwrap().is_none());
    assert_eq!(qb.recover_expired(0).await.unwrap().uncertain, 0);
    assert!(qa.validate_current_lease(&own).await.unwrap());
    a.release().await.unwrap();
    b.release().await.unwrap();
}
