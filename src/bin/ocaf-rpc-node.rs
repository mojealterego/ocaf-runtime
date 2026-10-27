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
use ocaf_runtime::ops::secrets::read_private_text;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{
    ReconnectPolicy, ResilientZenohRpcClient, ZenohRpcClient, ZenohRpcServer,
};
#[cfg(feature = "tls-bound")]
use ocaf_runtime::transport::tls_bound::{client_config, request as tls_request};
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

fn signed_rpc_client(node_id: &str, remote_id: &str, key: [u8; 32]) -> anyhow::Result<RpcClient> {
    let mut client = RpcClient::new(node_id, remote_id, key)?;
    match env::var("OCAF_RPC_CERT_KEY_FILE") {
        Ok(filename) => {
            let path = Path::new(&filename);
            if !path.is_absolute() {
                anyhow::bail!("OCAF_RPC_CERT_KEY_FILE must be an absolute path");
            }
            let pem = read_private_text(path, 16_384)?;
            client = client.with_certificate_private_key(pem.into_bytes())?;
        }
        Err(env::VarError::NotPresent) => {}
        Err(error) => anyhow::bail!("invalid RPC certificate key file environment: {error}"),
    }
    let names = [
        "OCAF_OMAS_TENANT",
        "OCAF_OMAS_SIGNING_KEY_FILE",
        "OCAF_OMAS_FUEL",
    ];
    let configured = names
        .iter()
        .filter(|name| env::var_os(name).is_some())
        .count();
    if configured != 0 {
        anyhow::ensure!(
            configured == names.len(),
            "incomplete OMAS signer configuration"
        );
        anyhow::ensure!(
            env::var("OCAF_ROLE").as_deref() == Ok("client")
                && env::var("OCAF_RPC_INGRESS").as_deref() == Ok("tls-bound"),
            "OMAS signer requires TLS-bound client mode"
        );
        let path = std::path::PathBuf::from(require("OCAF_OMAS_SIGNING_KEY_FILE")?);
        anyhow::ensure!(path.is_absolute(), "OMAS signer path must be absolute");
        let pem = read_private_text(&path, 16_384)?;
        let fuel = require("OCAF_OMAS_FUEL")?.parse::<u64>()?;
        client = client.with_omas_signer(&require("OCAF_OMAS_TENANT")?, pem.as_bytes(), fuel)?;
    }
    Ok(client)
}

