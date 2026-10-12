//! Shared PostgreSQL durable remote job queue across coordinator processes.
//! It uses a *single PostgreSQL primary* as the transactional authority.
//! PostgreSQL itself must be deployed with separately configured replication
//! and failover; this module is not a consensus implementation.
//! Remote jobs are single-attempt to avoid retrying ambiguous side effects.

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_postgres::{Client, Row};
use uuid::Uuid;
use serde_json::Value;

use crate::jobs::{JobError, JobRecord, JobSpec, JobStatus, LeasedJob, RecoverySummary};
use super::leadership::{open_database, PgLeaderError};

const MAX_ARGS: usize = 32_768;

struct PgConnection {
    client: Mutex<Client>,
    task: JoinHandle<()>,
}

impl Drop for PgConnection {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone)]
pub struct PgJobQueue {
    state: Arc<PgConnection>,
}

fn storage(_: tokio_postgres::Error) -> JobError {
    JobError::Storage
}

fn millis(value: u64) -> Result<i64, JobError> {
    i64::try_from(value).map_err(|_| JobError::Invalid)
}

fn validate_actor(actor: &str) -> Result<(), JobError> {
    if actor.is_empty() || actor.len() > 64
        || !actor.bytes().all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    {
        return Err(JobError::Invalid);
    }
    Ok(())
}

fn from_row(row: &Row) -> Result<JobRecord, JobError> {
    let status: String = row.get("status");
    let status = match status.as_str() {
        "pending" => JobStatus::Pending,
        "running" => JobStatus::Running,
        "succeeded" => JobStatus::Succeeded,
        "failed" => JobStatus::Failed,
        "uncertain" => JobStatus::Uncertain,
        _ => return Err(JobError::Storage),
    };
    let arguments: String = row.get("args");
    let result: Option<String> = row.get("result");
    let attempts: i32 = row.get("attempts");
    let max_attempts: i32 = row.get("max_attempts");
    let expiry: Option<i64> = row.get("lease_expires_at_ms");
    Ok(JobRecord {
        id: row.get("id"),
        tool: row.get("tool"),
        args: serde_json::from_str(&arguments).map_err(|_| JobError::Storage)?,
        retry_safe: row.get("retry_safe"),
        max_attempts: u32::try_from(max_attempts).map_err(|_| JobError::Storage)?,
        status,
        attempts: u32::try_from(attempts).map_err(|_| JobError::Storage)?,
        result: result.map(|v| serde_json::from_str(&v).map_err(|_| JobError::Storage)).transpose()?,
        last_error: row.get("last_error"),
        worker_id: row.get("worker_id"),
        lease_expires_at_ms: expiry.map(|v| u64::try_from(v).map_err(|_| JobError::Storage)).transpose()?,
    })
}

const SELECT_RECORD: &str = "SELECT id,tool,args,retry_safe,max_attempts,status,attempts,result,
 last_error,worker_id,lease_expires_at_ms FROM ocaf_pg_jobs_v1 WHERE id=$1";

