use ocaf_runtime::jobs::{JobError, JobSpec, JobStatus, JobStore};
use serde_json::json;
use std::path::Path;
use uuid::Uuid;

fn spec(retry_safe: bool, max_attempts: u32) -> JobSpec {
    JobSpec {
        id: Uuid::new_v4().to_string(),
        tool: "safe_echo".into(),
        args: json!({"message":"hello"}),
        retry_safe,
        max_attempts,
    }
}

fn open(path: &Path) -> JobStore {
    JobStore::open(path).expect("SQLite job ledger")
}

#[tokio::test]
async fn persistent_queue_survives_restart_and_stores_results() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jobs.sqlite");
    let store = open(&path);
    let job = spec(false, 1);
    assert_eq!(
        store.submit(job.clone()).await.unwrap().status,
        JobStatus::Pending
    );
    let lease = store
        .claim("worker-a", 10_000, 2_000)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lease.id, job.id);
    assert_eq!(lease.attempt, 1);
    store
        .succeed(&lease, json!({"output":"completed"}), 10_100)
        .await
        .unwrap();
    drop(store);

    let restarted = open(&path);
    let persisted = restarted.get(&job.id).await.unwrap().unwrap();
    assert_eq!(persisted.status, JobStatus::Succeeded);
    assert_eq!(persisted.attempts, 1);
    assert_eq!(persisted.result, Some(json!({"output":"completed"})));
    assert!(restarted
        .claim("worker-b", 20_000, 2_000)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn repeated_submission_is_idempotent_only_when_payload_matches() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("jobs.sqlite"));
    let job = spec(true, 3);
    store.submit(job.clone()).await.unwrap();
    let duplicate = store.submit(job.clone()).await.unwrap();
    assert_eq!(duplicate.status, JobStatus::Pending);
    let mut altered = job;
    altered.args = json!({"message":"different"});
    assert!(matches!(
        store.submit(altered).await,
        Err(JobError::Conflict)
    ));
}

#[tokio::test]
async fn competing_workers_cannot_claim_the_same_pending_job() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("jobs.sqlite"));
    let job = spec(true, 2);
    store.submit(job).await.unwrap();
    let (one, two) = tokio::join!(
        store.claim("worker-a", 10_000, 2_000),
        store.claim("worker-b", 10_000, 2_000),
    );
    let obtained = usize::from(one.unwrap().is_some()) + usize::from(two.unwrap().is_some());
    assert_eq!(obtained, 1);
}

#[tokio::test]
async fn stale_lease_cannot_complete_reassigned_work() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("jobs.sqlite"));
    let job = spec(true, 3);
    store.submit(job.clone()).await.unwrap();
    let first = store.claim("worker-a", 10_000, 100).await.unwrap().unwrap();
    assert_eq!(store.recover_expired(10_101).await.unwrap().retried, 1);
    let second = store
        .claim("worker-b", 10_102, 1_000)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.attempt, 2);
    assert_ne!(first.lease_token, second.lease_token);
    assert!(matches!(
        store
            .succeed(&first, json!({"wrong":"stale"}), 10_103)
            .await,
        Err(JobError::StaleLease)
    ));
    store
        .succeed(&second, json!({"ok":true}), 10_200)
        .await
        .unwrap();
    assert_eq!(
        store.get(&job.id).await.unwrap().unwrap().status,
        JobStatus::Succeeded
    );
}

