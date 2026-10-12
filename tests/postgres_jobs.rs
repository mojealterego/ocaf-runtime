//! Actual PostgreSQL shared durable job tests. CI supplies a live database.
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::jobs::{JobError, JobSpec, JobStatus};
use serde_json::json;

const NOW: u64 = 1_800_000_000_000;

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(
        value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "CI must configure PostgreSQL"
    );
    value
}

fn spec() -> JobSpec {
    JobSpec {
        id: uuid::Uuid::new_v4().to_string(),
        tool: "echo".into(),
        args: json!({"message":"remote job"}),
        retry_safe: false,
        max_attempts: 1,
    }
}

#[tokio::test]
async fn independent_connections_share_one_fenced_durable_job() {
    let Some(url) = dsn() else { return };
    let a = PgJobQueue::connect(&url, true).await.unwrap();
    let b = PgJobQueue::connect(&url, true).await.unwrap();
    let s = spec();
    a.submit_owned("submitter-a", s.clone()).await.unwrap();

    let allowed = vec!["echo".into()];
    let (x, y) = tokio::join!(
        a.claim_permitted("worker-a", NOW, 5_000, &allowed),
        b.claim_permitted("worker-b", NOW, 5_000, &allowed),
    );
    let claims: Vec<_> = [x.unwrap(), y.unwrap()].into_iter().flatten().collect();
    assert_eq!(
        claims.len(),
        1,
        "one job must not be assigned to two workers"
    );
    let lease = &claims[0];
    assert_eq!(lease.id, s.id);
    b.succeed(lease, json!({"echo":"done"}), NOW + 1)
        .await
        .unwrap();

    let rec = a.get_owned("submitter-a", &s.id).await.unwrap().unwrap();
    assert_eq!(rec.status, JobStatus::Succeeded);
    assert_eq!(rec.attempts, 1);
    assert_eq!(rec.result, Some(json!({"echo":"done"})));
    assert!(b.get_owned("submitter-b", &s.id).await.unwrap().is_none());
}

#[tokio::test]
async fn duplicate_uuid_rejects_other_owner_or_modified_payload() {
    let Some(url) = dsn() else { return };
    let db = PgJobQueue::connect(&url, true).await.unwrap();
    let s = spec();
    db.submit_owned("owner-a", s.clone()).await.unwrap();
    assert_eq!(
        db.submit_owned("owner-a", s.clone()).await.unwrap().status,
        JobStatus::Pending
    );
    assert!(matches!(
        db.submit_owned("owner-b", s.clone()).await,
        Err(JobError::Conflict)
    ));
    let mut changed = s;
    changed.args = json!({"message":"overwritten"});
    assert!(matches!(
        db.submit_owned("owner-a", changed).await,
        Err(JobError::Conflict)
    ));
}

#[tokio::test]
async fn crash_expiration_becomes_uncertain_and_stale_report_is_fenced() {
    let Some(url) = dsn() else { return };
    let first = PgJobQueue::connect(&url, true).await.unwrap();
    let job = spec();
    first
        .submit_owned("submitter-a", job.clone())
        .await
        .unwrap();
    let lease = first
        .claim_permitted("worker-a", NOW, 1_000, &["echo".into()])
        .await
        .unwrap()
        .unwrap();
    drop(first);

    let recovered = PgJobQueue::connect(&url, true).await.unwrap();
    assert_eq!(
        recovered
            .recover_expired(NOW + 1_001)
            .await
            .unwrap()
            .uncertain,
        1
    );
    assert!(matches!(
        recovered
            .succeed(&lease, json!({"forged":true}), NOW + 1_002)
            .await,
        Err(JobError::StaleLease)
    ));
    let record = recovered
        .get_owned("submitter-a", &job.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.status, JobStatus::Uncertain);
    assert_eq!(record.attempts, 1);
}

#[tokio::test]
async fn postgres_transport_downgrade_is_denied_by_default() {
    let err = PgJobQueue::connect(
        "host=example.org dbname=postgres user=nobody sslmode=disable",
        false,
    )
    .await
    .err()
    .expect("plaintext remote database denied");
    assert!(matches!(err, JobError::Invalid));
}
