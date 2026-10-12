//! 0.26 security acceptance: PostgreSQL capability routing must not cross
//! cluster namespaces, even for identical node IDs and tool names.
use std::sync::Arc;

use ocaf_runtime::cluster::directory::PgDirectory;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::cluster::observability::PgClusterMonitor;

const NOW: u64 = 1_800_000_000_000;

fn pg() -> Option<String> {
    let value = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(value.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    value
}

#[tokio::test]
async fn same_named_agents_are_isolated_and_stale_leader_cannot_modify_new_generation() {
    let Some(url) = pg() else { return };
    let one = format!("scope_one_{}", uuid::Uuid::new_v4().simple());
    let two = format!("scope_two_{}", uuid::Uuid::new_v4().simple());
    let a = Arc::new(PgLeader::connect(&url, &one, true).await.unwrap());
    let b = Arc::new(PgLeader::connect(&url, &two, true).await.unwrap());
    let replacement = Arc::new(PgLeader::connect(&url, &one, true).await.unwrap());
    assert!(a.try_acquire().await.unwrap());
    assert!(b.try_acquire().await.unwrap());

    let da = PgDirectory::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&a));
    let db = PgDirectory::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&b));

    // Same logical identity exists in both clusters. ACLs and capability
    // routing must not leak the capabilities of one cluster into another.
    da.heartbeat("shared-worker", &["tool_a".into()], NOW)
        .await
        .unwrap();
    db.heartbeat("shared-worker", &["tool_b".into()], NOW)
        .await
        .unwrap();
    assert!(da.resolve("tool_b", NOW + 1).await.unwrap().is_none());
    assert!(db.resolve("tool_a", NOW + 1).await.unwrap().is_none());
    assert_eq!(
        da.resolve("tool_a", NOW + 1)
            .await
            .unwrap()
            .unwrap()
            .node_id,
        "shared-worker"
    );
    assert_eq!(
        db.resolve("tool_b", NOW + 1)
            .await
            .unwrap()
            .unwrap()
            .node_id,
        "shared-worker"
    );

    let ma = PgClusterMonitor::connect(&url, &one, true).await.unwrap();
    let mb = PgClusterMonitor::connect(&url, &two, true).await.unwrap();
    assert_eq!(ma.snapshot().await.unwrap().agents_live, 1);
    assert_eq!(mb.snapshot().await.unwrap().agents_live, 1);
    assert!(ma.node_health("shared-worker").await.unwrap().is_some());

    a.release().await.unwrap();
    assert!(replacement.try_acquire().await.unwrap());
    assert!(da.heartbeat("shared-worker", &["tool_a".into()], NOW + 2).await.is_err());
    assert!(da.resolve("tool_a", NOW + 2).await.is_err());

    let refreshed = PgDirectory::connect(&url, true)
        .await
        .unwrap()
        .with_leader(Arc::clone(&replacement));
    refreshed
        .heartbeat("shared-worker", &["tool_c".into()], NOW + 2)
        .await
        .unwrap();
    assert!(refreshed.resolve("tool_a", NOW + 3).await.unwrap().is_none());
    assert!(db.resolve("tool_b", NOW + 3).await.unwrap().is_some());
    replacement.release().await.unwrap();
    b.release().await.unwrap();
}
