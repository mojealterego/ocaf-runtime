//! Durable, tenant-scoped collaborative object journal (Argon source 2.02).
//! Actor identity MUST be bound by authenticated ingress. A hash chain is
//! corruption evidence, not truncation resistance without a trusted pinned head.
use crate::collaboration_tracker::{
    CognitiveOperation, CollaborationEvent, CollaborationTracker, TrackerError, TrustedActor,
};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;

const GENESIS: &str = "OCAF-COLLAB-GENESIS-v1";
const MAX_OBJECTS: i64 = 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollaborationHead {
    pub revision: u64,
    pub sha256: String,
}

impl Default for CollaborationHead {
    fn default() -> Self {
        Self {
            revision: 0,
            sha256: GENESIS.into(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CollaborationStoreError {
    #[error("invalid tenant or object")]
    Invalid,
    #[error("collaboration store unavailable")]
    Database,
    #[error("provenance integrity check failed")]
    Integrity,
    #[error("stale or mismatched externally pinned head")]
    Conflict,
    #[error("collaboration history limit exceeded")]
    Limit,
    #[error("collaboration event rejected: {0}")]
    Event(#[from] TrackerError),
}

pub struct SqliteCollaborationStore {
    path: PathBuf,
}

fn wire(operation: CognitiveOperation) -> &'static str {
    match operation {
        CognitiveOperation::Propose => "propose",
        CognitiveOperation::Modify => "modify",
        CognitiveOperation::Validate => "validate",
        CognitiveOperation::Challenge => "challenge",
        CognitiveOperation::Synthesize => "synthesize",
    }
}

fn parse_operation(input: &str) -> Result<CognitiveOperation, CollaborationStoreError> {
    match input {
        "propose" => Ok(CognitiveOperation::Propose),
        "modify" => Ok(CognitiveOperation::Modify),
        "validate" => Ok(CognitiveOperation::Validate),
        "challenge" => Ok(CognitiveOperation::Challenge),
        "synthesize" => Ok(CognitiveOperation::Synthesize),
        _ => Err(CollaborationStoreError::Integrity),
    }
}

fn digest(tenant: &str, object: &str, event: &CollaborationEvent, previous: &str) -> String {
    let mut sha = Sha256::new();
    sha.update(b"OCAF-COLLABORATION-EVENT-v1");
    let revision = event.revision.to_be_bytes();
    let parts: [&[u8]; 8] = [
        tenant.as_bytes(),
        object.as_bytes(),
        &revision,
        event.actor.as_bytes(),
        wire(event.operation).as_bytes(),
        event.focus.as_bytes(),
        event.artifact_sha256.as_bytes(),
        previous.as_bytes(),
    ];
    for part in parts {
        sha.update((part.len() as u64).to_be_bytes());
        sha.update(part);
    }
    format!("{:x}", sha.finalize())
}

impl SqliteCollaborationStore {
    pub fn open_existing(path: &Path) -> Result<Self, CollaborationStoreError> {
        if !path.is_absolute() || !path.is_file() {
            return Err(CollaborationStoreError::Database);
        }
        let store = Self { path: path.into() };
        let mut conn = store.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CollaborationStoreError::Database)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS ocaf_collaboration_v1 (
                tenant TEXT NOT NULL, object TEXT NOT NULL,
                revision INTEGER NOT NULL, actor TEXT NOT NULL,
                operation TEXT NOT NULL, focus TEXT NOT NULL,
                artifact_sha256 TEXT NOT NULL,
                previous TEXT NOT NULL, digest TEXT NOT NULL,
                PRIMARY KEY (tenant, object, revision)
            );
            CREATE TABLE IF NOT EXISTS ocaf_collaboration_schema (
                subsystem TEXT PRIMARY KEY, version INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO ocaf_collaboration_schema(subsystem,version)
                VALUES ('collaboration',1);",
        )
        .map_err(|_| CollaborationStoreError::Database)?;
        let version: i64 = tx
            .query_row(
                "SELECT version FROM ocaf_collaboration_schema WHERE subsystem='collaboration'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| CollaborationStoreError::Integrity)?;
        if version != 1 {
            return Err(CollaborationStoreError::Integrity);
        }
        tx.commit().map_err(|_| CollaborationStoreError::Database)?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, CollaborationStoreError> {
        let c = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| CollaborationStoreError::Database)?;
        c.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| CollaborationStoreError::Database)?;
        c.execute_batch("PRAGMA synchronous=FULL;")
            .map_err(|_| CollaborationStoreError::Database)?;
        Ok(c)
    }

    fn checked_connection(&self) -> Result<Connection, CollaborationStoreError> {
        let c = self.connection()?;
        let version: i64 = c
            .query_row(
                "SELECT version FROM ocaf_collaboration_schema WHERE subsystem='collaboration'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| CollaborationStoreError::Integrity)?;
        if version != 1 {
            return Err(CollaborationStoreError::Integrity);
        }
        Ok(c)
    }

    /// The caller MUST authenticate the tenant before providing this identity.
    pub fn load(
        &self,
        tenant: &str,
        object: &str,
        expected: Option<&CollaborationHead>,
    ) -> Result<(Vec<CollaborationEvent>, CollaborationHead), CollaborationStoreError> {
        let mut conn = self.checked_connection()?;
        let tx = conn
            .transaction()
            .map_err(|_| CollaborationStoreError::Database)?;
        let (events, head) = Self::replay(&tx, tenant, object)?;
        if expected.is_some_and(|pin| pin != &head) {
            return Err(CollaborationStoreError::Conflict);
        }
        Ok((events, head))
    }

    fn replay(
        conn: &Connection,
        tenant: &str,
        object: &str,
    ) -> Result<(Vec<CollaborationEvent>, CollaborationHead), CollaborationStoreError> {
        let mut tracker = CollaborationTracker::default();
        tracker.history(tenant, object)?;
        let mut query = conn
            .prepare(
                "SELECT revision,actor,operation,focus,artifact_sha256,previous,digest
                 FROM ocaf_collaboration_v1 WHERE tenant=?1 AND object=?2
                 ORDER BY revision",
            )
            .map_err(|_| CollaborationStoreError::Database)?;
        let mut rows = query
            .query(params![tenant, object])
            .map_err(|_| CollaborationStoreError::Database)?;
        let mut head = CollaborationHead::default();
        while let Some(row) = rows.next().map_err(|_| CollaborationStoreError::Database)? {
            let revision: i64 = row.get(0).map_err(|_| CollaborationStoreError::Integrity)?;
            let revision =
                u64::try_from(revision).map_err(|_| CollaborationStoreError::Integrity)?;
            let actor: String = row.get(1).map_err(|_| CollaborationStoreError::Integrity)?;
            let operation: String = row.get(2).map_err(|_| CollaborationStoreError::Integrity)?;
            let focus: String = row.get(3).map_err(|_| CollaborationStoreError::Integrity)?;
            let artifact: String = row.get(4).map_err(|_| CollaborationStoreError::Integrity)?;
            let previous: String = row.get(5).map_err(|_| CollaborationStoreError::Integrity)?;
            let hash: String = row.get(6).map_err(|_| CollaborationStoreError::Integrity)?;
            let event = CollaborationEvent {
                revision,
                actor,
                operation: parse_operation(&operation)?,
                focus,
                artifact_sha256: artifact,
            };
            if revision != head.revision + 1
                || previous != head.sha256
                || digest(tenant, object, &event, &previous) != hash
            {
                return Err(CollaborationStoreError::Integrity);
            }
            let identity = TrustedActor {
                tenant: tenant.into(),
                actor: event.actor.clone(),
            };
            let observed = tracker
                .append(
                    &identity,
                    object,
                    head.revision,
                    event.operation,
                    &event.focus,
                    &event.artifact_sha256,
                )
                .map_err(|_| CollaborationStoreError::Integrity)?;
            if observed != revision {
                return Err(CollaborationStoreError::Integrity);
            }
            head = CollaborationHead {
                revision,
                sha256: hash,
            };
        }
        Ok((tracker.history(tenant, object)?, head))
    }

    /// Atomic compare-and-append with replay validation and monotonically fenced revision.
    pub fn append(
        &self,
        identity: &TrustedActor,
        object: &str,
        expected: &CollaborationHead,
        operation: CognitiveOperation,
        focus: &str,
        artifact_sha256: &str,
    ) -> Result<CollaborationHead, CollaborationStoreError> {
        let mut conn = self.checked_connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CollaborationStoreError::Database)?;
        let (history, head) = Self::replay(&tx, &identity.tenant, object)?;
        if &head != expected {
            return Err(CollaborationStoreError::Conflict);
        }
        if head.revision == 0 {
            let objects: i64 = tx
                .query_row(
                    "SELECT count(*) FROM (
                        SELECT 1 FROM ocaf_collaboration_v1
                        WHERE tenant=?1 GROUP BY object
                     )",
                    params![identity.tenant],
                    |row| row.get(0),
                )
                .map_err(|_| CollaborationStoreError::Database)?;
            if objects >= MAX_OBJECTS {
                return Err(CollaborationStoreError::Limit);
            }
        }
        let mut state = CollaborationTracker::default();
        for saved in &history {
            let saved_identity = TrustedActor {
                tenant: identity.tenant.clone(),
                actor: saved.actor.clone(),
            };
            state
                .append(
                    &saved_identity,
                    object,
                    saved.revision - 1,
                    saved.operation,
                    &saved.focus,
                    &saved.artifact_sha256,
                )
                .map_err(|_| CollaborationStoreError::Integrity)?;
        }
        let revision = state.append(
            identity,
            object,
            head.revision,
            operation,
            focus,
            artifact_sha256,
        )?;
        let event = state
            .history(&identity.tenant, object)?
            .pop()
            .ok_or(CollaborationStoreError::Integrity)?;
        let hash = digest(&identity.tenant, object, &event, &head.sha256);
        tx.execute(
            "INSERT INTO ocaf_collaboration_v1
             (tenant,object,revision,actor,operation,focus,artifact_sha256,previous,digest)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                identity.tenant,
                object,
                revision,
                event.actor,
                wire(event.operation),
                event.focus,
                event.artifact_sha256,
                head.sha256,
                hash,
            ],
        )
        .map_err(|_| CollaborationStoreError::Database)?;
        tx.commit().map_err(|_| CollaborationStoreError::Database)?;
        Ok(CollaborationHead {
            revision,
            sha256: hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(tenant: &str) -> TrustedActor {
        TrustedActor {
            tenant: tenant.into(),
            actor: "agent1".into(),
        }
    }

    #[test]
    fn restart_fencing_and_tenant_boundaries() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let store = SqliteCollaborationStore::open_existing(file.path()).unwrap();
        let hash = "a".repeat(64);
        let head1 = store
            .append(
                &actor("tenant-a"),
                "object1",
                &CollaborationHead::default(),
                CognitiveOperation::Propose,
                "design",
                &hash,
            )
            .unwrap();
        assert_eq!(head1.revision, 1);
        let reopened = SqliteCollaborationStore::open_existing(file.path()).unwrap();
        assert_eq!(
            reopened
                .load("tenant-a", "object1", Some(&head1))
                .unwrap()
                .0
                .len(),
            1
        );
        assert!(reopened
            .load("tenant-b", "object1", None)
            .unwrap()
            .0
            .is_empty());
        assert!(matches!(
            reopened.append(
                &actor("tenant-a"),
                "object1",
                &CollaborationHead::default(),
                CognitiveOperation::Modify,
                "design",
                &hash,
            ),
            Err(CollaborationStoreError::Conflict)
        ));
        let head2 = reopened
            .append(
                &actor("tenant-a"),
                "object1",
                &head1,
                CognitiveOperation::Validate,
                "check",
                &hash,
            )
            .unwrap();
        assert_eq!(
            reopened
                .load("tenant-a", "object1", Some(&head2))
                .unwrap()
                .0
                .len(),
            2
        );
        assert!(matches!(
            reopened.load("tenant-b", "object1", Some(&head2)),
            Err(CollaborationStoreError::Conflict)
        ));
    }

    #[test]
    fn tampering_and_truncation_fail_closed() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let store = SqliteCollaborationStore::open_existing(file.path()).unwrap();
        let head = store
            .append(
                &actor("tenant-a"),
                "object1",
                &CollaborationHead::default(),
                CognitiveOperation::Challenge,
                "focus",
                &"b".repeat(64),
            )
            .unwrap();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute("UPDATE ocaf_collaboration_v1 SET focus='forged'", [])
            .unwrap();
        assert!(matches!(
            store.load("tenant-a", "object1", None),
            Err(CollaborationStoreError::Integrity)
        ));
        connection
            .execute("DELETE FROM ocaf_collaboration_v1", [])
            .unwrap();
        assert!(matches!(
            store.load("tenant-a", "object1", Some(&head)),
            Err(CollaborationStoreError::Conflict)
        ));
    }
}
