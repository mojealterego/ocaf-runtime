//! Regression tests for PostgreSQL-authoritative distributed lease time.
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::jobs::{JobError, JobSpec, JobStatus};
use serde_json::json;

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(
        value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "CI requires a PostgreSQL database"
    );
    value
}

fn job() -> JobSpec {
    JobSpec {
        id: uuid::Uuid::new_v4().to_string(),
        tool: format!("clock_{}", uuid::Uuid::new_v4().simple()),
        args: json!({"message":"lease timing"}),
        retry_safe: false,
        max_attempts: 1,
    }
}

#[tokio::test]
async fn skewed_coordinator_clock_does_not_change_lease_deadline() {
    let Some(url) = dsn() else { return };
    let queue = PgJobQueue::connect(&url, true).await.unwrap();
    let task = job();
    queue
        .submit_owned("owner-clock", task.clone())
        .await
        .unwrap();
    let lease = queue
        .claim_permitted(
            "worker-clock",
            u64::MAX,
            60_000,
            std::slice::from_ref(&task.tool),
        )
        .await
        .unwrap()
        .unwrap();
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .unwrap();
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    let db_now: i64 = client
        .query_one(
            "SELECT (EXTRACT(EPOCH FROM clock_timestamp())*1000)::BIGINT",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    let now = u64::try_from(db_now).unwrap();
    assert!(lease.lease_expires_at_ms > now + 50_000);
    assert!(lease.lease_expires_at_ms <= now + 60_000);
    // Other tests may expire different jobs in this shared CI database;
    // verify THIS job survived recovery rather than checking a global count.
    queue.recover_expired(u64::MAX).await.unwrap();
    assert_eq!(
        queue
            .get_owned("owner-clock", &task.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Running
    );
    queue
        .succeed(&lease, json!({"done": true}), u64::MAX)
        .await
        .unwrap();
    assert_eq!(
        queue
            .get_owned("owner-clock", &task.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Succeeded
    );
    driver.abort();
}

#[tokio::test]
async fn database_expiration_rejects_backdated_completion() {
    let Some(url) = dsn() else { return };
    let queue = PgJobQueue::connect(&url, true).await.unwrap();
    let task = job();
    queue
        .submit_owned("owner-expiry", task.clone())
        .await
        .unwrap();
    let lease = queue
        .claim_permitted("worker-expiry", 0, 60_000, std::slice::from_ref(&task.tool))
        .await
        .unwrap()
        .unwrap();
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .unwrap();
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    let changed = client
        .execute(
            "UPDATE ocaf_pg_jobs_v1
             SET lease_expires_at_ms =
                (EXTRACT(EPOCH FROM clock_timestamp())*1000)::BIGINT - 1
             WHERE id=$1 AND status='running'",
            &[&task.id],
        )
        .await
        .unwrap();
    assert_eq!(changed, 1);
    assert!(matches!(
        queue.succeed(&lease, json!({"stale": true}), 0).await,
        Err(JobError::StaleLease)
    ));
    queue.recover_expired(0).await.unwrap();
    assert_eq!(
        queue
            .get_owned("owner-expiry", &task.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Uncertain
    );
    driver.abort();
}
