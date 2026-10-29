//! Disposable PG migration/snapshot drill. Requires an explicitly dedicated database.
//! The phase workflow restores a pre-migration pg_dump between test invocations.
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::jobs::JobStatus;
use serde_json::json;

const ID: &str = "c48b73d2-d747-41f8-a031-123456789abc";

#[tokio::test]
async fn real_migration_snapshot_runtime_and_revoked_credential() {
    let Ok(phase) = std::env::var("OCAF_STAGE042_PHASE") else {
        assert!(
            std::env::var("OCAF_REQUIRE_STAGE042_TEST").is_err(),
            "phase required"
        );
        return;
    };
    let dsn = std::env::var("OCAF_STAGE042_DSN").expect("dedicated lab DSN required");
    let config: tokio_postgres::Config = dsn.parse().expect("valid lab DSN");
    assert_eq!(config.get_dbname(), Some("ocaf_stage042_rollback"));
    if phase == "revoked" {
        assert!(PgJobQueue::connect(&dsn, true).await.is_err());
        let fresh = std::env::var("OCAF_STAGE042_FRESH_DSN").expect("rotated DSN required");
        let queue = PgJobQueue::connect(&fresh, true)
            .await
            .expect("fresh credential works");
        assert_eq!(
            queue.get_owned("owner", ID).await.unwrap().unwrap().status,
            JobStatus::Succeeded
        );
        return;
    }
    let (client, connection) = tokio_postgres::connect(&dsn, tokio_postgres::NoTls)
        .await
        .unwrap();
    let task = tokio::spawn(async move { connection.await.unwrap() });
    match phase.as_str() {
        "prepare" => {
            client.batch_execute("CREATE TABLE ocaf_pg_jobs_v1 (
                seq BIGSERIAL PRIMARY KEY, id TEXT NOT NULL UNIQUE, owner TEXT NOT NULL,
                tool TEXT NOT NULL, args TEXT NOT NULL, retry_safe BOOLEAN NOT NULL DEFAULT FALSE,
                max_attempts INT NOT NULL DEFAULT 1,
                status TEXT NOT NULL CHECK(status IN ('pending','running','succeeded','failed','uncertain')),
                attempts INT NOT NULL DEFAULT 0, result TEXT, last_error TEXT, worker_id TEXT,
                lease_token TEXT, lease_expires_at_ms BIGINT
            )").await.unwrap();
            client.execute("INSERT INTO ocaf_pg_jobs_v1(id,owner,tool,args,status) VALUES($1,'owner','rollback_echo','{}','pending')", &[&ID]).await.unwrap();
        }
        "migrate" | "restored" => {
            let columns: i64 = client.query_one("SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='ocaf_pg_jobs_v1' AND column_name IN ('cluster_id','leader_epoch')", &[]).await.unwrap().get(0);
            assert_eq!(columns, 0, "snapshot must really restore the legacy schema");
            let queue = PgJobQueue::connect(&dsn, true).await.unwrap();
            let record = queue.get_owned("owner", ID).await.unwrap().unwrap();
            assert_eq!(record.status, JobStatus::Pending);
            assert_eq!(record.attempts, 0);
            assert!(record.result.is_none());
            let migrated = client
                .query_one(
                    "SELECT cluster_id,leader_epoch FROM ocaf_pg_jobs_v1 WHERE id=$1",
                    &[&ID],
                )
                .await
                .unwrap();
            assert_eq!(migrated.get::<_, String>(0), "");
            assert_eq!(migrated.get::<_, i64>(1), 0);
            let lease = queue
                .claim_permitted("worker", 0, 300_000, &["rollback_echo".to_owned()])
                .await
                .unwrap()
                .unwrap();
            queue
                .succeed(&lease, json!({"phase":phase}), 1)
                .await
                .unwrap();
            drop(queue);
            let reopened = PgJobQueue::connect(&dsn, true).await.unwrap();
            let persisted = reopened.get_owned("owner", ID).await.unwrap().unwrap();
            assert_eq!(persisted.status, JobStatus::Succeeded);
            assert_eq!(persisted.attempts, 1);
            assert_eq!(persisted.result, Some(json!({"phase":phase})));
            assert!(reopened
                .get_owned("foreign-owner", ID)
                .await
                .unwrap()
                .is_none());
            assert!(reopened
                .claim_permitted("worker", 2, 300_000, &["rollback_echo".to_owned()])
                .await
                .unwrap()
                .is_none());
        }
        _ => panic!("unknown phase"),
    }
    task.abort();
}
