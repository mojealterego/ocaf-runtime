//! Crash-resistant SQLite job journal for a single-host agent runtime.
//!
//! Lease fencing prevents stale workers from changing journal state. It does
//! not stop external side effects already started by an expired worker.
//! Retries are only automatic when the trusted submitter explicitly marks
//! a tool operation idempotent/retry-safe.
#[cfg(feature = "zenoh")]
pub mod remote;
pub mod worker;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

const MAX_JOB_JSON_BYTES: usize = 32_768;
const MAX_TOOL_NAME_BYTES: usize = 128;
const MAX_ATTEMPTS: u32 = 32;
const MIN_LEASE_MS: u64 = 100;
const MAX_LEASE_MS: u64 = 300_000;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum JobError {
    #[error("invalid job, worker, or lease configuration")]
    Invalid,
    #[error("job identifier is already associated with different content")]
    Conflict,
    #[error("SQLite job storage is unavailable")]
    Storage,
    #[error("job lease expired or owned by another worker")]
    StaleLease,
}

#[derive(Debug, Clone)]
pub struct JobSpec {
    pub id: String,
    pub tool: String,
    pub args: Value,
    /// May only be set by a trusted submitter for a genuinely idempotent tool.
    pub retry_safe: bool,
    pub max_attempts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Uncertain,
}

impl JobStatus {
    fn parse(status: &str) -> Result<Self, JobError> {
        match status {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "uncertain" => Ok(Self::Uncertain),
            _ => Err(JobError::Storage),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub tool: String,
    pub args: Value,
    pub retry_safe: bool,
    pub max_attempts: u32,
    pub status: JobStatus,
    pub attempts: u32,
    pub result: Option<Value>,
    pub last_error: Option<String>,
    pub worker_id: Option<String>,
    pub lease_expires_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeasedJob {
    pub id: String,
    pub tool: String,
    pub args: Value,
    pub attempt: u32,
    pub worker_id: String,
    /// Unique fencing token. Old workers cannot commit after lease rotation.
    pub lease_token: String,
    pub lease_expires_at_ms: u64,
    /// Leader generation is zero outside a PostgreSQL HA coordinator.
    #[serde(default, skip_serializing_if = "epoch_is_zero")]
    pub leader_epoch: i64,
}
fn epoch_is_zero(epoch: &i64) -> bool {
    *epoch == 0
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecoverySummary {
    pub retried: usize,
    pub uncertain: usize,
}

#[derive(Clone)]
pub struct JobStore {
    connection: Arc<Mutex<Connection>>,
}

fn validate_worker(worker: &str) -> Result<(), JobError> {
    if worker.is_empty()
        || worker.len() > 64
        || !worker
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(JobError::Invalid);
    }
    Ok(())
}

fn checked_time(now_ms: u64) -> Result<i64, JobError> {
    i64::try_from(now_ms).map_err(|_| JobError::Invalid)
}

fn checked_lease(now_ms: u64, lease_ms: u64) -> Result<i64, JobError> {
    if !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&lease_ms) {
        return Err(JobError::Invalid);
    }
    checked_time(now_ms.checked_add(lease_ms).ok_or(JobError::Invalid)?)
}

fn valid_json(data: &Value) -> Result<String, JobError> {
    let text = serde_json::to_string(data).map_err(|_| JobError::Invalid)?;
    if text.len() > MAX_JOB_JSON_BYTES {
        return Err(JobError::Invalid);
    }
    Ok(text)
}

fn load_record(conn: &Connection, id: &str) -> Result<Option<JobRecord>, JobError> {
    let raw = conn
        .query_row(
            "SELECT id, tool, args, retry_safe, max_attempts, status,
                    attempts, result, last_error, worker_id, lease_expires_at_ms
             FROM jobs_v1 WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                ))
            },
        )
        .optional()
        .map_err(|_| JobError::Storage)?;
    let Some((
        id,
        tool,
        args,
        retry_safe,
        max_attempts,
        status,
        attempts,
        result,
        last_error,
        worker_id,
        expires,
    )) = raw
    else {
        return Ok(None);
    };
    Ok(Some(JobRecord {
        id,
        tool,
        args: serde_json::from_str(&args).map_err(|_| JobError::Storage)?,
        retry_safe: retry_safe == 1,
        max_attempts: u32::try_from(max_attempts).map_err(|_| JobError::Storage)?,
        status: JobStatus::parse(&status)?,
        attempts: u32::try_from(attempts).map_err(|_| JobError::Storage)?,
        result: result
            .map(|json| serde_json::from_str(&json).map_err(|_| JobError::Storage))
            .transpose()?,
        last_error,
        worker_id,
        lease_expires_at_ms: expires
            .map(|value| u64::try_from(value).map_err(|_| JobError::Storage))
            .transpose()?,
    }))
}

