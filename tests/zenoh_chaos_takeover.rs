#![cfg(feature = "zenoh")]
//! Real Zenoh transport + PostgreSQL crash/restart chaos scenario.
//! Three separate Zenoh sessions simulate isolated transport endpoints;
//! this is NOT a physical multi-host or dual-PostgreSQL-primary test.
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{ReconnectPolicy, ResilientZenohRpcClient, ZenohRpcServer};
use serde_json::json;

const SECRET: [u8; 32] = [0xA4; 32];

fn peers() -> HashMap<String, PeerPolicy> {
    HashMap::from([(
        "submitter".to_string(),
        PeerPolicy::new(SECRET, vec!["safe_echo".to_string()]).allow_job_submission(),
    )])
}

async fn gateway(dsn: &str, leader: Arc<PgLeader>) -> anyhow::Result<RpcGateway> {
    Ok(RpcGateway::new("coord", OcafRuntime::new(0), peers(), 16)?
        .with_postgres_replay(dsn, true)
        .await?
        .with_postgres_coordinator(dsn, true)
        .await?
        .with_postgres_job_db(dsn, true)
        .await?
        .with_pg_leader(leader))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn zenoh_coordinator_host_loss_and_postgres_takeover_preserve_job_and_retry_idempotency(
) -> anyhow::Result<()> {
    let dsn = match std::env::var("OCAF_TEST_PG_DSN") {
        Ok(v) => v,
        Err(_) if std::env::var("OCAF_REQUIRE_PG_TESTS").is_err() => return Ok(()),
        Err(_) => anyhow::bail!("chaos suite requires live PostgreSQL"),
    };
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let router_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"peer",listen:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    ))
    .map_err(|e| anyhow::anyhow!("router Zenoh config: {e}"))?;
    let client_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"client",connect:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    ))
    .map_err(|e| anyhow::anyhow!("client Zenoh config: {e}"))?;
    let router = zenoh::open(router_cfg)
        .await
        .map_err(|e| anyhow::anyhow!("Zenoh router: {e}"))?;

    let cluster = format!("chaos_zenoh_{}", uuid::Uuid::new_v4().simple());
    let a = Arc::new(PgLeader::connect(&dsn, &cluster, true).await?);
    let b = Arc::new(PgLeader::connect(&dsn, &cluster, true).await?);
    assert!(a.try_acquire().await?);
    assert!(!b.try_acquire().await?);
    let epoch_a = a.fencing_epoch().await?;

    let session_a = Arc::new(
        zenoh::open(client_cfg.clone())
            .await
            .map_err(|e| anyhow::anyhow!("open first coordinator: {e}"))?,
    );
    let server_a = ZenohRpcServer::new(session_a, Arc::new(gateway(&dsn, Arc::clone(&a)).await?));
    let server_a_task = tokio::spawn(async move { server_a.serve().await });
    tokio::time::sleep(Duration::from_millis(450)).await;

    let policy = ReconnectPolicy::new(
        4,
        Duration::from_millis(100),
        Duration::from_millis(400),
        Duration::from_secs(7),
    )?;
    let client = ResilientZenohRpcClient::new(
        client_cfg.clone(),
        RpcClient::new("submitter", "coord", SECRET)?,
        policy,
    );
    let id = uuid::Uuid::new_v4().to_string();
    let submitted = client
        .request(RpcAction::SubmitJob {
            id: id.clone(),
            tool: "safe_echo".into(),
            args: json!({"body": "durable-across-transport-loss"}),
        })
        .await?;
    assert_eq!(submitted.status, RpcStatus::Ok);
    let first = PgJobQueue::connect(&dsn, true)
        .await?
        .with_leader(Arc::clone(&a));
    assert_eq!(
        first.get_owned("submitter", &id).await?.unwrap().attempts,
        0
    );

    // Simulate loss of the whole coordinator process: drop the
    // queryable and abruptly terminate its dedicated PostgreSQL backend.
    server_a_task.abort();
    let _ = server_a_task.await;
    let pid = a.backend_pid().await?;
    let (pg, connection) = tokio_postgres::connect(&dsn, tokio_postgres::NoTls).await?;
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    let terminated: bool = pg
        .query_one("SELECT pg_terminate_backend($1)", &[&pid])
        .await?
        .get(0);
    assert!(terminated);
    let promoted = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if b.try_acquire().await? {
                break Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await??;
    assert_eq!(promoted, ());
    assert!(b.fencing_epoch().await? > epoch_a);
    assert!(a.verify().await.is_err());

    let second = PgJobQueue::connect(&dsn, true)
        .await?
        .with_leader(Arc::clone(&b));
    assert_eq!(second.recover_expired(0).await?.uncertain, 0);

    let session_b = Arc::new(
        zenoh::open(client_cfg)
            .await
            .map_err(|e| anyhow::anyhow!("open replacement coordinator: {e}"))?,
    );
    let server_b = ZenohRpcServer::new(session_b, Arc::new(gateway(&dsn, Arc::clone(&b)).await?));
    let server_b_task = tokio::spawn(async move { server_b.serve().await });
    tokio::time::sleep(Duration::from_millis(450)).await;

    // Client creates a new Zenoh connection after the coordinator host
    // replacement. A repeated job UUID cannot create a second job.
    let repeated = client
        .request(RpcAction::SubmitJob {
            id: id.clone(),
            tool: "safe_echo".into(),
            args: json!({"body": "durable-across-transport-loss"}),
        })
        .await?;
    assert_eq!(repeated.status, RpcStatus::Ok);
    let status = client.request(RpcAction::GetJob { id: id.clone() }).await?;
    assert_eq!(status.output["status"], "pending");
    let history = client
        .request(RpcAction::GetJobAudit { id, limit: 20 })
        .await?;
    assert_eq!(history.status, RpcStatus::Ok);
    assert_eq!(history.output.as_array().unwrap().len(), 1);
    assert_eq!(history.output[0]["event"], "submitted");

    server_b_task.abort();
    let _ = server_b_task.await;
    b.release().await?;
    drop(router);
    driver.abort();
    Ok(())
}
