//! Signed RPC replay capacity and nonce admission must not collide
//! across independent PostgreSQL clusters on the same database.
use std::sync::Arc;

use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::cluster::replay::PgReplay;
use ocaf_runtime::rpc::RpcError;

const NOW: u64 = 1_800_000_000_000;

#[tokio::test]
async fn identical_rpc_request_ids_are_unique_per_cluster_and_stale_generation_is_denied() {
    let url = match std::env::var("OCAF_TEST_PG_DSN") {
        Ok(value) => value,
        Err(_) => {
            assert!(std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
            return;
        }
    };
    let first = format!("replay_a_{}", uuid::Uuid::new_v4().simple());
    let second = format!("replay_b_{}", uuid::Uuid::new_v4().simple());
    let a = Arc::new(PgLeader::connect(&url, &first, true).await.unwrap());
    let b = Arc::new(PgLeader::connect(&url, &second, true).await.unwrap());
    let successor = Arc::new(PgLeader::connect(&url, &first, true).await.unwrap());
    assert!(a.try_acquire().await.unwrap());
    assert!(b.try_acquire().await.unwrap());

    let journal_a = PgReplay::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&a));
    let journal_b = PgReplay::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&b));

    let nonce = uuid::Uuid::new_v4().to_string();
    journal_a
        .reserve("coord", "worker", &nonce, NOW + 4000, NOW)
        .await
        .unwrap();
    journal_b
        .reserve("coord", "worker", &nonce, NOW + 4000, NOW)
        .await
        .unwrap();

    assert!(matches!(
        journal_a
            .reserve("coord", "worker", &nonce, NOW + 4000, NOW)
            .await,
        Err(RpcError::Replay)
    ));
    assert!(matches!(
        journal_b.reserve("coord", "worker", &nonce, NOW + 4000, NOW).await,
        Err(RpcError::Replay)
    ));

    a.release().await.unwrap();
    assert!(successor.try_acquire().await.unwrap());
    let next = uuid::Uuid::new_v4().to_string();
    assert!(journal_a.reserve("coord", "worker", &next, NOW + 4000, NOW)
        .await.is_err());
    let replacement = PgReplay::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&successor));
    replacement.reserve("coord", "worker", &next, NOW + 4000, NOW)
        .await.unwrap();
    assert_eq!(
        journal_b.reserve("coord", "worker", &next, NOW + 4000, NOW)
            .await.is_ok(),
        true
    );
    successor.release().await.unwrap();
    b.release().await.unwrap();
}
