//! Explicitly opted-in end-to-end RPC demonstration.
//! For a production deployment, load a vetted Zenoh TLS/mTLS configuration.
//! A shared HMAC key authenticates messages but does not encrypt payloads.

use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{ZenohRpcClient, ZenohRpcServer};
use ocaf_runtime::transport::{pricing_tool_manifest, EchoTool, ZenohTransport};
use serde_json::json;

fn require(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)
        .map_err(|_| anyhow::anyhow!("missing required environment variable: {name}"))?;
    if value.is_empty() {
        anyhow::bail!("empty environment variable: {name}");
    }
    Ok(value)
}

fn load_key() -> anyhow::Result<[u8; 32]> {
    let secret = require("OCAF_SHARED_KEY_B64")?;
    let decoded = B64
        .decode(secret.as_bytes())
        .map_err(|_| anyhow::anyhow!("OCAF_SHARED_KEY_B64 is not valid base64"))?;
    let key: [u8; 32] = decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("OCAF_SHARED_KEY_B64 must decode to exactly 32 bytes"))?;
    if key == [0; 32] {
        anyhow::bail!("all-zero shared key is forbidden");
    }
    Ok(key)
}

fn load_zenoh_config() -> anyhow::Result<zenoh::Config> {
    // No implicit network config: operator must explicitly select the
    // reviewed endpoint/transport topology.
    let path = require("OCAF_ZENOH_CONFIG")?;
    zenoh::Config::from_file(path).map_err(|e| anyhow::anyhow!("cannot load Zenoh config: {e}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let role = require("OCAF_ROLE")?;
    let node_id = require("OCAF_NODE_ID")?;
    let peer_id = require("OCAF_PEER_ID")?;
    let key = load_key()?;
    let config = load_zenoh_config()?;
    let transport = ZenohTransport::connect_with_config(config).await?;
    let session = transport.session();

    match role.as_str() {
        "server" => {
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
            tracing::info!(node = %node_id, "Authenticated RPC queryable is active");
            ZenohRpcServer::new(session, gateway).serve().await?;
        }
        "client" => {
            let protocol = RpcClient::new(&node_id, &peer_id, key)?;
            let client = ZenohRpcClient::new(session, protocol);
            let action = match env::var("OCAF_ACTION").as_deref() {
                Ok("discover") => RpcAction::Discover,
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
        other => anyhow::bail!("unknown OCAF_ROLE: {other} (expected server or client)"),
    }
    Ok(())
}
