//! Actual database-backed cross-process leadership integration tests.
//! GitHub Actions sets OCAF_TEST_PG_DSN and starts PostgreSQL. Without it,
//! tests are skipped locally (CI separately asserts it is configured).
use ocaf_runtime::cluster::leadership::{PgLeader, PgLeaderError};

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(
        value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "CI requires a live PostgreSQL instance"
    );
    value
}

#[tokio::test]
async fn only_one_connection_is_leader_across_independent_clients() {
    let Some(url) = dsn() else { return };
    let a = PgLeader::connect(&url, "ocaf-test-main", true)
        .await
        .unwrap();
    let b = PgLeader::connect(&url, "ocaf-test-main", true)
        .await
        .unwrap();
    assert!(a.try_acquire().await.unwrap());
    assert!(!b.try_acquire().await.unwrap());
    assert!(a.verify().await.is_ok());
    a.release().await.unwrap();
    assert!(b.try_acquire().await.unwrap());
    assert!(!a.try_acquire().await.unwrap());
    b.release().await.unwrap();
}

#[tokio::test]
async fn loss_of_session_unblocks_new_leader_without_lease_timeout() {
    let Some(url) = dsn() else { return };
    let a = PgLeader::connect(&url, "ocaf-test-crash", true)
        .await
        .unwrap();
    let b = PgLeader::connect(&url, "ocaf-test-crash", true)
        .await
        .unwrap();
    assert!(a.try_acquire().await.unwrap());
    assert!(!b.try_acquire().await.unwrap());
    drop(a);
    let mut promoted = false;
    for _ in 0..50 {
        if b.try_acquire().await.unwrap() {
            promoted = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        promoted,
        "leader session death must release the advisory lock"
    );
    b.release().await.unwrap();
}

#[tokio::test]
async fn separate_cluster_names_do_not_block_each_other() {
    let Some(url) = dsn() else { return };
    let a = PgLeader::connect(&url, "ocaf-test-a", true).await.unwrap();
    let b = PgLeader::connect(&url, "ocaf-test-b", true).await.unwrap();
    assert!(a.try_acquire().await.unwrap());
    assert!(b.try_acquire().await.unwrap());
    a.release().await.unwrap();
    b.release().await.unwrap();
}

#[tokio::test]
async fn transport_downgrade_to_plain_remote_database_is_rejected() {
    let err = PgLeader::connect(
        "host=example.org user=nobody dbname=postgres sslmode=disable",
        "cluster-1",
        true,
    )
    .await
    .err()
    .expect("remote plaintext denied");
    assert!(matches!(err, PgLeaderError::UnsafeTransport));
}

#[tokio::test]
async fn invalid_cluster_identity_fails_closed() {
    let err = PgLeader::connect(
        "host=127.0.0.1 user=nobody dbname=postgres sslmode=disable",
        "",
        true,
    )
    .await
    .err()
    .expect("invalid identity");
    assert!(matches!(err, PgLeaderError::InvalidCluster));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_elections_do_not_stack_postgres_session_locks() {
    use std::sync::Arc;
    use tokio::task::JoinSet;

    let Some(url) = dsn() else { return };
    let cluster = format!("concurrent_{}", uuid::Uuid::new_v4().simple());
    let active = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    let standby = PgLeader::connect(&url, &cluster, true).await.unwrap();
    let mut tasks = JoinSet::new();
    for _ in 0..32 {
        let candidate = Arc::clone(&active);
        tasks.spawn(async move { candidate.try_acquire().await });
    }
    while let Some(result) = tasks.join_next().await {
        assert!(result.unwrap().unwrap());
    }
    assert!(!standby.try_acquire().await.unwrap());
    let first_epoch = active.fencing_epoch().await.unwrap();
    active.release().await.unwrap();

    // PostgreSQL session advisory locks are reentrant. If several
    // acquisitions were stacked, this takeover would remain blocked.
    assert!(standby.try_acquire().await.unwrap());
    assert!(standby.fencing_epoch().await.unwrap() > first_epoch);
    assert!(active.verify().await.is_err());
    standby.release().await.unwrap();
}

#[tokio::test]
async fn fencing_epoch_is_durable_across_multiple_takeovers() {
    let Some(url) = dsn() else { return };
    let cluster = format!("epoch_{}", uuid::Uuid::new_v4().simple());
    let first = PgLeader::connect(&url, &cluster, true).await.unwrap();
    let second = PgLeader::connect(&url, &cluster, true).await.unwrap();
    assert!(first.try_acquire().await.unwrap());
    let epoch1 = first.fencing_epoch().await.unwrap();
    assert!(!second.try_acquire().await.unwrap());
    first.release().await.unwrap();
    assert!(first.fencing_epoch().await.is_err());
    assert!(second.try_acquire().await.unwrap());
    let epoch2 = second.fencing_epoch().await.unwrap();
    assert_eq!(epoch2, epoch1 + 1);
    second.release().await.unwrap();
    assert!(first.try_acquire().await.unwrap());
    assert_eq!(first.fencing_epoch().await.unwrap(), epoch2 + 1);
    first.release().await.unwrap();
}

#[tokio::test]
async fn changed_database_epoch_fences_a_still_connected_old_leader() {
    let Some(url) = dsn() else { return };
    let cluster = format!("fenced_{}", uuid::Uuid::new_v4().simple());
    let active = PgLeader::connect(&url, &cluster, true).await.unwrap();
    let standby = PgLeader::connect(&url, &cluster, true).await.unwrap();
    assert!(active.try_acquire().await.unwrap());
    let original_epoch = active.fencing_epoch().await.unwrap();
    let (db, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .unwrap();
    let task = tokio::spawn(async move {
        let _ = connection.await;
    });
    // Test-only epoch corruption models a stale leader seeing a newer
    // durable election record. It must fail closed despite SELECT 1.
    assert_eq!(
        db.execute(
            "UPDATE ocaf_pg_leader_epochs_v1 SET epoch=epoch+1 WHERE cluster_id=$1",
            &[&cluster],
        )
        .await
        .unwrap(),
        1
    );
    assert!(matches!(
        active.verify().await,
        Err(PgLeaderError::Database)
    ));
    assert!(!standby.try_acquire().await.unwrap());
    active.release().await.unwrap();
    assert!(standby.try_acquire().await.unwrap());
    assert_eq!(standby.fencing_epoch().await.unwrap(), original_epoch + 2);
    standby.release().await.unwrap();
    task.abort();
}
