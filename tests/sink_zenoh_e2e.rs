#![cfg(feature = "zenoh")]
//! Real Zenoh separate-session receiver exercise with PostgreSQL-backed
//! leader succession and durable SQLite receiver state.
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::sink::{EffectSink, SinkAction, SinkKeys, SinkStatus};
use ocaf_runtime::transport::sink::{ZenohSinkClient, ZenohSinkServer};
use serde_json::json;

const CONTROLLER: [u8; 32] = [0x53; 32];
const WORKER: [u8; 32] = [0x54; 32];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn separate_zenoh_receiver_fences_postgres_leader_takeover() -> anyhow::Result<()> {
    let pg = match std::env::var("OCAF_TEST_PG_DSN") {
        Ok(v) => v,
        Err(_) if std::env::var("OCAF_REQUIRE_PG_TESTS").is_err() => return Ok(()),
        Err(_) => anyhow::bail!("PostgreSQL required for Zenoh receiver CI test"),
    };
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let peer_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"peer",listen:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    ))
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let cli_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"client",connect:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    ))
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let server_session = Arc::new(zenoh::open(peer_cfg).await?);
    let controller_session = Arc::new(zenoh::open(cli_cfg.clone()).await?);
    let worker_session = Arc::new(zenoh::open(cli_cfg).await?);
    let dir = tempfile::tempdir()?;
    let sink = Arc::new(EffectSink::open(
        &dir.path().join("effect_sink.sqlite"),
        "standalone-sink",
        SinkKeys::new("leader", CONTROLLER, "worker", WORKER)?,
    )?);
    let server = ZenohSinkServer::new(server_session, sink);
    let serving = tokio::spawn(async move { server.serve().await });
    tokio::time::sleep(Duration::from_millis(350)).await;

    let controller = ZenohSinkClient::new(
        controller_session, "standalone-sink", "leader", CONTROLLER,
    )?;
    let worker = ZenohSinkClient::new(
        worker_session, "standalone-sink", "worker", WORKER,
    )?;
    let cluster = format!("sink_remote_{}", uuid::Uuid::new_v4().simple());
    let previous = PgLeader::connect(&pg, &cluster, true).await?;
    let next = PgLeader::connect(&pg, &cluster, true).await?;
    assert!(previous.try_acquire().await?);
    let activation = controller.activate_from_leader(&previous, 10_000).await?;
    assert_eq!(activation.status, SinkStatus::Ok);
    let id = uuid::Uuid::new_v4().to_string();
    let effect = json!({"remote_event":"committed"});
    let token = controller
        .issue_permit(
            &previous, "worker", &id,
            ocaf_runtime::rpc::now_ms()? + 10_000, &effect,
        )
        .await?;
    let committed = worker.request(SinkAction::Apply {
        permit: token.clone(), effect: effect.clone(),
    }).await?;
    assert_eq!(committed.status, SinkStatus::Ok);
    assert_eq!(committed.output["applied"], true);
    let repeat = worker.request(SinkAction::Apply {
        permit: token.clone(), effect: effect.clone(),
    }).await?;
    assert_eq!(repeat.status, SinkStatus::Ok);
    assert_eq!(repeat.output["applied"], false);

    previous.release().await?;
    assert!(next.try_acquire().await?);
    assert_eq!(
        controller.activate_from_leader(&next, 10_000).await?.status,
        SinkStatus::Ok
    );
    let stale = worker.request(SinkAction::Apply {
        permit: token, effect,
    }).await?;
    assert_eq!(stale.status, SinkStatus::Denied);
    next.release().await?;
    serving.abort();
    Ok(())
}
