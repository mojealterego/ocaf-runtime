//! Pull-based, authenticated remote job executor.
//!
//! The worker receives a 30-second lease from the coordinator, performs only
//! explicitly allowlisted, free and non-privileged local tools, and reports
//! the result with a lease fencing token. The coordinator owns persistence.
//! This is not a distributed exactly-once side-effect protocol.

use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use serde_json::from_value;

use crate::coordinator::validate_tool_name;
use crate::rpc::{RemoteJobOutcome, RpcAction, RpcStatus};
use crate::runtime::{OcafRuntime, RuntimeError, ToolCallRequest};
use crate::transport::rpc::ZenohRpcClient;

use super::{JobStatus, LeasedJob};

const REMOTE_LEASE_MS: u64 = 30_000;
const TIMEOUT_MARGIN_MS: u64 = 4_000;

pub struct RemoteJobWorker {
    client: ZenohRpcClient,
    runtime: OcafRuntime,
    allowed_tools: HashSet<String>,
    /// Only an operator-vetted, side-effect-free tool may run under a
    /// positive leadership generation without a sink-level adapter.
    cluster_pure_tools: HashSet<String>,
    node_id: String,
}

impl RemoteJobWorker {
    pub fn new(
        client: ZenohRpcClient,
        runtime: OcafRuntime,
        allowed_tools: HashSet<String>,
    ) -> Result<Self> {
        if allowed_tools.is_empty()
            || allowed_tools.len() > 64
            || allowed_tools.iter().any(|tool| !validate_tool_name(tool))
        {
            bail!("invalid remote worker tool allowlist");
        }
        let node_id = client.local_id().to_owned();
        Ok(Self {
            client,
            runtime,
            allowed_tools,
            cluster_pure_tools: HashSet::new(),
            node_id,
        })
    }

    /// Explicitly opt in proven side-effect-free tools for clustered
    /// execution. The worker refuses arbitrary tools under positive
    /// epochs even after passing point-in-time ValidateLease.
    pub fn with_cluster_pure_tools(mut self, tools: HashSet<String>) -> Result<Self> {
        if tools.len() > 64
            || tools
                .iter()
                .any(|tool| !validate_tool_name(tool) || !self.allowed_tools.contains(tool))
        {
            bail!("cluster pure-tool policy must be a subset of the worker allowlist");
        }
        self.cluster_pure_tools = tools;
        Ok(self)
    }

    /// Publish the worker's local capability list. Coordinator ACLs must
    /// independently allow every advertised tool for this signed peer.
    pub async fn heartbeat(&self) -> Result<()> {
        let mut tools: Vec<_> = self.allowed_tools.iter().cloned().collect();
        tools.sort();
        let response = self.client.request(RpcAction::Heartbeat { tools }).await?;
        if response.status != RpcStatus::Ok {
            bail!("coordinator heartbeat rejected");
        }
        Ok(())
    }

    /// Poll at most one job over Zenoh and report its fenced result. Returning
    /// None means no eligible pending task is available on the coordinator.
    pub async fn poll_once(&self) -> Result<Option<JobStatus>> {
        let reply = self.client.request(RpcAction::ClaimJob).await?;
        if reply.status != RpcStatus::Ok {
            bail!("coordinator refused job claim");
        }
        let lease: Option<LeasedJob> =
            from_value(reply.output).map_err(|_| anyhow!("malformed coordinator job lease"))?;
        let Some(lease) = lease else {
            return Ok(None);
        };
        if lease.worker_id != self.node_id
            || !self.allowed_tools.contains(&lease.tool)
            || lease.lease_token.is_empty()
            || lease.leader_epoch < 0
        {
            bail!("coordinator returned unauthorised job lease");
        }
        // A signed claim must be checked against the current leader epoch
        // immediately before execution. A transport error fails closed.
        if lease.leader_epoch > 0 {
            let verified = self
                .client
                .request(RpcAction::ValidateLease {
                    lease: lease.clone(),
                })
                .await?;
            if verified.status != RpcStatus::Ok || verified.output["valid"] != true {
                bail!("coordinator lease was fenced before execution");
            }
        }
        let maybe_tool = self.runtime.registry.resolve(&lease.tool).await;
        let runnable = maybe_tool.as_ref().is_some_and(|tool| {
            // In cluster mode, a pre-execution lease check alone cannot
            // fence external effects after a coordinator failover. Unsafe
            // tools are rejected before the runtime invokes their executor.
            (lease.leader_epoch == 0 || self.cluster_pure_tools.contains(&lease.tool))
                && tool.manifest.enabled
                && tool.manifest.cost_sats == 0
                && tool.manifest.permissions.is_empty()
                && tool
                    .manifest
                    .timeout_ms
                    .checked_add(TIMEOUT_MARGIN_MS)
                    .is_some_and(|limit| limit < REMOTE_LEASE_MS)
        });

        let outcome = if !runnable {
            RemoteJobOutcome::Failed
        } else {
            let request = ToolCallRequest {
                call_id: lease.id.clone(),
                tool: lease.tool.clone(),
                args: lease.args.clone(),
                auth: None,
            };
            match self.runtime.execute(request).await {
                Ok(result) => RemoteJobOutcome::Succeeded {
                    output: result.output,
                },
                Err(
                    RuntimeError::InvalidInput(_)
                    | RuntimeError::ToolNotFound(_)
                    | RuntimeError::ToolDisabled(_)
                    | RuntimeError::AuthRequired(_)
                    | RuntimeError::BudgetExceeded,
                ) => RemoteJobOutcome::Failed,
                Err(_) => RemoteJobOutcome::Uncertain,
            }
        };
        let status = match &outcome {
            RemoteJobOutcome::Succeeded { .. } => JobStatus::Succeeded,
            RemoteJobOutcome::Failed => JobStatus::Failed,
            RemoteJobOutcome::Uncertain => JobStatus::Uncertain,
        };
        let report = self
            .client
            .request(RpcAction::FinishJob { lease, outcome })
            .await?;
        if report.status != RpcStatus::Ok {
            bail!("coordinator rejected fenced job completion");
        }
        Ok(Some(status))
    }
}
