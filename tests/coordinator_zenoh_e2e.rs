#![cfg(feature = "zenoh")]
//! A coordinator, worker and requester use separate Zenoh TCP sessions.
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{ZenohRpcClient, ZenohRpcServer};
use ocaf_runtime::transport::{pricing_tool_manifest, EchoTool};
use serde_json::json;

const WORKER_KEY: [u8; 32] = [0x23; 32];
const CLIENT_KEY: [u8; 32] = [0x34; 32];
const EXECUTION_KEY: [u8; 32] = [0x56; 32];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signed_remote_agent_registers_and_client_discovers_it_over_real_tcp() -> anyhow::Result<()>
{
    let dir = tempfile::tempdir()?;
    let sock = TcpListener::bind("127.0.0.1:0")?;
    let port = sock.local_addr()?.port();
    drop(sock);
    let server_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"peer",listen:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    )).map_err(|e| anyhow::anyhow!("server config: {e}"))?;
    let client_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"client",connect:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    )).map_err(|e| anyhow::anyhow!("client config: {e}"))?;
    let requester_cfg = client_cfg.clone();

    let coordinator = Arc::new(
        zenoh::open(server_cfg)
            .await
            .map_err(|e| anyhow::anyhow!("coordinator open: {e}"))?,
    );
    let worker = Arc::new(
        zenoh::open(client_cfg)
            .await
            .map_err(|e| anyhow::anyhow!("worker open: {e}"))?,
    );
    let requester = Arc::new(
        zenoh::open(requester_cfg)
            .await
            .map_err(|e| anyhow::anyhow!("requester open: {e}"))?,
    );
    let policy = HashMap::from([
        (
            "worker-a".into(),
            PeerPolicy::new(WORKER_KEY, vec!["demo_echo".into()]),
        ),
        (
            "client-c".into(),
            PeerPolicy::new(CLIENT_KEY, vec!["demo_echo".into()]),
        ),
    ]);
    let gateway = RpcGateway::with_sqlite_replay(
        "coord",
        OcafRuntime::new(0),
        policy,
        16,
        &dir.path().join("rpc.sqlite"),
    )?
    .with_coordinator_db(&dir.path().join("membership.sqlite"))?;

    let server = ZenohRpcServer::new(coordinator, Arc::new(gateway));
    let task = tokio::spawn(async move { server.serve().await });

    let runtime = OcafRuntime::new(0);
    runtime.register(pricing_tool_manifest(), EchoTool).await?;
    let worker_acl = HashMap::from([(
        "client-c".into(),
        PeerPolicy::new(EXECUTION_KEY, vec!["demo_echo".into()]),
    )]);
    let worker_gateway = RpcGateway::new("worker-a", runtime, worker_acl, 8)?;
    let worker_server = ZenohRpcServer::new(Arc::clone(&worker), Arc::new(worker_gateway));
    let worker_task = tokio::spawn(async move { worker_server.serve().await });
    tokio::time::sleep(Duration::from_millis(400)).await;

    let agent = ZenohRpcClient::new(worker, RpcClient::new("worker-a", "coord", WORKER_KEY)?);
    let client = ZenohRpcClient::new(Arc::clone(&requester), RpcClient::new("client-c", "coord", CLIENT_KEY)?);

    let ping = agent
        .request(RpcAction::Heartbeat {
            tools: vec!["demo_echo".into()],
        })
        .await?;
    assert_eq!(ping.status, RpcStatus::Ok);
    assert_eq!(ping.output["node_id"], "worker-a");

    let routed = client
        .request(RpcAction::Resolve {
            tool: "demo_echo".into(),
        })
        .await?;
    assert_eq!(routed.status, RpcStatus::Ok);
    assert_eq!(routed.output["node_id"], "worker-a");

    // The coordinator never transports a peer key or executes the task.
    // The requester separately authenticates to the selected worker.
    let execution = ZenohRpcClient::new(
        Arc::clone(&requester),
        RpcClient::new("client-c", "worker-a", EXECUTION_KEY)?,
    );
    let result = execution
        .request(RpcAction::Execute {
            tool: "demo_echo".into(),
            args: json!({"message": "coordinated RPC"}),
        })
        .await?;
    assert_eq!(result.status, RpcStatus::Ok);
    assert_eq!(result.output["message"], "processed: coordinated RPC");
    worker_task.abort();
    task.abort();
    Ok(())
}