#[cfg(feature = "tls-bound")]
fn tls_client_settings() -> anyhow::Result<(std::net::SocketAddr, String, Arc<rustls::ClientConfig>)>
{
    let cert_path = Path::new(&require("OCAF_TLS_CLIENT_CERT_FILE")?).to_path_buf();
    let key_path = Path::new(&require("OCAF_TLS_CLIENT_KEY_FILE")?).to_path_buf();
    let ca_path = Path::new(&require("OCAF_TLS_SERVER_CA_FILE")?).to_path_buf();
    let cert = read_private_text(&cert_path, 32_768)?;
    let key = read_private_text(&key_path, 16_384)?;
    let ca = read_private_text(&ca_path, 65_536)?;
    let address: std::net::SocketAddr = require("OCAF_TLS_RPC_ADDRESS")?
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid explicit TLS RPC address"))?;
    let server_dns = require("OCAF_TLS_RPC_SERVER_DNS")?;
    let config = client_config(cert.as_bytes(), key.as_bytes(), ca.as_bytes())?;
    Ok((address, server_dns, Arc::new(config)))
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

/// A typo in transport policy must NEVER silently fall back to Zenoh.
fn ingress_mode(setting: Result<String, env::VarError>) -> anyhow::Result<bool> {
    match setting {
        Ok(value) if value == "tls-bound" => Ok(true),
        Ok(value) if value == "zenoh" => Ok(false),
        Err(env::VarError::NotPresent) => Ok(false),
        _ => anyhow::bail!("OCAF_RPC_INGRESS must be exactly zenoh or tls-bound"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let role = require("OCAF_ROLE")?;
    let node_id = require("OCAF_NODE_ID")?;
    let peer_id = require("OCAF_PEER_ID")?;
    let key = load_key_env("OCAF_SHARED_KEY_B64")?;
    let tls_mode = ingress_mode(env::var("OCAF_RPC_INGRESS"))?;
    match role.as_str() {
        "server" => {
            anyhow::ensure!(
                !tls_mode,
                "TLS-bound worker/server mode is not available via demo node; use ocaf-coordinator"
            );
            let config = load_zenoh_config()?;
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
                let protocol = signed_rpc_client(&node_id, &coord_id, coord_key)?;
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
            #[cfg(target_os = "linux")]
            let runtime = ocaf_runtime::ops::wasi_config::runtime_from_env().await?;
            #[cfg(not(target_os = "linux"))]
            let runtime = {
                if env::var("OCAF_WASI_MANIFEST_FILE").is_ok() {
                    anyhow::bail!("signed WASI remote workers require Linux");
                }
                OcafRuntime::new(0)
            };
            runtime.register(pricing_tool_manifest(), EchoTool).await?;
            let protocol = signed_rpc_client(&node_id, &peer_id, key)?;
            let pure_list = env::var("OCAF_CLUSTER_PURE_TOOLS").unwrap_or_default();
            let pure_tools: HashSet<String> = if pure_list.is_empty() {
                HashSet::new()
            } else {
                pure_list.split(',').map(str::to_owned).collect()
            };
            let mut allowed = HashSet::from(["demo_echo".into()]);
            for tool in runtime.registry.list().await {
                if tool.name != "demo_echo" {
                    // Exactly the operator-signed tool registered during
                    // runtime_from_env; no untrusted capability discovery.
                    allowed.insert(tool.name);
                }
            }
            let worker = if tls_mode {
                #[cfg(feature = "tls-bound")]
                {
                    let (address, server_dns, config) = tls_client_settings()?;
                    RemoteJobWorker::new_tls_bound(
                        protocol, address, server_dns, config, runtime, allowed,
                    )?
                }
                #[cfg(not(feature = "tls-bound"))]
                {
                    anyhow::bail!("TLS-bound worker requires compile-time tls-bound feature");
                }
            } else {
                let config = load_zenoh_config()?;
                let session = ZenohTransport::connect_with_config(config).await?.session();
                let client = ZenohRpcClient::new(session, protocol);
                RemoteJobWorker::new(client, runtime, allowed)?
            }
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
            let protocol = signed_rpc_client(&node_id, &peer_id, key)?;
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
                    tool: env::var("OCAF_TOOL").unwrap_or_else(|_| "demo_echo".into()),
                    args: json!({"message": env::var("OCAF_MESSAGE").unwrap_or_else(|_| "hello over Zenoh RPC".into())}),
                },
                Ok(other) => anyhow::bail!("unknown OCAF_ACTION: {other}"),
            };
            let reply = if tls_mode {
                #[cfg(feature = "tls-bound")]
                {
                    let (address, server_dns, config) = tls_client_settings()?;
                    tls_request(address, &server_dns, config, &protocol, action).await?
                }
                #[cfg(not(feature = "tls-bound"))]
                {
                    anyhow::bail!("TLS-bound ingress requires compile-time feature tls-bound");
                }
            } else {
                let config = load_zenoh_config()?;
                let client = ResilientZenohRpcClient::new(config, protocol, reconnect);
                client.request(action).await?
            };
            println!("{}", serde_json::to_string_pretty(&reply)?);
            if reply.status != RpcStatus::Ok {
                anyhow::bail!("remote tool call failed: {:?}", reply.status);
            }
        }
        other => anyhow::bail!("unknown OCAF_ROLE: {other} (expected server, worker or client)"),
    }
    Ok(())
}

#[cfg(test)]
mod ingress_tests {
    use super::*;

    #[test]
    fn rejects_malformed_transport_selection_without_legacy_fallback() {
        assert!(ingress_mode(Ok("tls-bound".into())).unwrap());
        assert!(!ingress_mode(Ok("zenoh".into())).unwrap());
        assert!(!ingress_mode(Err(env::VarError::NotPresent)).unwrap());
        for value in ["tls-boud", "none", "", "TLS-BOUND"] {
            assert!(ingress_mode(Ok(value.to_owned())).is_err());
        }
    }
}
