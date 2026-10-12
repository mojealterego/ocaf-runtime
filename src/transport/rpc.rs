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
                // Drop a completed reply if the coordinator lost its
                // PostgreSQL leader session during request execution.
                // This fences replies, not side effects from in-flight tools.
                let result = match result {
                    Ok(reply_bytes) => gateway.verify_current_leader().await.map(|()| reply_bytes),
                    Err(error) => Err(error),
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

/// Operators may automatically repeat only reads or submission with a
/// caller-supplied, stable UUID. In particular, Execute, ClaimJob,
/// FinishJob and Heartbeat cannot safely be reissued after a lost reply.
pub fn can_retry_rpc_action(action: &RpcAction) -> bool {
    matches!(
        action,
        RpcAction::Discover
            | RpcAction::Resolve { .. }
            | RpcAction::GetJob { .. }
            | RpcAction::GetJobAudit { .. }
            | RpcAction::SubmitJob { .. }
    )
}

/// Strictly bounded reconnect attempts. No retry loop is infinite and each
/// attempt (opening a new Zenoh session AND RPC exchange) has a timeout.
#[derive(Debug, Clone, Copy)]
pub struct ReconnectPolicy {
    attempts: u8,
    initial_delay: Duration,
    max_delay: Duration,
    attempt_timeout: Duration,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            attempts: 4,
            initial_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(1),
            attempt_timeout: Duration::from_secs(9),
        }
    }
}

impl ReconnectPolicy {
    pub fn new(
        attempts: u8,
        initial_delay: Duration,
        max_delay: Duration,
        attempt_timeout: Duration,
    ) -> anyhow::Result<Self> {
        if !(1..=8).contains(&attempts)
            || initial_delay.is_zero()
            || initial_delay > max_delay
            || max_delay > Duration::from_secs(5)
            || attempt_timeout.is_zero()
            || attempt_timeout > Duration::from_secs(30)
        {
            anyhow::bail!("invalid bounded RPC reconnection policy");
        }
        Ok(Self {
            attempts,
            initial_delay,
            max_delay,
            attempt_timeout,
        })
    }

    /// The operation is invoked with a freshly constructed signed request on
    /// each attempt. SubmitJob MUST retain the original job UUID and payload.
    /// Returned signed RPC replies (including application errors) are never
    /// retried: only failure to obtain a verified reply is retried.
    pub async fn run<F, Fut, T>(&self, action: &RpcAction, mut operation: F) -> anyhow::Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<T>>,
    {
        let allowed_attempts = if can_retry_rpc_action(action) {
            self.attempts
        } else {
            1
        };
        let mut delay = self.initial_delay;
        for index in 0..allowed_attempts {
            let result = tokio::time::timeout(self.attempt_timeout, operation()).await;
            match result {
                Ok(Ok(value)) => return Ok(value),
                Ok(Err(err)) if index + 1 == allowed_attempts => return Err(err),
                Err(_) if index + 1 == allowed_attempts => {
                    anyhow::bail!("RPC reconnection attempt timed out");
                }
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = delay.saturating_mul(2).min(self.max_delay);
                }
            }
        }
        unreachable!("at least one RPC attempt is required")
    }
}

/// Recreates the entire Zenoh transport/session for each bounded attempt,
/// allowing clients to connect after an active/passive coordinator switch.
/// The caller MUST have validated this configuration with the CLI security
/// preflight; HMAC alone does not encrypt network traffic.
pub struct ResilientZenohRpcClient {
    config: zenoh::Config,
    protocol: RpcClient,
    policy: ReconnectPolicy,
}

impl ResilientZenohRpcClient {
    pub fn new(config: zenoh::Config, protocol: RpcClient, policy: ReconnectPolicy) -> Self {
        Self {
            config,
            protocol,
            policy,
        }
    }

    pub async fn request(&self, action: RpcAction) -> anyhow::Result<RpcReply> {
        self.policy
            .run(&action, || {
                let config = self.config.clone();
                let action = action.clone();
                async move {
                    let session = zenoh::open(config)
                        .await
                        .map_err(|e| error("Zenoh reconnect", e))?;
                    request_over_session(&session, &self.protocol, action).await
                }
            })
            .await
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

    pub fn local_id(&self) -> &str {
        self.protocol.local_id()
    }

    pub async fn request(&self, action: RpcAction) -> anyhow::Result<RpcReply> {
        request_over_session(&self.session, &self.protocol, action).await
    }
}

async fn request_over_session(
    session: &zenoh::Session,
    protocol: &RpcClient,
    action: RpcAction,
) -> anyhow::Result<RpcReply> {
    let (request, body) = protocol
        .prepare(action, 5_000)
        .map_err(|e| error("prepare signed RPC", e))?;
    let key = rpc_key(protocol.remote_id());
    let replies = session
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
    protocol
        .read_reply(payload.to_bytes().as_ref(), &request)
        .map_err(|e| error("verify signed response", e))
}
