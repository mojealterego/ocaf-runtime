//! Trusted local job worker. Jobs execute through the normal runtime
//! authorization and JSON Schema validation path, not direct ToolExecutor.
//!
//! A lease must outlive the manifest's execution timeout by a safety margin.
//! Stale lease holders are prevented from committing *journal* results; tool
//! side effects must still be explicitly idempotent if retries are enabled.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::runtime::{OcafRuntime, ToolCallRequest};

use super::{validate_worker, JobError, JobStatus, JobStore, MAX_LEASE_MS, MIN_LEASE_MS};

const LEASE_MARGIN_MS: u64 = 1_000;

pub struct JobWorker {
    store: JobStore,
    runtime: OcafRuntime,
    worker_id: String,
    lease_ms: u64,
    allowed_tools: HashSet<String>,
}

fn wallclock_ms() -> Result<u64, JobError> {
    let delta = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| JobError::Storage)?;
    u64::try_from(delta.as_millis()).map_err(|_| JobError::Storage)
}

impl JobWorker {
    pub fn new(
        store: JobStore,
        runtime: OcafRuntime,
        worker_id: &str,
        lease_ms: u64,
        allowed_tools: HashSet<String>,
    ) -> Result<Self, JobError> {
        validate_worker(worker_id)?;
        if !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&lease_ms)
            || allowed_tools.is_empty()
            || allowed_tools
                .iter()
                .any(|tool| tool.is_empty() || tool.len() > 128)
        {
            return Err(JobError::Invalid);
        }
        Ok(Self {
            store,
            runtime,
            worker_id: worker_id.to_owned(),
            lease_ms,
            allowed_tools,
        })
    }

    /// Processes at most one locally queued job. Returns None when idle.
    /// Deliberately does not implement automatic background scheduling.
    /// Recovery should be invoked by the supervisor before claiming work.
    pub async fn run_once(&self) -> Result<Option<JobStatus>, JobError> {
        let started = wallclock_ms()?;
        let job = match self
            .store
            .claim(&self.worker_id, started, self.lease_ms)
            .await?
        {
            Some(job) => job,
            None => return Ok(None),
        };

        let maybe_manifest = self.runtime.registry.resolve(&job.tool).await;
        let allowed = self.allowed_tools.contains(&job.tool)
            && maybe_manifest.as_ref().is_some_and(|handle| {
                handle.manifest.enabled
                    && handle.manifest.cost_sats == 0
                    && handle.manifest.permissions.is_empty()
                    && handle
                        .manifest
                        .timeout_ms
                        .checked_add(LEASE_MARGIN_MS)
                        .is_some_and(|limit| limit < self.lease_ms)
            });

        if !allowed {
            self.store
                .fail(&job, "job worker policy or lease safety margin denied", wallclock_ms()?)
                .await?;
        } else {
            let request = ToolCallRequest {
                call_id: job.id.clone(),
                tool: job.tool.clone(),
                args: job.args.clone(),
                auth: None,
            };
            match self.runtime.execute(request).await {
                Ok(result) => {
                    self.store.succeed(&job, result.output, wallclock_ms()?).await?;
                }
                Err(error) => {
                    // Do not store potentially sensitive error messages in
                    // the job ledger. Preserve only a stable diagnostic.
                    let reason = match error {
                        crate::runtime::RuntimeError::Timeout => "tool timeout",
                        crate::runtime::RuntimeError::InvalidInput(_) => "invalid input",
                        crate::runtime::RuntimeError::InvalidOutput(_) => "invalid output",
                        _ => "tool execution failed",
                    };
                    self.store.fail(&job, reason, wallclock_ms()?).await?;
                }
            }
        }

        Ok(self.store.get(&job.id).await?.map(|record| record.status))
    }

    pub async fn recover_expired(&self) -> Result<super::RecoverySummary, JobError> {
        self.store.recover_expired(wallclock_ms()?).await
    }
}
