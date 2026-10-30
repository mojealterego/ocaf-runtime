//! PDF 1.19–1.22: versioned SQLite event driver, pure replay and scoped links.
//! Hash chains detect corruption; trusted head receipts are needed to detect truncation.
use crate::document_state::{DocumentState, LabelPath, Mutation, StateError};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

const MAX_EVENTS: u64 = 4096;
const MAX_EVENT_BYTES: usize = 5 * 1024 * 1024;
const MAX_LOG_BYTES: usize = 32 * 1024 * 1024;
const GENESIS: &str = "GENESIS-v1";
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("invalid document identity or event")]
    Invalid,
    #[error("document database unavailable")]
    Database,
    #[error("event chain integrity or version rejected")]
    Integrity,
    #[error("stale document head")]
    Conflict,
    #[error("document limit exceeded")]
    Limit,
    #[error("document mutation rejected: {0}")]
    State(#[from] StateError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentKey {
    tenant: String,
    document: String,
}
impl DocumentKey {
    pub fn new(tenant: &str, document: &str) -> Result<Self, StoreError> {
        for s in [tenant, document] {
            if s.is_empty()
                || s.len() > 128
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            {
                return Err(StoreError::Invalid);
            }
        }
        Ok(Self {
            tenant: tenant.into(),
            document: document.into(),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub revision: u64,
    pub sha256: String,
}
impl Default for Head {
    fn default() -> Self {
        Self {
            revision: 0,
            sha256: GENESIS.into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum Change {
    Create {
        label: String,
    },
    Attach {
        label: String,
        guid: String,
        payload: Vec<u8>,
    },
    Update {
        label: String,
        guid: String,
        payload: Vec<u8>,
    },
    Forget {
        label: String,
        guid: String,
    },
    Remember {
        label: String,
        guid: String,
    },
}
impl Change {
    fn mutation(&self) -> Result<Mutation, StoreError> {
        let guid = |s: &str| Uuid::parse_str(s).map_err(|_| StoreError::Invalid);
        Ok(match self {
            Self::Create { label } => Mutation::CreateLabel(LabelPath::parse(label)?),
            Self::Attach {
                label,
                guid: id,
                payload,
            } => Mutation::Attach {
                label: LabelPath::parse(label)?,
                guid: guid(id)?,
                payload: payload.clone(),
            },
            Self::Update { label, guid: id, payload } => Mutation::Update {
                label: LabelPath::parse(label)?,
                guid: guid(id)?,
                payload: payload.clone(),
            },
            Self::Forget { label, guid: id } => Mutation::Forget {
                label: LabelPath::parse(label)?,
                guid: guid(id)?,
            },
            Self::Remember { label, guid: id } => Mutation::Remember {
                label: LabelPath::parse(label)?,
                guid: guid(id)?,
            },
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Event {
    Commit { name: String, changes: Vec<Change> },
    Undo,
    Redo,
    ClearHistory,
}
impl Event {
    fn apply(&self, state: &mut DocumentState) -> Result<(), StoreError> {
        match self {
            Self::Commit { name, changes } => {
                if changes.len() > 1024 {
                    return Err(StoreError::Limit);
                }
                let mutations = changes
                    .iter()
                    .map(Change::mutation)
                    .collect::<Result<Vec<_>, _>>()?;
                state.commit(name, &mutations)?;
            }
            Self::Undo => {
                state.undo()?;
            }
            Self::Redo => {
                state.redo()?;
            }
            Self::ClearHistory => state.clear_history(),
        }
        Ok(())
    }
}
/// A driver returns reconstructed state only; replay has no network/process callbacks.
pub trait DocumentDriver {
    fn load(
        &self,
        key: &DocumentKey,
        expected: Option<&Head>,
    ) -> Result<(DocumentState, Head), StoreError>;
    fn append(&self, key: &DocumentKey, expected: &Head, event: &Event)
        -> Result<Head, StoreError>;
}
pub struct SqliteDocumentDriver {
    path: PathBuf,
}
impl SqliteDocumentDriver {
    pub fn open_existing(path: &Path) -> Result<Self, StoreError> {
        if !path.is_absolute() || !path.is_file() {
            return Err(StoreError::Database);
        }
        let driver = Self { path: path.into() };
        driver.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS document_events_v1 (
            tenant TEXT NOT NULL, document TEXT NOT NULL, revision INTEGER NOT NULL,
            version INTEGER NOT NULL, previous TEXT NOT NULL, digest TEXT NOT NULL, payload BLOB NOT NULL,
            PRIMARY KEY(tenant, document, revision));").map_err(|_| StoreError::Database)?;
        Ok(driver)
    }
    fn connection(&self) -> Result<Connection, StoreError> {
        let c = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| StoreError::Database)?;
        c.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| StoreError::Database)?;
        c.execute_batch("PRAGMA synchronous=FULL;")
            .map_err(|_| StoreError::Database)?;
        Ok(c)
    }
}
fn digest(key: &DocumentKey, revision: u64, previous: &str, payload: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"OCAF-DOCUMENT-EVENT-v1");
    for bytes in [
        key.tenant.as_bytes(),
        key.document.as_bytes(),
        &revision.to_be_bytes(),
        previous.as_bytes(),
        payload,
    ] {
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    }
    format!("{:x}", h.finalize())
}
fn replay(c: &Connection, key: &DocumentKey) -> Result<(DocumentState, Head, usize), StoreError> {
    let mut q = c.prepare("SELECT revision, version, previous, digest, length(payload) FROM document_events_v1 WHERE tenant=?1 AND document=?2 ORDER BY revision").map_err(|_| StoreError::Database)?;
    let mut rows = q
        .query(params![key.tenant, key.document])
        .map_err(|_| StoreError::Database)?;
    let mut state = DocumentState::default();
    let mut head = Head::default();
    let mut total = 0;
    while let Some(row) = rows.next().map_err(|_| StoreError::Database)? {
        let rev: u64 = row.get(0).map_err(|_| StoreError::Integrity)?;
        let version: u32 = row.get(1).map_err(|_| StoreError::Integrity)?;
        let previous: String = row.get(2).map_err(|_| StoreError::Integrity)?;
        let hash: String = row.get(3).map_err(|_| StoreError::Integrity)?;
        let len: usize = row.get(4).map_err(|_| StoreError::Integrity)?;
        if rev != head.revision + 1 || version != 1 || previous != head.sha256 {
            return Err(StoreError::Integrity);
        }
        if rev > MAX_EVENTS || len > MAX_EVENT_BYTES || total + len > MAX_LOG_BYTES {
            return Err(StoreError::Limit);
        }
        let payload: Vec<u8> = c.query_row("SELECT payload FROM document_events_v1 WHERE tenant=?1 AND document=?2 AND revision=?3",params![key.tenant,key.document,rev],|r|r.get(0)).map_err(|_| StoreError::Database)?;
        if digest(key, rev, &previous, &payload) != hash {
            return Err(StoreError::Integrity);
        }
        let event: Event = serde_json::from_slice(&payload).map_err(|_| StoreError::Integrity)?;
        event.apply(&mut state)?;
        total += len;
        head = Head {
            revision: rev,
            sha256: hash,
        };
    }
    Ok((state, head, total))
}
impl DocumentDriver for SqliteDocumentDriver {
    fn load(
        &self,
        key: &DocumentKey,
        expected: Option<&Head>,
    ) -> Result<(DocumentState, Head), StoreError> {
        let mut c = self.connection()?;
        let tx = c.transaction().map_err(|_| StoreError::Database)?;
        let (state, head, _) = replay(&tx, key)?;
        if expected.is_some_and(|h| h != &head) {
            return Err(StoreError::Conflict);
        }
        Ok((state, head))
    }
    fn append(
        &self,
        key: &DocumentKey,
        expected: &Head,
        event: &Event,
    ) -> Result<Head, StoreError> {
        let payload = serde_json::to_vec(event).map_err(|_| StoreError::Invalid)?;
        if payload.len() > MAX_EVENT_BYTES {
            return Err(StoreError::Limit);
        }
        let mut c = self.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Database)?;
        let (mut state, head, total) = replay(&tx, key)?;
        if &head != expected {
            return Err(StoreError::Conflict);
        }
        if head.revision >= MAX_EVENTS || total + payload.len() > MAX_LOG_BYTES {
            return Err(StoreError::Limit);
        }
        event.apply(&mut state)?;
        let next = Head {
            revision: head.revision + 1,
            sha256: digest(key, head.revision + 1, &head.sha256, &payload),
        };
        tx.execute(
            "INSERT INTO document_events_v1 VALUES (?1,?2,?3,1,?4,?5,?6)",
            params![
                key.tenant,
                key.document,
                next.revision,
                head.sha256,
                next.sha256,
                payload
            ],
        )
        .map_err(|_| StoreError::Database)?;
        tx.commit().map_err(|_| StoreError::Database)?;
        Ok(next)
    }
}
/// Caller obtains both keys from its authenticated tenant mapping. Pin target head
/// to prevent a link silently following a different document version.
pub fn resolve_link(
    driver: &impl DocumentDriver,
    source: &DocumentKey,
    source_label: &LabelPath,
    target: &DocumentKey,
    target_label: &LabelPath,
    target_head: &Head,
) -> Result<(), StoreError> {
    if source.tenant != target.tenant {
        return Err(StoreError::Invalid);
    }
    let (s, _) = driver.load(source, None)?;
    let (t, _) = driver.load(target, Some(target_head))?;
    if !s.contains_label(source_label) || !t.contains_label(target_label) {
        return Err(StoreError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::NamedTempFile, SqliteDocumentDriver, DocumentKey) {
        let f = tempfile::NamedTempFile::new().unwrap();
        let d = SqliteDocumentDriver::open_existing(f.path()).unwrap();
        (f, d, DocumentKey::new("tenant", "doc").unwrap())
    }
    fn create() -> Event {
        Event::Commit {
            name: "create".into(),
            changes: vec![Change::Create {
                label: "0:1".into(),
            }],
        }
    }
    #[test]
    fn restart_undo_redo_and_stale_writer() {
        let (f, d, key) = setup();
        let first = d.append(&key, &Head::default(), &create()).unwrap();
        assert!(matches!(
            d.append(&key, &Head::default(), &create()),
            Err(StoreError::Conflict)
        ));
        let second = d.append(&key, &first, &Event::Undo).unwrap();
        let reopened = SqliteDocumentDriver::open_existing(f.path()).unwrap();
        let label = LabelPath::parse("0:1").unwrap();
        assert!(!reopened
            .load(&key, Some(&second))
            .unwrap()
            .0
            .contains_label(&label));
        let third = reopened.append(&key, &second, &Event::Redo).unwrap();
        assert!(reopened
            .load(&key, Some(&third))
            .unwrap()
            .0
            .contains_label(&label));
        assert!(reopened.append(&key, &third, &create()).is_err());
        assert_eq!(reopened.load(&key, None).unwrap().1, third);
    }
    #[test]
    fn tamper_and_truncation_rejected() {
        let (f, d, key) = setup();
        let head = d.append(&key, &Head::default(), &create()).unwrap();
        let c = Connection::open(f.path()).unwrap();
        c.execute("UPDATE document_events_v1 SET digest='forged'", [])
            .unwrap();
        assert!(matches!(d.load(&key, None), Err(StoreError::Integrity)));
        c.execute("DELETE FROM document_events_v1", []).unwrap();
        assert!(matches!(
            d.load(&key, Some(&head)),
            Err(StoreError::Conflict)
        ));
    }
    #[test]
    fn links_require_same_tenant_existing_labels_and_pinned_head() {
        let (_file, d, key) = setup();
        let head = d.append(&key, &Head::default(), &create()).unwrap();
        let root = LabelPath::parse("0").unwrap();
        let child = LabelPath::parse("0:1").unwrap();
        resolve_link(&d, &key, &root, &key, &child, &head).unwrap();
        let other = DocumentKey::new("other", "doc").unwrap();
        assert!(resolve_link(&d, &key, &root, &other, &child, &head).is_err());
        assert!(resolve_link(&d, &key, &root, &key, &child, &Head::default()).is_err());
        assert!(resolve_link(
            &d,
            &key,
            &root,
            &key,
            &LabelPath::parse("0:9").unwrap(),
            &head
        )
        .is_err());
    }
}
