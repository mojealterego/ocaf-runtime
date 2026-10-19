//! Trusted local job worker. Jobs execute through the normal runtime
//! authorization and JSON Schema validation path, not direct ToolExecutor.
//!
//! A lease must outlive the manifest's execution timeout by a safety margin.
//! Stale lease holders are prevented from committing *journal* results; tool
//! side effects must still be explicitly idempotent if retries are enabled.

use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

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
            || allowed_tools.len() > 64
            || allowed_tools
                .iter()
                .any(|tool| !crate::coordinator::validate_tool_name(tool))
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
        // A restarted worker reconciles expired claims before claiming
        // further jobs. Non-idempotent/unsafe work remains "uncertain".
        self.store.recover_expired(started).await?;
        // Never claim a task owned by a different tool worker. Claiming first
        // and rejecting afterwards would terminally fail unrelated work.
        let mut permitted: Vec<_> = self.allowed_tools.iter().cloned().collect();
        permitted.sort();
        let job = match self
            .store
            .claim_permitted(&self.worker_id, started, self.lease_ms, &permitted)
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
                .fail(
                    &job,
                    "job worker policy or lease safety margin denied",
                    wallclock_ms()?,
                )
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
                    self.store
                        .succeed(&job, result.output, wallclock_ms()?)
                        .await?;
                }
                Err(error) => {
                    // Do not store potentially sensitive error messages in
                    // the job ledger. Preserve only a stable diagnostic.
                    use crate::runtime::RuntimeError;
                    let definitely_pre_execution = matches!(
                        error,
                        RuntimeError::InvalidInput(_)
                            | RuntimeError::ToolNotFound(_)
                            | RuntimeError::ToolDisabled(_)
                            | RuntimeError::AuthRequired(_)
                            | RuntimeError::BudgetExceeded
                    );
                    if !definitely_pre_execution
                        && !self
                            .store
                            .get(&job.id)
                            .await?
                            .ok_or(JobError::Storage)?
                            .retry_safe
                    {
                        // A timed-out or failed external tool may have made
                        // irreversible changes before returning an error.
                        self.store.mark_uncertain(&job, wallclock_ms()?).await?;
                    } else {
                        self.store
                            .fail(&job, "validated execution failure", wallclock_ms()?)
                            .await?;
                    }
                }
            }
        }

        Ok(self.store.get(&job.id).await?.map(|record| record.status))
    }

    /// Poll the durable local queue continuously until the trusted supervisor
    /// requests shutdown or drops the shutdown channel.
    ///
    /// This does not start external Make scenarios. In-flight jobs are
    /// permitted to finish before shutdown, preserving journal fencing.
    /// On a storage/runtime error the supervisor must reconcile and restart.
    pub async fn run_until_shutdown(
        &self,
        interval: Duration,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<(), JobError> {
        if interval < Duration::from_millis(25) || interval > Duration::from_secs(60) {
            return Err(JobError::Invalid);
        }
        let mut ticks = tokio::time::interval(interval);
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if *shutdown.borrow() {
                return Ok(());
            }
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow_and_update() {
                        return Ok(());
                    }
                }
                _ = ticks.tick() => {
                    self.run_once().await?;
                }
            }
        }
    }

    pub async fn recover_expired(&self) -> Result<super::RecoverySummary, JobError> {
        self.store.recover_expired(wallclock_ms()?).await
    }
}

#[cfg(test)]
mod worker_regression_tests {
    use super::super::JobSpec;
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn worker(store: &JobStore, allow: &str) -> JobWorker {
        JobWorker::new(
            store.clone(),
            OcafRuntime::new(0),
            "worker_a",
            5_000,
            HashSet::from([allow.to_owned()]),
        )
        .unwrap()
    }

    async fn submit(store: &JobStore, tool: &str) -> String {
        let id = Uuid::new_v4().to_string();
        store
            .submit(JobSpec {
                id: id.clone(),
                tool: tool.into(),
                args: json!({}),
                retry_safe: false,
                max_attempts: 1,
            })
            .await
            .unwrap();
        id
    }

    #[tokio::test]
    async fn worker_never_steals_another_tools_queued_job() {
        let dir = tempfile::tempdir().unwrap();
        let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
        let other = submit(&store, "other_tool").await;
        let mine = submit(&store, "my_tool").await;
        // No executor registered: the permitted job is rejected, but the
        // unrelated earlier job must remain pending and unclaimed.
        assert_eq!(
            worker(&store, "my_tool").run_once().await.unwrap(),
            Some(JobStatus::Failed)
        );
        let unrelated = store.get(&other).await.unwrap().unwrap();
        assert_eq!(unrelated.status, JobStatus::Pending);
        assert_eq!(unrelated.attempts, 0);
        assert_eq!(
            store.get(&mine).await.unwrap().unwrap().status,
            JobStatus::Failed
        );
    }

    #[tokio::test]
    async fn worker_remains_idle_when_only_other_workers_have_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
        let other = submit(&store, "other_tool").await;
        assert_eq!(worker(&store, "my_tool").run_once().await.unwrap(), None);
        assert_eq!(
            store.get(&other).await.unwrap().unwrap().status,
            JobStatus::Pending
        );
    }

    #[tokio::test]
    async fn shutdown_and_poll_interval_are_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
        let runner = worker(&store, "my_tool");
        let (_sender, receiver) = watch::channel(true);
        assert!(matches!(
            runner
                .run_until_shutdown(Duration::ZERO, receiver.clone())
                .await,
            Err(JobError::Invalid)
        ));
        assert!(matches!(
            runner
                .run_until_shutdown(Duration::from_secs(61), receiver.clone())
                .await,
            Err(JobError::Invalid)
        ));
        assert!(runner
            .run_until_shutdown(Duration::from_millis(25), receiver)
            .await
            .is_ok());
    }

    #[test]
    fn worker_rejects_invalid_tool_allowlists() {
        let dir = tempfile::tempdir().unwrap();
        let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
        let invalid = JobWorker::new(
            store.clone(),
            OcafRuntime::new(0),
            "worker_a",
            5_000,
            HashSet::from(["bad/tool".to_owned()]),
        );
        assert!(matches!(invalid, Err(JobError::Invalid)));
        let oversized: HashSet<String> = (0..65).map(|i| format!("tool_{i}")).collect();
        let oversized_worker =
            JobWorker::new(store, OcafRuntime::new(0), "worker_a", 5_000, oversized);
        assert!(matches!(oversized_worker, Err(JobError::Invalid)));
    }
}
