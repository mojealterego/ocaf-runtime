#![cfg(feature = "zenoh")]

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{ReconnectPolicy, ResilientZenohRpcClient, ZenohRpcServer};

const KEY: [u8; 32] = [0x56; 32];

/// The resilient client starts before the remote coordinator is listening.
/// It must create a new authenticated Zenoh session and complete the
/// signed request once the coordinator appears on the same stable address.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_recovers_when_coordinator_appears_after_first_connection_attempt(
) -> anyhow::Result<()> {
    let socket = TcpListener::bind("127.0.0.1:0")?;
    let port = socket.local_addr()?.port();
    drop(socket);

    let server_config = zenoh::Config::from_json5(&format!(
        r#"{{mode:"peer",listen:{{endpoints:["tcp/127.0.0.1:{port}"]}},
           scouting:{{multicast:{{enabled:false}},gossip:{{enabled:false}}}},
           transport:{{link:{{protocols:["tcp"]}}}}}}"#
    ))
    .map_err(|err| anyhow::anyhow!("server config: {err}"))?;
    let client_config = zenoh::Config::from_json5(&format!(
        r#"{{mode:"client",connect:{{endpoints:["tcp/127.0.0.1:{port}"]}},
           scouting:{{multicast:{{enabled:false}},gossip:{{enabled:false}}}},
           transport:{{link:{{protocols:["tcp"]}}}}}}"#
    ))
    .map_err(|err| anyhow::anyhow!("client config: {err}"))?;

    let protocol = RpcClient::new("submitter", "coord", KEY)?;
    let policy = ReconnectPolicy::new(
        8,
        Duration::from_millis(100),
        Duration::from_millis(400),
        Duration::from_secs(2),
    )?;
    let client = ResilientZenohRpcClient::new(client_config, protocol, policy);

    let server = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(450)).await;
        let session = Arc::new(
            zenoh::open(server_config)
                .await
                .map_err(|e| anyhow::anyhow!("start coordinator: {e}"))?,
        );
        let peers = HashMap::from([(
            "submitter".into(),
            PeerPolicy::new(KEY, vec!["demo_echo".into()]),
        )]);
        let gateway = Arc::new(RpcGateway::new("coord", OcafRuntime::new(0), peers, 8)?);
        ZenohRpcServer::new(session, gateway).serve().await
    });
    let reply = tokio::time::timeout(Duration::from_secs(14), client.request(RpcAction::Discover))
        .await??;
    assert_eq!(reply.status, RpcStatus::Ok);
    server.abort();
    Ok(())
}