impl JobStore {
    pub fn open(path: &Path) -> Result<Self, JobError> {
        if path.as_os_str().is_empty() {
            return Err(JobError::Invalid);
        }
        #[cfg(unix)]
        {
            use std::fs::{OpenOptions, Permissions};
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            if let Ok(metadata) = std::fs::symlink_metadata(path) {
                if metadata.file_type().is_symlink() {
                    return Err(JobError::Storage);
                }
            }
            let _file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(path)
                .map_err(|_| JobError::Storage)?;
            std::fs::set_permissions(path, Permissions::from_mode(0o600))
                .map_err(|_| JobError::Storage)?;
        }
        let conn = Connection::open(path).map_err(|_| JobError::Storage)?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|_| JobError::Storage)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             CREATE TABLE IF NOT EXISTS jobs_v1 (
                 seq INTEGER PRIMARY KEY AUTOINCREMENT,
                 id TEXT NOT NULL UNIQUE,
                 tool TEXT NOT NULL,
                 args TEXT NOT NULL,
                 retry_safe INTEGER NOT NULL CHECK (retry_safe IN (0,1)),
                 max_attempts INTEGER NOT NULL,
                 status TEXT NOT NULL CHECK (status IN
                     ('pending','running','succeeded','failed','uncertain')),
                 attempts INTEGER NOT NULL DEFAULT 0,
                 result TEXT,
                 last_error TEXT,
                 worker_id TEXT,
                 lease_token TEXT,
                 lease_expires_at_ms INTEGER
             );
             CREATE TABLE IF NOT EXISTS job_owners_v1 (
                 job_id TEXT PRIMARY KEY NOT NULL,
                 owner TEXT NOT NULL,
                 FOREIGN KEY(job_id) REFERENCES jobs_v1(id)
             );
             CREATE INDEX IF NOT EXISTS idx_jobs_pending
                 ON jobs_v1(status, seq);
             CREATE INDEX IF NOT EXISTS idx_jobs_leases
                 ON jobs_v1(status, lease_expires_at_ms);",
        )
        .map_err(|_| JobError::Storage)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(conn)),
        })
    }

    async fn run<T, F>(&self, action: F) -> Result<T, JobError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, JobError> + Send + 'static,
    {
        let conn = Arc::clone(&self.connection);
        tokio::task::spawn_blocking(move || {
            let mut locked = conn.lock().map_err(|_| JobError::Storage)?;
            action(&mut locked)
        })
        .await
        .map_err(|_| JobError::Storage)?
    }

    /// Identical job submissions return the existing record, while a UUID
    /// collision with changed arguments or retry policy is rejected.
    pub async fn submit(&self, spec: JobSpec) -> Result<JobRecord, JobError> {
        self.submit_inner(spec, None).await
    }

    /// Atomically bind a remote job UUID to its authenticated submitter.
    /// An unrelated submitter may neither claim the ID nor read its result.
    pub async fn submit_owned(&self, owner: &str, spec: JobSpec) -> Result<JobRecord, JobError> {
        validate_worker(owner)?;
        self.submit_inner(spec, Some(owner.to_owned())).await
    }

    async fn submit_inner(
        &self,
        spec: JobSpec,
        owner: Option<String>,
    ) -> Result<JobRecord, JobError> {
        if Uuid::parse_str(&spec.id).is_err()
            || spec.tool.is_empty()
            || spec.tool.len() > MAX_TOOL_NAME_BYTES
            || spec.max_attempts == 0
            || spec.max_attempts > MAX_ATTEMPTS
            || (!spec.retry_safe && spec.max_attempts != 1)
        {
            return Err(JobError::Invalid);
        }
        let args = valid_json(&spec.args)?;
        self.run(move |conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| JobError::Storage)?;
            tx.execute(
                "INSERT OR IGNORE INTO jobs_v1
                 (id, tool, args, retry_safe, max_attempts, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
                params![
                    spec.id,
                    spec.tool,
                    args,
                    i64::from(spec.retry_safe),
                    spec.max_attempts,
                ],
            )
            .map_err(|_| JobError::Storage)?;
            let existing = load_record(&tx, &spec.id)?.ok_or(JobError::Storage)?;
            if existing.tool != spec.tool
                || existing.args != spec.args
                || existing.retry_safe != spec.retry_safe
                || existing.max_attempts != spec.max_attempts
            {
                return Err(JobError::Conflict);
            }
            if let Some(owner) = owner {
                tx.execute(
                    "INSERT OR IGNORE INTO job_owners_v1(job_id,owner) VALUES (?1,?2)",
                    params![spec.id, owner],
                )
                .map_err(|_| JobError::Storage)?;
                let saved: String = tx
                    .query_row(
                        "SELECT owner FROM job_owners_v1 WHERE job_id=?1",
                        params![spec.id],
                        |row| row.get(0),
                    )
                    .map_err(|_| JobError::Storage)?;
                if saved != owner {
                    return Err(JobError::Conflict);
                }
            }
            tx.commit().map_err(|_| JobError::Storage)?;
            Ok(existing)
        })
        .await
    }

    /// Atomically take the oldest pending job; callers own a unique fencing
    /// token valid only until the recorded lease expiry.
    pub async fn claim(
        &self,
        worker: &str,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<Option<LeasedJob>, JobError> {
        self.claim_filtered(worker, now_ms, lease_ms, None).await
    }

    /// Atomic claim restricted to a worker's authenticated, administrator
    /// configured tool allowlist. Never take an unrelated queued task.
    pub async fn claim_permitted(
        &self,
        worker: &str,
        now_ms: u64,
        lease_ms: u64,
        tools: &[String],
    ) -> Result<Option<LeasedJob>, JobError> {
        if tools.is_empty()
            || tools.len() > 64
            || tools
                .iter()
                .any(|name| !crate::coordinator::validate_tool_name(name))
        {
            return Err(JobError::Invalid);
        }
        self.claim_filtered(worker, now_ms, lease_ms, Some(tools.to_vec()))
            .await
    }

    async fn claim_filtered(
        &self,
        worker: &str,
        now_ms: u64,
        lease_ms: u64,
        allowed: Option<Vec<String>>,
    ) -> Result<Option<LeasedJob>, JobError> {
        validate_worker(worker)?;
        let expires = checked_lease(now_ms, lease_ms)?;
        let worker = worker.to_owned();
        self.run(move |conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| JobError::Storage)?;
            let sql = if let Some(ref tools) = allowed {
                format!(
                    "SELECT id, tool, args, attempts FROM jobs_v1
                     WHERE status='pending' AND tool IN ({})
                     ORDER BY seq LIMIT 1",
                    vec!["?"; tools.len()].join(",")
                )
            } else {
                "SELECT id, tool, args, attempts FROM jobs_v1
                 WHERE status='pending' ORDER BY seq LIMIT 1"
                    .into()
            };
            let arguments = allowed.as_deref().unwrap_or_default();
            let row = tx
                .query_row(&sql, rusqlite::params_from_iter(arguments.iter()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .optional()
                .map_err(|_| JobError::Storage)?;
            let Some((id, tool, args, previous_attempts)) = row else {
                return Ok(None);
            };
            let attempt = u32::try_from(previous_attempts)
                .map_err(|_| JobError::Storage)?
                .checked_add(1)
                .ok_or(JobError::Storage)?;
            let token = Uuid::new_v4().to_string();
            tx.execute(
                "UPDATE jobs_v1
                 SET status='running', attempts=?2, worker_id=?3,
                     lease_token=?4, lease_expires_at_ms=?5
                 WHERE id=?1 AND status='pending'",
                params![id, attempt, worker, token, expires],
            )
            .map_err(|_| JobError::Storage)?;
            tx.commit().map_err(|_| JobError::Storage)?;
            Ok(Some(LeasedJob {
                id,
                tool,
                args: serde_json::from_str(&args).map_err(|_| JobError::Storage)?,
                attempt,
                worker_id: worker,
                lease_token: token,
                lease_expires_at_ms: u64::try_from(expires).map_err(|_| JobError::Storage)?,
                leader_epoch: 0,
            }))
        })
        .await
    }

    pub async fn heartbeat(
        &self,
        job: &LeasedJob,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<(), JobError> {
        let expires = checked_lease(now_ms, lease_ms)?;
        let now = checked_time(now_ms)?;
        let job = job.clone();
        self.run(move |conn| {
            let changed = conn
                .execute(
                    "UPDATE jobs_v1 SET lease_expires_at_ms=?5
                     WHERE id=?1 AND status='running' AND worker_id=?2
                       AND lease_token=?3 AND lease_expires_at_ms>?4",
                    params![job.id, job.worker_id, job.lease_token, now, expires],
                )
                .map_err(|_| JobError::Storage)?;
            if changed != 1 {
                return Err(JobError::StaleLease);
            }
            Ok(())
        })
        .await
    }

    pub async fn succeed(
        &self,
        job: &LeasedJob,
        output: Value,
        now_ms: u64,
    ) -> Result<(), JobError> {
        let result = valid_json(&output)?;
        let now = checked_time(now_ms)?;
        let job = job.clone();
        self.run(move |conn| {
            let changed = conn
                .execute(
                    "UPDATE jobs_v1 SET status='succeeded', result=?5,
                     worker_id=NULL, lease_token=NULL, lease_expires_at_ms=NULL
                     WHERE id=?1 AND status='running' AND worker_id=?2
                       AND lease_token=?3 AND lease_expires_at_ms>?4",
                    params![job.id, job.worker_id, job.lease_token, now, result],
                )
                .map_err(|_| JobError::Storage)?;
            if changed != 1 {
                return Err(JobError::StaleLease);
            }
            Ok(())
        })
        .await
    }

    /// Mark an ambiguous external side-effecting attempt as terminal
    /// uncertain. A stale lease holder cannot commit this transition.
    pub async fn mark_uncertain(&self, job: &LeasedJob, now_ms: u64) -> Result<(), JobError> {
        let now = checked_time(now_ms)?;
        let job = job.clone();
        self.run(move |conn| {
            let changed = conn
                .execute(
                    "UPDATE jobs_v1 SET status='uncertain',
                        last_error='external outcome unknown',
                        worker_id=NULL, lease_token=NULL,
                        lease_expires_at_ms=NULL
                     WHERE id=?1 AND status='running' AND worker_id=?2
                       AND lease_token=?3 AND lease_expires_at_ms>?4",
                    params![job.id, job.worker_id, job.lease_token, now],
                )
                .map_err(|_| JobError::Storage)?;
            if changed != 1 {
                return Err(JobError::StaleLease);
            }
            Ok(())
        })
        .await
    }

    pub async fn fail(&self, job: &LeasedJob, reason: &str, now_ms: u64) -> Result<(), JobError> {
        if reason.is_empty() || reason.len() > 1024 {
            return Err(JobError::Invalid);
        }
        let now = checked_time(now_ms)?;
        let job = job.clone();
        let reason = reason.to_owned();
        self.run(move |conn| {
            let changed = conn
                .execute(
                    "UPDATE jobs_v1
                     SET status=CASE WHEN retry_safe=1 AND attempts<max_attempts
                              THEN 'pending' ELSE 'failed' END,
                         last_error=?5, worker_id=NULL,
                         lease_token=NULL, lease_expires_at_ms=NULL
                     WHERE id=?1 AND status='running' AND worker_id=?2
                       AND lease_token=?3 AND lease_expires_at_ms>?4",
                    params![job.id, job.worker_id, job.lease_token, now, reason],
                )
                .map_err(|_| JobError::Storage)?;
            if changed != 1 {
                return Err(JobError::StaleLease);
            }
            Ok(())
        })
        .await
    }

    /// Expired running operations are either requeued if explicitly declared
    /// retry-safe AND under max_attempts, or marked uncertain. The latter
    /// requires manual reconciliation of external effects.
    pub async fn recover_expired(&self, now_ms: u64) -> Result<RecoverySummary, JobError> {
        let now = checked_time(now_ms)?;
        self.run(move |conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| JobError::Storage)?;
            let retried = tx
                .execute(
                    "UPDATE jobs_v1 SET status='pending', worker_id=NULL,
                            lease_token=NULL, lease_expires_at_ms=NULL
                     WHERE status='running' AND lease_expires_at_ms<=?1
                       AND retry_safe=1 AND attempts<max_attempts",
                    params![now],
                )
                .map_err(|_| JobError::Storage)?;
            let uncertain = tx
                .execute(
                    "UPDATE jobs_v1 SET status='uncertain', worker_id=NULL,
                            lease_token=NULL, lease_expires_at_ms=NULL,
                            last_error='lease expired; external outcome unknown'
                     WHERE status='running' AND lease_expires_at_ms<=?1",
                    params![now],
                )
                .map_err(|_| JobError::Storage)?;
            tx.commit().map_err(|_| JobError::Storage)?;
            Ok(RecoverySummary { retried, uncertain })
        })
        .await
    }

    /// Return a job only when the caller owns its remote submission ID.
    /// Unowned legacy/local jobs are not visible over this interface.
    pub async fn get_owned(&self, owner: &str, id: &str) -> Result<Option<JobRecord>, JobError> {
        validate_worker(owner)?;
        let owner = owner.to_owned();
        let id = id.to_owned();
        self.run(move |conn| {
            let saved: Option<String> = conn
                .query_row(
                    "SELECT owner FROM job_owners_v1 WHERE job_id=?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| JobError::Storage)?;
            if saved.as_deref() != Some(owner.as_str()) {
                return Ok(None);
            }
            load_record(conn, &id)
        })
        .await
    }

    pub async fn get(&self, id: &str) -> Result<Option<JobRecord>, JobError> {
        let id = id.to_owned();
        self.run(move |conn| load_record(conn, &id)).await
    }
}
