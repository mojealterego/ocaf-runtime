//! Uniform coordinator RPC job API backed by local SQLite or shared PostgreSQL.
use std::sync::Arc;

use crate::cluster::jobs::{FencedEffectReceipt, PgJobQueue};
use crate::cluster::leadership::PgLeader;
use crate::jobs::{JobError, JobRecord, JobSpec, JobStore, LeasedJob, RecoverySummary};
use serde_json::Value;

pub(super) enum JobBackend {
    Sqlite(JobStore),
    Postgres(PgJobQueue),
}

impl JobBackend {
    pub(super) fn with_leader(self, leader: Arc<PgLeader>) -> Self {
        match self {
            Self::Postgres(db) => Self::Postgres(db.with_leader(leader)),
            other => other,
        }
    }

    pub(super) async fn validate_current_lease(&self, lease: &LeasedJob) -> Result<bool, JobError> {
        match self {
            Self::Postgres(db) => db.validate_current_lease(lease).await,
            Self::Sqlite(_) => Err(JobError::Invalid),
        }
    }

    pub(super) async fn apply_fenced_effect(
        &self,
        lease: &LeasedJob,
        effect: Value,
    ) -> Result<FencedEffectReceipt, JobError> {
        match self {
            Self::Postgres(db) => db.apply_fenced_effect(lease, effect).await,
            Self::Sqlite(_) => Err(JobError::Invalid),
        }
    }

    pub(super) async fn submit_owned(
        &self,
        owner: &str,
        job: JobSpec,
    ) -> Result<JobRecord, JobError> {
        match self {
            Self::Sqlite(db) => db.submit_owned(owner, job).await,
            Self::Postgres(db) => db.submit_owned(owner, job).await,
        }
    }

    pub(super) async fn get_owned(
        &self,
        owner: &str,
        id: &str,
    ) -> Result<Option<JobRecord>, JobError> {
        match self {
            Self::Sqlite(db) => db.get_owned(owner, id).await,
            Self::Postgres(db) => db.get_owned(owner, id).await,
        }
    }

    pub(super) async fn claim_permitted(
        &self,
        worker: &str,
        now: u64,
        lease_ms: u64,
        tools: &[String],
    ) -> Result<Option<LeasedJob>, JobError> {
        match self {
            Self::Sqlite(db) => db.claim_permitted(worker, now, lease_ms, tools).await,
            Self::Postgres(db) => db.claim_permitted(worker, now, lease_ms, tools).await,
        }
    }

    pub(super) async fn recover_expired(&self, now: u64) -> Result<RecoverySummary, JobError> {
        match self {
            Self::Sqlite(db) => db.recover_expired(now).await,
            Self::Postgres(db) => db.recover_expired(now).await,
        }
    }

    pub(super) async fn succeed(
        &self,
        lease: &LeasedJob,
        value: Value,
        now: u64,
    ) -> Result<(), JobError> {
        match self {
            Self::Sqlite(db) => db.succeed(lease, value, now).await,
            Self::Postgres(db) => db.succeed(lease, value, now).await,
        }
    }

    pub(super) async fn fail(
        &self,
        lease: &LeasedJob,
        reason: &str,
        now: u64,
    ) -> Result<(), JobError> {
        match self {
            Self::Sqlite(db) => db.fail(lease, reason, now).await,
            Self::Postgres(db) => db.fail(lease, reason, now).await,
        }
    }

    pub(super) async fn mark_uncertain(&self, lease: &LeasedJob, now: u64) -> Result<(), JobError> {
        match self {
            Self::Sqlite(db) => db.mark_uncertain(lease, now).await,
            Self::Postgres(db) => db.mark_uncertain(lease, now).await,
        }
    }
    /// Audit is currently backed by PostgreSQL only. A legacy SQLite
    /// coordinator fails closed rather than fabricating audit history.
    pub(super) async fn audit_owned(
        &self,
        owner: &str,
        id: &str,
        limit: usize,
    ) -> Result<Vec<crate::cluster::jobs::JobAuditEvent>, JobError> {
        match self {
            Self::Sqlite(_) => Err(JobError::Invalid),
            Self::Postgres(db) => db.audit_owned(owner, id, limit).await,
        }
    }
}
