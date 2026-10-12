//! Explicitly opted-in end-to-end RPC demonstration.
//! For a production deployment, load a vetted Zenoh TLS/mTLS configuration.
//! A shared HMAC key authenticates messages but does not encrypt payloads.

use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::jobs::remote::RemoteJobWorker;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{
    ReconnectPolicy, ResilientZenohRpcClient, ZenohRpcClient, ZenohRpcServer,
};
use ocaf_runtime::transport::{pricing_tool_manifest, EchoTool, ZenohTransport};
use ocaf_runtime::transport_policy::{read_zenoh_config, validate_zenoh_config};
use serde_json::json;

fn require(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)
        .map_err(|_| anyhow::anyhow!("missing required environment variable: {name}"))?;
    if value.is_empty() {
        anyhow::bail!("empty environment variable: {name}");
    }
    Ok(value)
}

fn load_key_env(name: &str) -> anyhow::Result<[u8; 32]> {
    let secret = require(name)?;
    let decoded = B64
        .decode(secret.as_bytes())
        .map_err(|_| anyhow::anyhow!("{name} is not valid base64"))?;
    let key: [u8; 32] = decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("{name} must decode to exactly 32 bytes"))?;
    if key == [0; 32] {
        anyhow::bail!("all-zero shared key is forbidden");
    }
    Ok(key)
}

