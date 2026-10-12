#![cfg(feature = "zenoh")]
use std::collections::{HashMap, HashSet};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::jobs::remote::RemoteJobWorker;
use ocaf_runtime::jobs::JobStatus;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{ZenohRpcClient, ZenohRpcServer};
use ocaf_runtime::transport::{pricing_tool_manifest, EchoTool};
use serde_json::json;

const SUBMIT_KEY: [u8; 32] = [0x71; 32];
const WORKER_KEY: [u8; 32] = [0x72; 32];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_zenoh_remote_worker_executes_shared_postgres_job() -> anyhow::Result<()> {
    let pg = match std::env::var("OCAF_TEST_PG_DSN") {
        Ok(value) => value,
        Err(_) if std::env::var("OCAF_REQUIRE_PG_TESTS").is_err() => return Ok(()),
        Err(_) => anyhow::bail!("CI requires a PostgreSQL database"),
    };
    let dir = tempfile::tempdir()?;
    let sock = TcpListener::bind("127.0.0.1:0")?;
    let port = sock.local_addr()?.port();
    drop(sock);
    let coordinator_config = zenoh::Config::from_json5(&format!(
        r#"{{mode:"peer",listen:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    ))
    .map_err(|e| anyhow::anyhow!("coordinator config: {e}"))?;
    let client_config = zenoh::Config::from_json5(&format!(
        r#"{{mode:"client",connect:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    ))
    .map_err(|e| anyhow::anyhow!("client config: {e}"))?;
    let coordinator_session = Arc::new(
        zenoh::open(coordinator_config)
            .await
            .map_err(|e| anyhow::anyhow!("coordinator: {e}"))?,
    );
    let worker_session = Arc::new(
        zenoh::open(client_config.clone())
            .await
            .map_err(|e| anyhow::anyhow!("worker: {e}"))?,
    );
    let submitter_session = Arc::new(
        zenoh::open(client_config)
            .await
            .map_err(|e| anyhow::anyhow!("submitter: {e}"))?,
    );
    let peers = HashMap::from([
        (
            "submitter".into(),
            PeerPolicy::new(SUBMIT_KEY, vec!["demo_echo".into()]).allow_job_submission(),
        ),
        (
            "worker-a".into(),
            PeerPolicy::new(WORKER_KEY, vec!["demo_echo".into()]).allow_job_worker(),
        ),
    ]);
    let gateway = RpcGateway::with_sqlite_replay(
        "coord",
        OcafRuntime::new(0),
        peers,
        16,
        &dir.path().join("replay.sqlite"),
    )?
    .with_coordinator_db(&dir.path().join("members.sqlite"))?
    .with_postgres_job_db(&pg, true)
    .await?;
    let server = ZenohRpcServer::new(coordinator_session, Arc::new(gateway));
    let handle = tokio::spawn(async move { server.serve().await });
    tokio::time::sleep(Duration::from_millis(400)).await;

    let submitter = ZenohRpcClient::new(
        submitter_session,
        RpcClient::new("submitter", "coord", SUBMIT_KEY)?,
    );
    let executor = OcafRuntime::new(0);
    executor.register(pricing_tool_manifest(), EchoTool).await?;
    let worker = RemoteJobWorker::new(
        ZenohRpcClient::new(
            worker_session,
            RpcClient::new("worker-a", "coord", WORKER_KEY)?,
        ),
        executor,
        HashSet::from(["demo_echo".into()]),
    )?;
    worker.heartbeat().await?;
    let job_id = uuid::Uuid::new_v4().to_string();
    let submitted = submitter
        .request(RpcAction::SubmitJob {
            id: job_id.clone(),
            tool: "demo_echo".into(),
            args: json!({"message":"distributed round trip"}),
        })
        .await?;
    assert_eq!(submitted.status, RpcStatus::Ok);
    assert_eq!(submitted.output["job_id"], job_id);

    assert_eq!(worker.poll_once().await?, Some(JobStatus::Succeeded));
    assert_eq!(worker.poll_once().await?, None);
    let stored = PgJobQueue::connect(&pg, true)
        .await?
        .get_owned("submitter", &job_id)
        .await?
        .unwrap();
    assert_eq!(stored.status, JobStatus::Succeeded);
    assert_eq!(stored.attempts, 1);
    assert_eq!(
        stored.result.unwrap()["message"],
        "processed: distributed round trip"
    );
    handle.abort();
    Ok(())
}
