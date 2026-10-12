//! Uniform coordinator RPC job API backed by local SQLite or shared PostgreSQL.
use crate::cluster::jobs::PgJobQueue;
use crate::jobs::{JobError, JobRecord, JobSpec, JobStore, LeasedJob, RecoverySummary};
use serde_json::Value;

pub(super) enum JobBackend {
    Sqlite(JobStore),
    Postgres(PgJobQueue),
}

impl JobBackend {
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
}
