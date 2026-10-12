//! Replay admission controls. The durable backend commits a request claim to
//! SQLite before the tool executor begins; a restart cannot accept that ID
//! again while its bounded validity window remains active.
//!
//! This is at-most-once *attempt admission*, NOT exactly-once side effects.
//! Failures and crashes can leave the operation's outcome unknown.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{params, Connection, TransactionBehavior};

use super::RpcError;
use crate::cluster::replay::PgReplay;

const MAX_REPLAY_ENTRIES: i64 = 4_096;

pub(super) enum ReplayBackend {
    Volatile(Mutex<HashMap<(String, String), u64>>),
    Durable(Arc<SqliteReplayLedger>),
    Postgres(PgReplay),
}

impl ReplayBackend {
    pub(super) fn volatile() -> Self {
        Self::Volatile(Mutex::new(HashMap::new()))
    }

    pub(super) fn sqlite(path: &Path) -> Result<Self, RpcError> {
        Ok(Self::Durable(Arc::new(SqliteReplayLedger::open(path)?)))
    }

    pub(super) fn postgres(db: PgReplay) -> Self {
        Self::Postgres(db)
    }

    pub(super) async fn reserve(
        &self,
        node: &str,
        sender: &str,
        request_id: &str,
        expires_at_ms: u64,
        now_ms: u64,
    ) -> Result<(), RpcError> {
        match self {
            Self::Postgres(db) => {
                db.reserve(node, sender, request_id, expires_at_ms, now_ms)
                    .await
            }
            Self::Volatile(seen) => {
                let mut seen = seen.lock().map_err(|_| RpcError::Unavailable)?;
                seen.retain(|_, expiry| *expiry >= now_ms);
                let key = (sender.to_owned(), request_id.to_owned());
                if seen.contains_key(&key) {
                    return Err(RpcError::Replay);
                }
                if seen.len() >= MAX_REPLAY_ENTRIES as usize {
                    return Err(RpcError::Busy);
                }
                seen.insert(key, expires_at_ms);
                Ok(())
            }
            Self::Durable(store) => {
                let store = Arc::clone(store);
                let node = node.to_owned();
                let sender = sender.to_owned();
                let request_id = request_id.to_owned();
                tokio::task::spawn_blocking(move || {
                    store.reserve(&node, &sender, &request_id, expires_at_ms, now_ms)
                })
                .await
                .map_err(|_| RpcError::Unavailable)?
            }
        }
    }
}

pub(super) struct SqliteReplayLedger {
    connection: Mutex<Connection>,
}

impl SqliteReplayLedger {
    fn open(path: &Path) -> Result<Self, RpcError> {
        // Opens an explicitly named on-disk file. No database auto-fallback,
        // in-memory substitution or implicit parent-directory creation.
        let connection = Connection::open(path).map_err(|_| RpcError::Unavailable)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|_| RpcError::Unavailable)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 CREATE TABLE IF NOT EXISTS rpc_replay_v1 (
                     node TEXT NOT NULL,
                     sender TEXT NOT NULL,
                     request_id TEXT NOT NULL,
                     expires_at_ms INTEGER NOT NULL,
                     PRIMARY KEY (node, sender, request_id)
                 );
                 CREATE INDEX IF NOT EXISTS idx_rpc_replay_expiry
                     ON rpc_replay_v1(expires_at_ms);",
            )
            .map_err(|_| RpcError::Unavailable)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn reserve(
        &self,
        node: &str,
        sender: &str,
        request_id: &str,
        expires_at_ms: u64,
        now_ms: u64,
    ) -> Result<(), RpcError> {
        let expires_at_ms = i64::try_from(expires_at_ms).map_err(|_| RpcError::Unavailable)?;
        let now_ms = i64::try_from(now_ms).map_err(|_| RpcError::Unavailable)?;
        let mut connection = self.connection.lock().map_err(|_| RpcError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| RpcError::Unavailable)?;

        tx.execute(
            "DELETE FROM rpc_replay_v1 WHERE expires_at_ms < ?1",
            params![now_ms],
        )
        .map_err(|_| RpcError::Unavailable)?;

        // Check the exact composite key before global capacity. Duplicates
        // always return Replay, including when the ledger is otherwise full.
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM rpc_replay_v1
                  WHERE node = ?1 AND sender = ?2 AND request_id = ?3)",
                params![node, sender, request_id],
                |row| row.get(0),
            )
            .map_err(|_| RpcError::Unavailable)?;
        if exists {
            return Err(RpcError::Replay);
        }

        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM rpc_replay_v1", [], |row| row.get(0))
            .map_err(|_| RpcError::Unavailable)?;
        if count >= MAX_REPLAY_ENTRIES {
            return Err(RpcError::Busy);
        }

        tx.execute(
            "INSERT INTO rpc_replay_v1
             (node, sender, request_id, expires_at_ms)
             VALUES (?1, ?2, ?3, ?4)",
            params![node, sender, request_id, expires_at_ms],
        )
        .map_err(|_| RpcError::Unavailable)?;
        tx.commit().map_err(|_| RpcError::Unavailable)?;
        Ok(())
    }
}
