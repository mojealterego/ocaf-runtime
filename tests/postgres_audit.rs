//! Operational resilience: transactional PostgreSQL job audit and recovery.
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::jobs::{JobError, JobSpec, JobStatus};
use serde_json::json;

const NOW: u64 = 1_800_000_000_000;

async fn expire_lease_in_postgres(url: &str, id: &str) {
    // Inject a genuinely expired lease using the database's clock.
    let (client, connection) = tokio_postgres::connect(url, tokio_postgres::NoTls)
        .await
        .unwrap();
    let task = tokio::spawn(async move {
        let _ = connection.await;
    });
    let changed = client
        .execute(
            "UPDATE ocaf_pg_jobs_v1
             SET lease_expires_at_ms =
               (EXTRACT(EPOCH FROM clock_timestamp())*1000)::BIGINT - 1
             WHERE id=$1 AND status='running'",
            &[&id],
        )
        .await
        .unwrap();
    assert_eq!(changed, 1);
    task.abort();
}


fn db() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    value
}

fn spec() -> JobSpec {
    JobSpec {
        id: uuid::Uuid::new_v4().to_string(),
        tool: format!("audit_{}", uuid::Uuid::new_v4().simple()),
        args: json!({"private_data":"do not expose job inputs in audit"}),
        retry_safe: false,
        max_attempts: 1,
    }
}

#[tokio::test]
async fn audit_is_transactional_owner_scoped_and_cross_connection() {
    let Some(url) = db() else { return };
    let primary = PgJobQueue::connect(&url, true).await.unwrap();
    let observer = PgJobQueue::connect(&url, true).await.unwrap();
    let job = spec();
    primary.submit_owned("owner-a", job.clone()).await.unwrap();
    primary.submit_owned("owner-a", job.clone()).await.unwrap();
    assert!(matches!(
        primary.submit_owned("owner-b", job.clone()).await,
        Err(JobError::Conflict)
    ));
    let lease = observer
        .claim_permitted("worker-a", NOW, 10_000, std::slice::from_ref(&job.tool))
        .await
        .unwrap()
        .unwrap();
    primary
        .succeed(&lease, json!({"result":"SECRET"}), NOW + 1)
        .await
        .unwrap();

    let history = observer.audit_owned("owner-a", &job.id, 20).await.unwrap();
    let transitions: Vec<_> = history.iter().map(|e| e.event.as_str()).collect();
    assert_eq!(transitions, vec!["submitted", "claimed", "succeeded"]);
    assert_eq!(history[0].actor, "owner-a");
    assert_eq!(history[1].actor, "worker-a");
    assert_eq!(history[2].actor, "worker-a");
    assert!(history
        .windows(2)
        .all(|two| two[0].sequence < two[1].sequence));

    // Neither payload nor result nor raw lease secret may enter the journal.
    let serialized = serde_json::to_string(&history).unwrap();
    assert!(!serialized.contains("private_data"));
    assert!(!serialized.contains("SECRET"));
    assert!(!serialized.contains(&lease.lease_token));
    assert!(observer
        .audit_owned("owner-b", &job.id, 20)
        .await
        .unwrap()
        .is_empty());
    assert!(observer.audit_owned("owner-a", &job.id, 0).await.is_err());
    assert!(observer.audit_owned("owner-a", &job.id, 101).await.is_err());
}

#[tokio::test]
async fn recovery_writes_one_uncertain_event_and_rejects_stale_completion() {
    let Some(url) = db() else { return };
    let first = PgJobQueue::connect(&url, true).await.unwrap();
    let job = spec();
    first.submit_owned("owner-c", job.clone()).await.unwrap();
    let lease = first
        .claim_permitted("worker-c", NOW, 1000, std::slice::from_ref(&job.tool))
        .await
        .unwrap()
        .unwrap();
    drop(first);
    expire_lease_in_postgres(&url, &job.id).await;
    let replacement = PgJobQueue::connect(&url, true).await.unwrap();
    assert_eq!(
        replacement
            .recover_expired(NOW + 1001)
            .await
            .unwrap()
            .uncertain,
        1
    );
    assert_eq!(
        replacement
            .recover_expired(NOW + 1002)
            .await
            .unwrap()
            .uncertain,
        0
    );
    assert!(matches!(
        replacement
            .succeed(&lease, json!({"forged":true}), NOW + 1003)
            .await,
        Err(JobError::StaleLease)
    ));
    assert_eq!(
        replacement
            .get_owned("owner-c", &job.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Uncertain
    );
    let history = replacement
        .audit_owned("owner-c", &job.id, 20)
        .await
        .unwrap();
    assert_eq!(
        history.iter().map(|e| e.event.as_str()).collect::<Vec<_>>(),
        vec!["submitted", "claimed", "uncertain"]
    );
    assert_eq!(history[2].actor, "coordinator");
}
