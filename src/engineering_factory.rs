//! Durable, single-host admission queue for independent engineering workers.
//!
//! This module does not run a model or execute code. A supervisor must give
//! each claimed item its own isolated checkout and must validate the result.
//! Expired claims become `uncertain`: repo edits may have happened already.

use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const MAX_TEXT_BYTES: usize = 32_768;
const MAX_LEASE_MS: i64 = 300_000;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FactoryError {
    #[error("invalid factory input")]
    Invalid,
    #[error("identifier already exists with different content")]
    Conflict,
    #[error("mission or dependency does not exist")]
    Missing,
    #[error("claim is stale or expired")]
    StaleClaim,
    #[error("factory storage is unavailable")]
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionSpec {
    pub id: String,
    pub repository: String,
    pub base_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkSpec {
    pub id: String,
    pub mission_id: String,
    pub role: String,
    pub instructions: String,
    /// Dependencies must already exist in the same mission. This insertion
    /// order prevents cycles without a separate graph reconciliation pass.
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Uncertain,
}

impl WorkStatus {
    fn parse(value: &str) -> Result<Self, FactoryError> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "uncertain" => Ok(Self::Uncertain),
            _ => Err(FactoryError::Storage),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRecord {
    pub spec: WorkSpec,
    pub status: WorkStatus,
    pub attempt: i64,
    pub worker_id: Option<String>,
    pub lease_expires_at_ms: Option<i64>,
    pub result_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub work: WorkSpec,
    pub worker_id: String,
    pub token: String,
    pub attempt: i64,
    pub lease_expires_at_ms: i64,
}

/// Open a separate `FactoryStore` per process. SQLite serializes competing
/// claims with `BEGIN IMMEDIATE`, including across independent processes.
pub struct FactoryStore {
    connection: Connection,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_TEXT_BYTES
}

fn db_now(connection: &Connection) -> Result<i64, FactoryError> {
    connection
        .query_row(
            "SELECT CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)",
            [],
            |row| row.get(0),
        )
        .map_err(|_| FactoryError::Storage)
}

fn storage(_: rusqlite::Error) -> FactoryError {
    FactoryError::Storage
}

impl FactoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FactoryError> {
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(storage)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS engineering_missions_v1 (
                   id TEXT PRIMARY KEY, repository TEXT NOT NULL, base_ref TEXT NOT NULL,
                   created_at_ms INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS engineering_work_v1 (
                   id TEXT PRIMARY KEY, mission_id TEXT NOT NULL REFERENCES engineering_missions_v1(id),
                   role TEXT NOT NULL, instructions TEXT NOT NULL,
                   status TEXT NOT NULL CHECK (status IN ('pending','running','succeeded','failed','uncertain')),
                   attempt INTEGER NOT NULL DEFAULT 0, worker_id TEXT, token TEXT,
                   lease_expires_at_ms INTEGER, result_ref TEXT,
                   created_at_ms INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS engineering_dependencies_v1 (
                   work_id TEXT NOT NULL REFERENCES engineering_work_v1(id),
                   dependency_id TEXT NOT NULL REFERENCES engineering_work_v1(id),
                   PRIMARY KEY (work_id, dependency_id),
                   CHECK (work_id <> dependency_id)
                 );
                 CREATE INDEX IF NOT EXISTS engineering_ready_v1
                   ON engineering_work_v1(status, created_at_ms, id);",
            )
            .map_err(storage)?;
        Ok(Self { connection })
    }

    pub fn submit_mission(&mut self, spec: &MissionSpec) -> Result<(), FactoryError> {
        if !valid_id(&spec.id) || !valid_text(&spec.repository) || !valid_text(&spec.base_ref) {
            return Err(FactoryError::Invalid);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT repository, base_ref FROM engineering_missions_v1 WHERE id = ?1",
                [&spec.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        if let Some((repository, base_ref)) = existing {
            return if repository == spec.repository && base_ref == spec.base_ref {
                Ok(())
            } else {
                Err(FactoryError::Conflict)
            };
        }
        let now = db_now(&tx)?;
        tx.execute(
            "INSERT INTO engineering_missions_v1 VALUES (?1,?2,?3,?4)",
            params![spec.id, spec.repository, spec.base_ref, now],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)
    }

    pub fn submit_work(&mut self, spec: &WorkSpec) -> Result<(), FactoryError> {
        if !valid_id(&spec.id)
            || !valid_id(&spec.mission_id)
            || !valid_id(&spec.role)
            || !valid_text(&spec.instructions)
            || spec.depends_on.len() > 128
            || spec
                .depends_on
                .iter()
                .any(|id| !valid_id(id) || id == &spec.id)
        {
            return Err(FactoryError::Invalid);
        }
        let mut dependencies = spec.depends_on.clone();
        dependencies.sort();
        dependencies.dedup();
        if dependencies.len() != spec.depends_on.len() {
            return Err(FactoryError::Invalid);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let existing: Option<(String, String, String)> = tx
            .query_row(
                "SELECT mission_id, role, instructions FROM engineering_work_v1 WHERE id = ?1",
                [&spec.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage)?;
        if let Some((mission_id, role, instructions)) = existing {
            let mut stmt = tx
                .prepare("SELECT dependency_id FROM engineering_dependencies_v1 WHERE work_id = ?1 ORDER BY dependency_id")
                .map_err(storage)?;
            let prior: Vec<String> = stmt
                .query_map([&spec.id], |row| row.get(0))
                .map_err(storage)?
                .collect::<Result<_, _>>()
                .map_err(storage)?;
            return if mission_id == spec.mission_id
                && role == spec.role
                && instructions == spec.instructions
                && prior == dependencies
            {
                Ok(())
            } else {
                Err(FactoryError::Conflict)
            };
        }
        let mission_exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM engineering_missions_v1 WHERE id = ?1)",
                [&spec.mission_id],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if !mission_exists {
            return Err(FactoryError::Missing);
        }
        for id in &dependencies {
            let dependency_mission: Option<String> = tx
                .query_row(
                    "SELECT mission_id FROM engineering_work_v1 WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            if dependency_mission.as_deref() != Some(spec.mission_id.as_str()) {
                return Err(FactoryError::Missing);
            }
        }
        let now = db_now(&tx)?;
        tx.execute(
            "INSERT INTO engineering_work_v1
             (id, mission_id, role, instructions, status, created_at_ms)
             VALUES (?1,?2,?3,?4,'pending',?5)",
            params![spec.id, spec.mission_id, spec.role, spec.instructions, now],
        )
        .map_err(storage)?;
        for id in dependencies {
            tx.execute(
                "INSERT INTO engineering_dependencies_v1 VALUES (?1,?2)",
                params![spec.id, id],
            )
            .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }

    /// Recovers expired claims as uncertain and claims one ready item.
    /// There is no automatic retry of an expired engineering task.
    pub fn claim(&mut self, worker_id: &str, lease_ms: i64) -> Result<Option<Claim>, FactoryError> {
        if !valid_id(worker_id) || !(100..=MAX_LEASE_MS).contains(&lease_ms) {
            return Err(FactoryError::Invalid);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let now = db_now(&tx)?;
        tx.execute(
            "UPDATE engineering_work_v1 SET status='uncertain', token=NULL,
             lease_expires_at_ms=NULL WHERE status='running' AND lease_expires_at_ms <= ?1",
            [now],
        )
        .map_err(storage)?;
        let row: Option<(String, String, String, String, i64)> = tx
            .query_row(
                "SELECT w.id, w.mission_id, w.role, w.instructions, w.attempt
                 FROM engineering_work_v1 w
                 WHERE w.status='pending' AND NOT EXISTS (
                   SELECT 1 FROM engineering_dependencies_v1 d
                   JOIN engineering_work_v1 upstream ON upstream.id=d.dependency_id
                   WHERE d.work_id=w.id AND upstream.status <> 'succeeded'
                 ) ORDER BY w.created_at_ms, w.id LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()
            .map_err(storage)?;
        let Some((id, mission_id, role, instructions, old_attempt)) = row else {
            tx.commit().map_err(storage)?;
            return Ok(None);
        };
        let dependencies: Vec<String> = {
            let mut stmt = tx
                .prepare("SELECT dependency_id FROM engineering_dependencies_v1 WHERE work_id=?1 ORDER BY dependency_id")
                .map_err(storage)?;
            let rows = stmt
                .query_map([&id], |row| row.get(0))
                .map_err(storage)?
                .collect::<Result<_, _>>()
                .map_err(storage)?;
            rows
        };
        let token = Uuid::new_v4().to_string();
        let attempt = old_attempt.checked_add(1).ok_or(FactoryError::Storage)?;
        let expiry = now.checked_add(lease_ms).ok_or(FactoryError::Invalid)?;
        tx.execute(
            "UPDATE engineering_work_v1 SET status='running', worker_id=?2,
             token=?3, attempt=?4, lease_expires_at_ms=?5 WHERE id=?1 AND status='pending'",
            params![id, worker_id, token, attempt, expiry],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(Some(Claim {
            work: WorkSpec {
                id,
                mission_id,
                role,
                instructions,
                depends_on: dependencies,
            },
            worker_id: worker_id.to_owned(),
            token,
            attempt,
            lease_expires_at_ms: expiry,
        }))
    }

    pub fn heartbeat(&mut self, claim: &Claim, lease_ms: i64) -> Result<i64, FactoryError> {
        if !(100..=MAX_LEASE_MS).contains(&lease_ms) {
            return Err(FactoryError::Invalid);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let now = db_now(&tx)?;
        let expiry = now.checked_add(lease_ms).ok_or(FactoryError::Invalid)?;
        let changed = tx
            .execute(
                "UPDATE engineering_work_v1 SET lease_expires_at_ms=?5
             WHERE id=?1 AND status='running' AND worker_id=?2 AND token=?3
             AND attempt=?4 AND lease_expires_at_ms > ?6",
                params![
                    claim.work.id,
                    claim.worker_id,
                    claim.token,
                    claim.attempt,
                    expiry,
                    now
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(FactoryError::StaleClaim);
        }
        tx.commit().map_err(storage)?;
        Ok(expiry)
    }

    pub fn finish(
        &mut self,
        claim: &Claim,
        succeeded: bool,
        result_ref: &str,
    ) -> Result<(), FactoryError> {
        if !valid_text(result_ref) {
            return Err(FactoryError::Invalid);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let now = db_now(&tx)?;
        let status = if succeeded { "succeeded" } else { "failed" };
        let changed = tx
            .execute(
                "UPDATE engineering_work_v1 SET status=?5, result_ref=?6, token=NULL,
             lease_expires_at_ms=NULL WHERE id=?1 AND status='running' AND worker_id=?2
             AND token=?3 AND attempt=?4 AND lease_expires_at_ms > ?7",
                params![
                    claim.work.id,
                    claim.worker_id,
                    claim.token,
                    claim.attempt,
                    status,
                    result_ref,
                    now
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(FactoryError::StaleClaim);
        }
        tx.commit().map_err(storage)
    }

    pub fn get(&self, id: &str) -> Result<Option<WorkRecord>, FactoryError> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            i64,
            Option<String>,
            Option<i64>,
            Option<String>,
        )> = self
            .connection
            .query_row(
                "SELECT id, mission_id, role, instructions, status, attempt, worker_id,
             lease_expires_at_ms, result_ref FROM engineering_work_v1 WHERE id=?1",
                [id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .optional()
            .map_err(storage)?;
        let Some((
            id,
            mission_id,
            role,
            instructions,
            status,
            attempt,
            worker_id,
            lease_expires_at_ms,
            result_ref,
        )) = row
        else {
            return Ok(None);
        };
        let mut stmt = self.connection.prepare("SELECT dependency_id FROM engineering_dependencies_v1 WHERE work_id=?1 ORDER BY dependency_id").map_err(storage)?;
        let depends_on = stmt
            .query_map([&id], |row| row.get(0))
            .map_err(storage)?
            .collect::<Result<_, _>>()
            .map_err(storage)?;
        Ok(Some(WorkRecord {
            spec: WorkSpec {
                id,
                mission_id,
                role,
                instructions,
                depends_on,
            },
            status: WorkStatus::parse(&status)?,
            attempt,
            worker_id,
            lease_expires_at_ms,
            result_ref,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn seed(store: &mut FactoryStore) {
        store
            .submit_mission(&MissionSpec {
                id: "m".into(),
                repository: "ocaf-runtime".into(),
                base_ref: "main".into(),
            })
            .unwrap();
    }
    fn work(id: &str, deps: &[&str]) -> WorkSpec {
        WorkSpec {
            id: id.into(),
            mission_id: "m".into(),
            role: "rust".into(),
            instructions: "Implement and test".into(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn independent_workers_claim_distinct_work_and_unlock_dependency() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("queue.sqlite");
        let mut a = FactoryStore::open(&path).unwrap();
        let mut b = FactoryStore::open(&path).unwrap();
        seed(&mut a);
        a.submit_work(&work("a", &[])).unwrap();
        a.submit_work(&work("b", &[])).unwrap();
        a.submit_work(&work("merge", &["a", "b"])).unwrap();
        let first = a.claim("worker-a", 10_000).unwrap().unwrap();
        let second = b.claim("worker-b", 10_000).unwrap().unwrap();
        assert_ne!(first.work.id, second.work.id);
        assert!(a.claim("worker-c", 10_000).unwrap().is_none());
        a.finish(&first, true, "commit-a").unwrap();
        assert!(b.claim("worker-c", 10_000).unwrap().is_none());
        b.finish(&second, true, "commit-b").unwrap();
        assert_eq!(
            a.claim("worker-c", 10_000).unwrap().unwrap().work.id,
            "merge"
        );
    }

    #[test]
    fn stale_and_expired_claims_cannot_commit() {
        let dir = tempdir().unwrap();
        let mut store = FactoryStore::open(dir.path().join("queue.sqlite")).unwrap();
        seed(&mut store);
        store.submit_work(&work("a", &[])).unwrap();
        let claim = store.claim("worker-a", 100).unwrap().unwrap();
        let mut forged = claim.clone();
        forged.token = "wrong".into();
        assert_eq!(
            store.finish(&forged, true, "commit").unwrap_err(),
            FactoryError::StaleClaim
        );
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(
            store.finish(&claim, true, "commit").unwrap_err(),
            FactoryError::StaleClaim
        );
        assert!(store.claim("worker-b", 100).unwrap().is_none());
        assert_eq!(
            store.get("a").unwrap().unwrap().status,
            WorkStatus::Uncertain
        );
    }

    #[test]
    fn rejects_cross_mission_dependency_and_conflicting_replay() {
        let dir = tempdir().unwrap();
        let mut store = FactoryStore::open(dir.path().join("queue.sqlite")).unwrap();
        seed(&mut store);
        let item = work("a", &[]);
        store.submit_work(&item).unwrap();
        store.submit_work(&item).unwrap();
        let mut changed = item.clone();
        changed.instructions = "other".into();
        assert_eq!(
            store.submit_work(&changed).unwrap_err(),
            FactoryError::Conflict
        );
        store
            .submit_mission(&MissionSpec {
                id: "other".into(),
                repository: "repo".into(),
                base_ref: "main".into(),
            })
            .unwrap();
        let mut cross = work("cross", &["a"]);
        cross.mission_id = "other".into();
        assert_eq!(
            store.submit_work(&cross).unwrap_err(),
            FactoryError::Missing
        );
    }
}
