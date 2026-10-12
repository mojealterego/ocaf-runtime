#![cfg(feature = "zenoh")]

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{ZenohRpcClient, ZenohRpcServer};
use serde_json::{json, Value};

const KEY: [u8; 32] = [42u8; 32];

struct Echo;
#[async_trait::async_trait]
impl ToolExecutor for Echo {
    async fn execute(&self, args: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        Ok(json!({"echo":args["message"]}))
    }
}

fn manifest() -> ToolManifest {
    ToolManifest {
        name: "echo".into(),
        description: "round-trip integration echo".into(),
        input_schema: json!({"type":"object","required":["message"],
            "properties":{"message":{"type":"string"}}}),
        output_schema: json!({"type":"object","required":["echo"],
            "properties":{"echo":{"type":"string"}}}),
        cost_sats: 0,
        permissions: vec![],
        timeout_ms: 1000,
        enabled: true,
    }
}

/// Exercises two independent Zenoh sessions via real TCP on a loopback port.
/// This proves actual Zenoh query/reply and signed RPC, not TLS or cross-host
/// production readiness.
#[tokio::test]
async fn two_zenoh_sessions_exchange_authenticated_rpc_over_tcp() -> anyhow::Result<()> {
    let socket = TcpListener::bind("127.0.0.1:0")?;
    let port = socket.local_addr()?.port();
    drop(socket);

    let server_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"peer",listen:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    )).map_err(|e| anyhow::anyhow!("server config: {e}"))?;

    let client_cfg = zenoh::Config::from_json5(&format!(
        r#"{{mode:"client",connect:{{endpoints:["tcp/127.0.0.1:{port}"]}},scouting:{{multicast:{{enabled:false}}}}}}"#
    )).map_err(|e| anyhow::anyhow!("client config: {e}"))?;

    let server_session = Arc::new(
        zenoh::open(server_cfg).await.map_err(|e| anyhow::anyhow!("open server: {e}"))?,
    );
    let client_session = Arc::new(
        zenoh::open(client_cfg).await.map_err(|e| anyhow::anyhow!("open client: {e}"))?,
    );

    let runtime = OcafRuntime::new(0);
    runtime.register(manifest(), Echo).await?;
    let peers = HashMap::from([(
        "client-node".into(),
        PeerPolicy::new(KEY, vec!["echo".into()]),
    )]);
    let gateway = Arc::new(RpcGateway::new("server-node", runtime, peers, 8)?);
    let server = ZenohRpcServer::new(server_session, gateway);
    let task = tokio::spawn(async move { server.serve().await });

    tokio::time::sleep(Duration::from_millis(400)).await;
    let client = ZenohRpcClient::new(
        client_session,
        RpcClient::new("client-node", "server-node", KEY)?,
    );

    let discovered = client.request(RpcAction::Discover).await?;
    assert_eq!(discovered.status, RpcStatus::Ok);
    assert_eq!(discovered.output[0]["name"], "echo");

    let executed = client.request(RpcAction::Execute {
        tool: "echo".into(),
        args: json!({"message":"end-to-end"}),
    }).await?;
    assert_eq!(executed.status, RpcStatus::Ok);
    assert_eq!(executed.output, json!({"echo":"end-to-end"}));

    task.abort();
    Ok(())
}
