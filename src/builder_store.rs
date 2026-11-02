//! Stage 0.50: tenant-scoped, versioned SQLite storage for Builder graph projects.
//!
//! The trusted tenant MUST originate from authenticated ingress. This store
//! does not authenticate users, run agent tools, or publish artifacts. Hash
//! chaining detects modification, while externally pinned heads are required
//! to detect deletion/truncation of the latest revisions.
use crate::builder_core::{from_dsl, to_dsl, BuilderError, BuilderProject};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;

const GENESIS: &str = "OCAF-BUILDER-GENESIS-v1";
const MAX_REVISIONS: usize = 128;
const MAX_PAYLOAD_BYTES: usize = 128 * 1024;
const MAX_TOTAL_BYTES: usize = MAX_REVISIONS * MAX_PAYLOAD_BYTES;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuilderHead {
    pub revision: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BuilderStoreError {
    #[error("invalid tenant or project identifier")]
    Invalid,
    #[error("authenticated tenant does not own this project")]
    Denied,
    #[error("stale head, unexpected project or revision gap")]
    Conflict,
    #[error("builder project revision chain integrity rejected")]
    Integrity,
    #[error("builder database unavailable")]
    Database,
    #[error("builder history or payload exceeds a hard resource limit")]
    Limit,
    #[error("builder graph rejected: {0}")]
    Builder(#[from] BuilderError),
}

pub struct SqliteBuilderStore {
    path: PathBuf,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

fn check_identity(tenant: &str, project: &str) -> Result<(), BuilderStoreError> {
    if !valid_id(tenant) || !valid_id(project) {
        return Err(BuilderStoreError::Invalid);
    }
    Ok(())
}

fn digest(tenant: &str, project: &str, revision: u64, previous: &str, payload: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"OCAF-BUILDER-REVISION-v1");
    let revision_bytes = revision.to_be_bytes();
    for part in [
        tenant.as_bytes(),
        project.as_bytes(),
        &revision_bytes,
        previous.as_bytes(),
        payload,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    format!("{:x}", hash.finalize())
}

impl SqliteBuilderStore {
    /// Open a pre-provisioned absolute regular SQLite file; never create a
    /// database from an untrusted project path or follow a symlink.
    pub fn open_existing(path: &Path) -> Result<Self, BuilderStoreError> {
        if !path.is_absolute() || !path.is_file() {
            return Err(BuilderStoreError::Database);
        }
        let store = Self {
            path: path.to_path_buf(),
        };
        let mut connection = store.connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| BuilderStoreError::Database)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS ocaf_builder_schema (
                subsystem TEXT PRIMARY KEY, version INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO ocaf_builder_schema(subsystem,version)
                VALUES ('builder_projects',1);
            CREATE TABLE IF NOT EXISTS ocaf_builder_revisions_v1 (
                tenant TEXT NOT NULL, project TEXT NOT NULL,
                revision INTEGER NOT NULL CHECK (revision > 0),
                previous TEXT NOT NULL, digest TEXT NOT NULL,
                payload BLOB NOT NULL,
                PRIMARY KEY (tenant,project,revision)
            );",
        )
        .map_err(|_| BuilderStoreError::Database)?;
        let version: i64 = tx
            .query_row(
                "SELECT version FROM ocaf_builder_schema WHERE subsystem='builder_projects'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| BuilderStoreError::Integrity)?;
        if version != 1 {
            return Err(BuilderStoreError::Integrity);
        }
        tx.commit().map_err(|_| BuilderStoreError::Database)?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, BuilderStoreError> {
        let connection = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| BuilderStoreError::Database)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| BuilderStoreError::Database)?;
        connection
            .execute_batch("PRAGMA synchronous=FULL;")
            .map_err(|_| BuilderStoreError::Database)?;
        Ok(connection)
    }

    fn checked_connection(&self) -> Result<Connection, BuilderStoreError> {
        let connection = self.connection()?;
        let version: i64 = connection
            .query_row(
                "SELECT version FROM ocaf_builder_schema WHERE subsystem='builder_projects'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| BuilderStoreError::Integrity)?;
        if version != 1 {
            return Err(BuilderStoreError::Integrity);
        }
        Ok(connection)
    }

    fn replay(
        connection: &Connection,
        tenant: &str,
        project: &str,
    ) -> Result<Vec<(BuilderProject, BuilderHead)>, BuilderStoreError> {
        let mut statement = connection
            .prepare(
                "SELECT revision,previous,digest,payload
                 FROM ocaf_builder_revisions_v1
                 WHERE tenant=?1 AND project=?2 ORDER BY revision",
            )
            .map_err(|_| BuilderStoreError::Database)?;
        let mut rows = statement
            .query(params![tenant, project])
            .map_err(|_| BuilderStoreError::Database)?;
        let mut result = Vec::new();
        let mut previous = GENESIS.to_owned();
        let mut total = 0_usize;
        while let Some(row) = rows.next().map_err(|_| BuilderStoreError::Database)? {
            let revision: i64 = row.get(0).map_err(|_| BuilderStoreError::Integrity)?;
            let stored_previous: String = row.get(1).map_err(|_| BuilderStoreError::Integrity)?;
            let stored_digest: String = row.get(2).map_err(|_| BuilderStoreError::Integrity)?;
            let payload: Vec<u8> = row.get(3).map_err(|_| BuilderStoreError::Integrity)?;
            if result.len() >= MAX_REVISIONS || payload.len() > MAX_PAYLOAD_BYTES {
                return Err(BuilderStoreError::Limit);
            }
            total = total.checked_add(payload.len()).ok_or(BuilderStoreError::Limit)?;
            if total > MAX_TOTAL_BYTES {
                return Err(BuilderStoreError::Limit);
            }
            let expected_revision = result.len() as u64 + 1;
            if revision != expected_revision as i64 || stored_previous != previous {
                return Err(BuilderStoreError::Integrity);
            }
            if digest(tenant, project, expected_revision, &previous, &payload) != stored_digest {
                return Err(BuilderStoreError::Integrity);
            }
            let source = std::str::from_utf8(&payload).map_err(|_| BuilderStoreError::Integrity)?;
            let snapshot = from_dsl(source, tenant).map_err(|_| BuilderStoreError::Integrity)?;
            if snapshot.project_id != project || snapshot.revision != expected_revision {
                return Err(BuilderStoreError::Integrity);
            }
            previous = stored_digest.clone();
            result.push((
                snapshot,
                BuilderHead {
                    revision: expected_revision,
                    sha256: stored_digest,
                },
            ));
        }
        Ok(result)
    }

    /// Return the latest snapshot. An expected externally pinned head detects
    /// stale reads and deleted/truncated revisions.
    pub fn load(
        &self,
        trusted_tenant: &str,
        project_id: &str,
        expected: Option<&BuilderHead>,
    ) -> Result<Option<(BuilderProject, BuilderHead)>, BuilderStoreError> {
        check_identity(trusted_tenant, project_id)?;
        let mut connection = self.checked_connection()?;
        let tx = connection
            .transaction()
            .map_err(|_| BuilderStoreError::Database)?;
        let snapshots = Self::replay(&tx, trusted_tenant, project_id)?;
        let latest = snapshots.last().cloned();
        if expected.is_some_and(|pin| latest.as_ref().map(|(_, head)| head) != Some(pin)) {
            return Err(BuilderStoreError::Conflict);
        }
        tx.commit().map_err(|_| BuilderStoreError::Database)?;
        Ok(latest)
    }

    /// Append one validated project snapshot with optimistic concurrency.
    /// expected=None means create-only; an existing project is never replaced.
    pub fn save(
        &self,
        project: &BuilderProject,
        trusted_tenant: &str,
        expected: Option<&BuilderHead>,
    ) -> Result<BuilderHead, BuilderStoreError> {
        if project.tenant_id != trusted_tenant {
            return Err(BuilderStoreError::Denied);
        }
        check_identity(trusted_tenant, &project.project_id)?;
        let source = to_dsl(project, trusted_tenant)?;
        if source.len() > MAX_PAYLOAD_BYTES {
            return Err(BuilderStoreError::Limit);
        }
        let mut connection = self.checked_connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| BuilderStoreError::Database)?;
        let snapshots = Self::replay(&tx, trusted_tenant, &project.project_id)?;
        let current = snapshots.last().map(|(_, head)| head);
        if current != expected {
            return Err(BuilderStoreError::Conflict);
        }
        if snapshots.len() >= MAX_REVISIONS || project.revision != snapshots.len() as u64 + 1 {
            return Err(if snapshots.len() >= MAX_REVISIONS {
                BuilderStoreError::Limit
            } else {
                BuilderStoreError::Conflict
            });
        }
        let previous = current.map_or(GENESIS, |head| head.sha256.as_str());
        let head = BuilderHead {
            revision: project.revision,
            sha256: digest(
                trusted_tenant,
                &project.project_id,
                project.revision,
                previous,
                source.as_bytes(),
            ),
        };
        tx.execute(
            "INSERT INTO ocaf_builder_revisions_v1
             (tenant,project,revision,previous,digest,payload)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                trusted_tenant,
                project.project_id,
                project.revision as i64,
                previous,
                head.sha256,
                source.as_bytes(),
            ],
        )
        .map_err(|_| BuilderStoreError::Database)?;
        tx.commit().map_err(|_| BuilderStoreError::Database)?;
        Ok(head)
    }

    /// Read fully verified revision heads. The caller must supply trusted tenant.
    pub fn history(
        &self,
        trusted_tenant: &str,
        project_id: &str,
    ) -> Result<Vec<BuilderHead>, BuilderStoreError> {
        check_identity(trusted_tenant, project_id)?;
        let mut connection = self.checked_connection()?;
        let tx = connection
            .transaction()
            .map_err(|_| BuilderStoreError::Database)?;
        let snapshots = Self::replay(&tx, trusted_tenant, project_id)?;
        tx.commit().map_err(|_| BuilderStoreError::Database)?;
        Ok(snapshots.into_iter().map(|(_, head)| head).collect())
    }
}