impl PgJobQueue {
    /// On remote networks SSL must be required; explicitly permitted local
    /// tests may use a loopback PostgreSQL service without TLS.
    pub async fn connect(dsn: &str, allow_insecure_loopback: bool) -> Result<Self, JobError> {
        let (client, task) = open_database(dsn, allow_insecure_loopback).await
            .map_err(|e| match e {
                PgLeaderError::UnsafeTransport | PgLeaderError::InvalidCluster => JobError::Invalid,
                PgLeaderError::Database => JobError::Storage,
            })?;
        client.batch_execute(
            "CREATE TABLE IF NOT EXISTS ocaf_pg_jobs_v1 (
                 seq BIGSERIAL PRIMARY KEY,
                 id TEXT NOT NULL UNIQUE,
                 owner TEXT NOT NULL,
                 tool TEXT NOT NULL,
                 args TEXT NOT NULL,
                 retry_safe BOOLEAN NOT NULL DEFAULT FALSE,
                 max_attempts INT NOT NULL DEFAULT 1,
                 status TEXT NOT NULL CHECK(status IN
                   ('pending','running','succeeded','failed','uncertain')),
                 attempts INT NOT NULL DEFAULT 0,
                 result TEXT,
                 last_error TEXT,
                 worker_id TEXT,
                 lease_token TEXT,
                 lease_expires_at_ms BIGINT
             );
             CREATE INDEX IF NOT EXISTS ocaf_pg_jobs_pending
                 ON ocaf_pg_jobs_v1(status,seq);
             CREATE INDEX IF NOT EXISTS ocaf_pg_jobs_expiration
                 ON ocaf_pg_jobs_v1(status,lease_expires_at_ms);"
        ).await.map_err(storage)?;
        Ok(Self {
            state: Arc::new(PgConnection { client: Mutex::new(client), task }),
        })
    }

    pub async fn submit_owned(&self, owner: &str, job: JobSpec) -> Result<JobRecord, JobError> {
        validate_actor(owner)?;
        if Uuid::parse_str(&job.id).is_err()
            || !crate::coordinator::validate_tool_name(&job.tool)
            || job.retry_safe
            || job.max_attempts != 1
        {
            return Err(JobError::Invalid);
        }
        let args = serde_json::to_string(&job.args).map_err(|_| JobError::Invalid)?;
        if args.len() > MAX_ARGS {
            return Err(JobError::Invalid);
        }

        let mut client = self.state.client.lock().await;
        let tx = client.transaction().await.map_err(storage)?;
        tx.execute(
            "INSERT INTO ocaf_pg_jobs_v1(id,owner,tool,args,retry_safe,max_attempts,status)
             VALUES($1,$2,$3,$4,FALSE,1,'pending') ON CONFLICT(id) DO NOTHING",
            &[&job.id, &owner, &job.tool, &args],
        ).await.map_err(storage)?;
        let row = tx.query_one(
            "SELECT id,owner,tool,args,retry_safe,max_attempts,status,attempts,result,
             last_error,worker_id,lease_expires_at_ms FROM ocaf_pg_jobs_v1 WHERE id=$1 FOR UPDATE",
            &[&job.id],
        ).await.map_err(storage)?;
        let stored_owner: String = row.get("owner");
        if stored_owner != owner || row.get::<_, String>("tool") != job.tool
            || serde_json::from_str::<Value>(&row.get::<_, String>("args"))
                .map_err(|_| JobError::Storage)? != job.args
        {
            return Err(JobError::Conflict);
        }
        let result = from_row(&row)?;
        tx.commit().await.map_err(storage)?;
        Ok(result)
    }

    /// PostgreSQL row-level locking and SKIP LOCKED allow workers connected
    /// through independent coordinator processes to claim distinct jobs.
    pub async fn claim_permitted(
        &self,
        worker: &str,
        now_ms: u64,
        lease_ms: u64,
        tools: &[String],
    ) -> Result<Option<LeasedJob>, JobError> {
        validate_actor(worker)?;
        if !(100..=300_000).contains(&lease_ms)
            || tools.is_empty()
            || tools.len() > 64
            || tools.iter().any(|t| !crate::coordinator::validate_tool_name(t))
        {
            return Err(JobError::Invalid);
        }
        let expires = millis(now_ms.checked_add(lease_ms).ok_or(JobError::Invalid)?)?;
        let mut client = self.state.client.lock().await;
        let tx = client.transaction().await.map_err(storage)?;
        let item = tx.query_opt(
            "SELECT id,tool,args,attempts FROM ocaf_pg_jobs_v1
             WHERE status='pending' AND tool=ANY($1)
             ORDER BY seq FOR UPDATE SKIP LOCKED LIMIT 1",
            &[&tools],
        ).await.map_err(storage)?;
        let Some(row) = item else {
            tx.commit().await.map_err(storage)?;
            return Ok(None);
        };
        let id: String = row.get("id");
        let tool: String = row.get("tool");
        let args: String = row.get("args");
        let prev: i32 = row.get("attempts");
        let attempt = prev.checked_add(1).ok_or(JobError::Storage)?;
        let token = Uuid::new_v4().to_string();
        tx.execute(
            "UPDATE ocaf_pg_jobs_v1 SET status='running',attempts=$2,
             worker_id=$3,lease_token=$4,lease_expires_at_ms=$5 WHERE id=$1",
            &[&id,&attempt,&worker,&token,&expires],
        ).await.map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        Ok(Some(LeasedJob {
            id,
            tool,
            args: serde_json::from_str(&args).map_err(|_| JobError::Storage)?,
            attempt: u32::try_from(attempt).map_err(|_| JobError::Storage)?,
            worker_id: worker.into(),
            lease_token: token,
            lease_expires_at_ms: u64::try_from(expires).map_err(|_| JobError::Storage)?,
        }))
    }

    pub async fn recover_expired(&self, now_ms: u64) -> Result<RecoverySummary, JobError> {
        let now = millis(now_ms)?;
        let client = self.state.client.lock().await;
        let changed = client.execute(
            "UPDATE ocaf_pg_jobs_v1 SET status='uncertain',
             last_error='expired remote lease; external outcome unknown',
             worker_id=NULL,lease_token=NULL,lease_expires_at_ms=NULL
             WHERE status='running' AND lease_expires_at_ms<=$1",
            &[&now],
        ).await.map_err(storage)?;
        Ok(RecoverySummary { retried: 0, uncertain: usize::try_from(changed).map_err(|_| JobError::Storage)? })
    }

    pub async fn get_owned(&self, owner: &str, id: &str) -> Result<Option<JobRecord>, JobError> {
        validate_actor(owner)?;
        let client = self.state.client.lock().await;
        let row = client.query_opt(
            "SELECT id,tool,args,retry_safe,max_attempts,status,attempts,result,
             last_error,worker_id,lease_expires_at_ms FROM ocaf_pg_jobs_v1
             WHERE id=$1 AND owner=$2",
            &[&id,&owner],
        ).await.map_err(storage)?;
        row.as_ref().map(from_row).transpose()
    }

    async fn finish(
        &self,
        lease: &LeasedJob,
        now_ms: u64,
        status: &str,
        result: Option<String>,
        error: Option<&str>,
    ) -> Result<(), JobError> {
        let now = millis(now_ms)?;
        if result.as_ref().is_some_and(|v| v.len() > MAX_ARGS) {
            return Err(JobError::Invalid);
        }
        let attempt = i32::try_from(lease.attempt).map_err(|_| JobError::Invalid)?;
        let client = self.state.client.lock().await;
        let changed = client.execute(
            "UPDATE ocaf_pg_jobs_v1 SET status=$6,result=$7,last_error=$8,
             worker_id=NULL,lease_token=NULL,lease_expires_at_ms=NULL
             WHERE id=$1 AND status='running' AND worker_id=$2
               AND lease_token=$3 AND tool=$4 AND attempts=$5
               AND lease_expires_at_ms>$9",
            &[&lease.id,&lease.worker_id,&lease.lease_token,&lease.tool,
              &attempt,&status,&result,&error,&now],
        ).await.map_err(storage)?;
        if changed != 1 {
            return Err(JobError::StaleLease);
        }
        Ok(())
    }

    pub async fn succeed(&self, lease: &LeasedJob, output: Value, now_ms: u64) -> Result<(), JobError> {
        let result = serde_json::to_string(&output).map_err(|_| JobError::Invalid)?;
        self.finish(lease, now_ms, "succeeded", Some(result), None).await
    }

    pub async fn fail(&self, lease: &LeasedJob, _: &str, now_ms: u64) -> Result<(), JobError> {
        self.finish(lease, now_ms, "failed", None, Some("remote worker reported failure")).await
    }

    pub async fn mark_uncertain(&self, lease: &LeasedJob, now_ms: u64) -> Result<(), JobError> {
        self.finish(lease, now_ms, "uncertain", None, Some("external outcome unknown")).await
    }
}