#[tokio::test]
async fn unsafe_to_retry_job_becomes_uncertain_after_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jobs.sqlite");
    let store = open(&path);
    let job = spec(false, 1);
    store.submit(job.clone()).await.unwrap();
    let lease = store.claim("worker-a", 5_000, 100).await.unwrap().unwrap();
    drop(store);
    let restarted = open(&path);
    assert_eq!(restarted.recover_expired(5_101).await.unwrap().uncertain, 1);
    assert_eq!(
        restarted.get(&job.id).await.unwrap().unwrap().status,
        JobStatus::Uncertain
    );
    assert!(matches!(
        restarted.succeed(&lease, json!({}), 5_102).await,
        Err(JobError::StaleLease)
    ));
    assert!(restarted
        .claim("worker-b", 5_102, 100)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn safe_retry_budget_exhaustion_becomes_uncertain_not_success() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("jobs.sqlite"));
    let job = spec(true, 2);
    store.submit(job.clone()).await.unwrap();
    let first = store.claim("worker-a", 1_000, 100).await.unwrap().unwrap();
    store.recover_expired(1_101).await.unwrap();
    let second = store.claim("worker-b", 1_102, 100).await.unwrap().unwrap();
    assert_eq!(second.attempt, 2);
    assert_ne!(first.lease_token, second.lease_token);
    assert_eq!(store.recover_expired(1_203).await.unwrap().uncertain, 1);
    assert_eq!(
        store.get(&job.id).await.unwrap().unwrap().status,
        JobStatus::Uncertain
    );
}

#[tokio::test]
async fn heartbeat_extends_lease_but_stale_heartbeat_is_denied() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("jobs.sqlite"));
    let job = spec(true, 2);
    store.submit(job).await.unwrap();
    let first = store.claim("worker-a", 1_000, 200).await.unwrap().unwrap();
    store.heartbeat(&first, 1_100, 300).await.unwrap();
    assert_eq!(store.recover_expired(1_201).await.unwrap().retried, 0);
    assert_eq!(store.recover_expired(1_401).await.unwrap().retried, 1);
    assert!(matches!(
        store.heartbeat(&first, 1_402, 300).await,
        Err(JobError::StaleLease)
    ));
}

#[tokio::test]
async fn completed_tool_failures_are_retried_only_if_explicitly_safe() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("jobs.sqlite"));
    let job = spec(true, 2);
    store.submit(job.clone()).await.unwrap();
    let first = store
        .claim("worker-a", 1_000, 1_000)
        .await
        .unwrap()
        .unwrap();
    store
        .fail(&first, "transient failure", 1_050)
        .await
        .unwrap();
    assert_eq!(
        store.get(&job.id).await.unwrap().unwrap().status,
        JobStatus::Pending
    );
    let second = store
        .claim("worker-b", 1_100, 1_000)
        .await
        .unwrap()
        .unwrap();
    store
        .fail(&second, "permanent failure", 1_200)
        .await
        .unwrap();
    assert_eq!(
        store.get(&job.id).await.unwrap().unwrap().status,
        JobStatus::Failed
    );
    assert!(store
        .claim("worker-c", 1_201, 1_000)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn invalid_specs_and_storage_failure_are_denied() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        JobStore::open(&dir.path().join("missing").join("jobs.sqlite")),
        Err(JobError::Storage)
    ));
    let store = open(&dir.path().join("jobs.sqlite"));
    let mut job = spec(false, 2);
    assert!(matches!(
        store.submit(job.clone()).await,
        Err(JobError::Invalid)
    ));
    job.retry_safe = true;
    job.max_attempts = 0;
    assert!(matches!(
        store.submit(job.clone()).await,
        Err(JobError::Invalid)
    ));
    job.max_attempts = 3;
    job.id = "not-a-uuid".into();
    assert!(matches!(store.submit(job).await, Err(JobError::Invalid)));
    assert!(matches!(
        store.claim("worker-a", 1000, 0).await,
        Err(JobError::Invalid)
    ));
}

#[tokio::test]
async fn two_independent_sqlite_connections_atomically_claim_only_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("jobs.sqlite");
    let first = open(&db);
    let second = open(&db);
    let job = spec(true, 2);
    first.submit(job.clone()).await.unwrap();

    let (a, b) = tokio::join!(
        first.claim("worker-a", 10_000, 2_000),
        second.claim("worker-b", 10_000, 2_000)
    );
    let admitted = usize::from(a.unwrap().is_some()) + usize::from(b.unwrap().is_some());
    assert_eq!(admitted, 1);
    assert_eq!(
        first.get(&job.id).await.unwrap().unwrap().status,
        JobStatus::Running
    );
}

#[cfg(unix)]
#[test]
fn job_database_file_is_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jobs.sqlite");
    let _store = open(&path);
    let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "job arguments/results must not be world-readable");
}
