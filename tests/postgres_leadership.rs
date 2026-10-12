//! Actual database-backed cross-process leadership integration tests.
//! GitHub Actions sets OCAF_TEST_PG_DSN and starts PostgreSQL. Without it,
//! tests are skipped locally (CI separately asserts it is configured).
use ocaf_runtime::cluster::leadership::{PgLeader, PgLeaderError};

fn dsn() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err(),
        "CI requires a live PostgreSQL instance");
    value
}

#[tokio::test]
async fn only_one_connection_is_leader_across_independent_clients() {
    let Some(url) = dsn() else { return };
    let a = PgLeader::connect(&url, "ocaf-test-main", true).await.unwrap();
    let b = PgLeader::connect(&url, "ocaf-test-main", true).await.unwrap();
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
    let a = PgLeader::connect(&url, "ocaf-test-crash", true).await.unwrap();
    let b = PgLeader::connect(&url, "ocaf-test-crash", true).await.unwrap();
    assert!(a.try_acquire().await.unwrap());
    assert!(!b.try_acquire().await.unwrap());
    a.simulate_crash().await;
    let mut promoted = false;
    for _ in 0..50 {
        if b.try_acquire().await.unwrap() {
            promoted = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(promoted, "leader session death must release the advisory lock");
    assert!(a.verify().await.is_err(), "dead sessions cannot authorize requests");
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
    ).await.err().expect("remote plaintext denied");
    assert!(matches!(err, PgLeaderError::UnsafeTransport));
}

#[tokio::test]
async fn invalid_cluster_identity_fails_closed() {
    let err = PgLeader::connect(
        "host=127.0.0.1 user=nobody dbname=postgres sslmode=disable",
        "",
        true,
    ).await.err().expect("invalid identity");
    assert!(matches!(err, PgLeaderError::InvalidCluster));
}