fn load_zenoh_config() -> anyhow::Result<zenoh::Config> {
    // No implicit network config: operator must explicitly select the
    // reviewed endpoint/transport topology.
    let path = require("OCAF_ZENOH_CONFIG")?;
    let source = read_zenoh_config(Path::new(&path))?;
    validate_zenoh_config(
        &source,
        env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1"),
    )?;
    zenoh::Config::from_json5(&source)
        .map_err(|e| anyhow::anyhow!("cannot parse vetted Zenoh config: {e}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let role = require("OCAF_ROLE")?;
    let node_id = require("OCAF_NODE_ID")?;
    let peer_id = require("OCAF_PEER_ID")?;
    let key = load_key_env("OCAF_SHARED_KEY_B64")?;
    let config = load_zenoh_config()?;

    match role.as_str() {
        "server" => {
            let session = ZenohTransport::connect_with_config(config).await?.session();
            let runtime = OcafRuntime::new(0);
            runtime.register(pricing_tool_manifest(), EchoTool).await?;
            // The peer may invoke ONLY the demo echo tool, not arbitrary local tools.
            let peers =
                HashMap::from([(peer_id, PeerPolicy::new(key, vec!["demo_echo".to_string()]))]);
            let db_path = require("OCAF_REPLAY_DB_PATH")?;
            if !Path::new(&db_path).is_absolute() {
                anyhow::bail!("OCAF_REPLAY_DB_PATH must be an absolute path");
            }
            let gateway = Arc::new(RpcGateway::with_sqlite_replay(
                &node_id,
                runtime,
                peers,
                16,
                Path::new(&db_path),
            )?);
            let coordinator_id = env::var("OCAF_COORDINATOR_NODE_ID").ok();
            let coordinator_key = env::var("OCAF_COORDINATOR_KEY_B64").ok();
            if coordinator_id.is_some() != coordinator_key.is_some() {
                anyhow::bail!(
                    "OCAF_COORDINATOR_NODE_ID and OCAF_COORDINATOR_KEY_B64 must be set together"
                );
            }
            let heartbeat_task = if let Some(coord_id) = coordinator_id {
                let coord_key = load_key_env("OCAF_COORDINATOR_KEY_B64")?;
                let protocol = RpcClient::new(&node_id, &coord_id, coord_key)?;
                let reporter = ZenohRpcClient::new(Arc::clone(&session), protocol);
                Some(tokio::spawn(async move {
                    // Avoid advertising until the local queryable has started.
                    tokio::time::sleep(Duration::from_millis(750)).await;
                    loop {
                        if let Err(error) = reporter
                            .request(RpcAction::Heartbeat {
                                tools: vec!["demo_echo".into()],
                            })
                            .await
                        {
                            tracing::warn!(error = %error, "coordinator heartbeat failed");
                        }
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }))
            } else {
                None
            };
            tracing::info!(node = %node_id, "Authenticated RPC queryable is active");
            let served = ZenohRpcServer::new(session, gateway).serve().await;
            if let Some(task) = heartbeat_task {
                task.abort();
            }
            served?;
        }
        "worker" => {
            let session = ZenohTransport::connect_with_config(config).await?.session();
            let runtime = OcafRuntime::new(0);
            runtime.register(pricing_tool_manifest(), EchoTool).await?;
            let protocol = RpcClient::new(&node_id, &peer_id, key)?;
            let client = ZenohRpcClient::new(session, protocol);
            let pure_list = env::var("OCAF_CLUSTER_PURE_TOOLS").unwrap_or_default();
            let pure_tools: HashSet<String> = if pure_list.is_empty() {
                HashSet::new()
            } else {
                pure_list.split(',').map(str::to_owned).collect()
            };
            let worker = RemoteJobWorker::new(
                client,
                runtime,
                HashSet::from(["demo_echo".into()]),
            )?
            .with_cluster_pure_tools(pure_tools)?;
            let mut heartbeat = tokio::time::interval(Duration::from_secs(5));
            let mut polling = tokio::time::interval(Duration::from_millis(500));
            loop {
                tokio::select! {
                    _ = heartbeat.tick() => {
                        if let Err(error) = worker.heartbeat().await {
                            tracing::warn!(error = %error, "remote worker heartbeat rejected");
                        }
                    }
                    _ = polling.tick() => {
                        if let Err(error) = worker.poll_once().await {
                            tracing::warn!(error = %error, "remote job claim/execution failed");
                        }
                    }
                }
            }
        }
        "client" => {
            let protocol = RpcClient::new(&node_id, &peer_id, key)?;
            let attempts = env::var("OCAF_RECONNECT_ATTEMPTS")
                .unwrap_or_else(|_| "4".to_string())
                .parse::<u8>()
                .map_err(|_| anyhow::anyhow!("OCAF_RECONNECT_ATTEMPTS must be 1 through 8"))?;
            let reconnect = ReconnectPolicy::new(
                attempts,
                Duration::from_millis(250),
                Duration::from_secs(1),
                Duration::from_secs(9),
            )?;
            let client = ResilientZenohRpcClient::new(config, protocol, reconnect);
            let action = match env::var("OCAF_ACTION").as_deref() {
                Ok("discover") => RpcAction::Discover,
                Ok("heartbeat") => RpcAction::Heartbeat {
                    tools: vec![env::var("OCAF_TOOL").unwrap_or_else(|_| "demo_echo".into())],
                },
                Ok("resolve") => RpcAction::Resolve {
                    tool: env::var("OCAF_TOOL").unwrap_or_else(|_| "demo_echo".into()),
                },
                Ok("status") => RpcAction::GetJob {
                    id: require("OCAF_JOB_ID")?,
                },
                Ok("audit") => RpcAction::GetJobAudit {
                    id: require("OCAF_JOB_ID")?,
                    limit: env::var("OCAF_AUDIT_LIMIT")
                        .unwrap_or_else(|_| "50".into())
                        .parse::<usize>()
                        .map_err(|_| {
                            anyhow::anyhow!("OCAF_AUDIT_LIMIT must be an integer between 1 and 100")
                        })?,
                },
                Ok("submit") => RpcAction::SubmitJob {
                    // An implicit new UUID on each CLI restart can execute
                    // the same external action twice after an ambiguous
                    // network outage. Require stable caller-supplied UUID.
                    id: require("OCAF_JOB_ID")?,
                    tool: "demo_echo".into(),
                    args: json!({
                        "message": env::var("OCAF_MESSAGE")
                            .unwrap_or_else(|_| "durable distributed job".into())
                    }),
                },
                Ok("call") | Err(_) => RpcAction::Execute {
                    tool: "demo_echo".into(),
                    args: json!({"message": env::var("OCAF_MESSAGE").unwrap_or_else(|_| "hello over Zenoh RPC".into())}),
                },
                Ok(other) => anyhow::bail!("unknown OCAF_ACTION: {other}"),
            };
            let reply = client.request(action).await?;
            println!("{}", serde_json::to_string_pretty(&reply)?);
            if reply.status != RpcStatus::Ok {
                anyhow::bail!("remote tool call failed: {:?}", reply.status);
            }
        }
        other => anyhow::bail!("unknown OCAF_ROLE: {other} (expected server, worker or client)"),
    }
    Ok(())
}
