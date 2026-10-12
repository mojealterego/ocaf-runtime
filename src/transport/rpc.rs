//! Zenoh v1.10 query/reply adapter for authenticated OCAF RPC.
//!
//! Uses the RPC gateway for HMAC authentication and ACL checks, *not* Zenoh's
//! transport identity. Traffic is not encrypted unless the caller configures
//! Zenoh TLS externally.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::rpc::{RpcAction, RpcClient, RpcError, RpcGateway, RpcReply, MAX_FRAME_BYTES};

const RPC_TIMEOUT: Duration = Duration::from_secs(6);

fn error(context: &str, err: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!("{context}: {err}")
}

pub fn rpc_key(node_id: &str) -> String {
    format!("ocaf/v1/nodes/{node_id}/rpc")
}

pub struct ZenohRpcServer {
    session: Arc<zenoh::Session>,
    gateway: Arc<RpcGateway>,
}

impl ZenohRpcServer {
    pub fn new(session: Arc<zenoh::Session>, gateway: Arc<RpcGateway>) -> Self {
        Self { session, gateway }
    }

    /// Accept queries on exactly one node-specific key. Every request is
    /// authenticated by the gateway before it can invoke a registered tool.
    /// Dropping the future undeclares the queryable; spawned in-flight
    /// executions are not durable and may finish after shutdown.
    pub async fn serve(&self) -> anyhow::Result<()> {
        let key = rpc_key(self.gateway.node_id());
        let ingress = Arc::new(Semaphore::new(64));
        let queryable = self
            .session
            .declare_queryable(key.as_str())
            .await
            .map_err(|err| error("declare queryable", err))?;
        while let Ok(query) = queryable.recv_async().await {
            let permit = match Arc::clone(&ingress).try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    let _ = query.reply_err("rejected").await;
                    continue;
                }
            };
            let gateway = Arc::clone(&self.gateway);
            let reply_key = key.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let result = match query.payload() {
                    Some(body) if body.len() <= MAX_FRAME_BYTES => {
                        gateway.handle(body.to_bytes().as_ref()).await
                    }
                    Some(_) => Err(RpcError::FrameTooLarge),
                    None => Err(RpcError::Malformed),
                };
                match result {
                    Ok(reply_bytes) => {
                        if let Err(err) = query.reply(reply_key, reply_bytes).await {
                            tracing::warn!(error = %err, "Zenoh RPC reply failed");
                        }
                    }
                    Err(rejected) => {
                        tracing::warn!(reason = %rejected, "Zenoh RPC request rejected");
                        // All unauthenticated requests get an indistinguishable
                        // wire error; avoid leaking policy and existence of tools.
                        let _ = query.reply_err("rejected").await;
                    }
                }
            });
        }
        Ok(())
    }
}

pub struct ZenohRpcClient {
    session: Arc<zenoh::Session>,
    protocol: RpcClient,
}

impl ZenohRpcClient {
    pub fn new(session: Arc<zenoh::Session>, protocol: RpcClient) -> Self {
        Self { session, protocol }
    }

    pub async fn request(&self, action: RpcAction) -> anyhow::Result<RpcReply> {
        let (request, body) = self
            .protocol
            .prepare(action, 5_000)
            .map_err(|e| error("prepare signed RPC", e))?;
        let key = rpc_key(self.protocol.remote_id());
        let replies = self
            .session
            .get(key.as_str())
            .payload(body)
            .timeout(RPC_TIMEOUT)
            .await
            .map_err(|e| error("Zenoh get", e))?;
        let reply = tokio::time::timeout(RPC_TIMEOUT, replies.recv_async())
            .await
            .map_err(|e| error("Zenoh RPC timed out", e))?
            .map_err(|e| error("Zenoh RPC reply unavailable", e))?;

        let sample = reply
            .result()
            .map_err(|_| anyhow::anyhow!("remote RPC was rejected"))?;
        let payload = sample.payload();
        if payload.len() > MAX_FRAME_BYTES {
            anyhow::bail!("remote reply exceeds RPC frame size");
        }
        self.protocol
            .read_reply(payload.to_bytes().as_ref(), &request)
            .map_err(|e| error("verify signed response", e))
    }
}
