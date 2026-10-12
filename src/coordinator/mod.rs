//! Persistent single-host coordinator directory.
//!
//! Agents periodically advertise capabilities through the signed RPC
//! boundary. The directory is only an *availability hint*: it does not
//! attest that remote executors are running, and routing is not a
//! distributed job lock, consensus election or side-effect guarantee.
pub mod leadership;

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rpc::validate_node_id;

pub const NODE_LEASE_MS: u64 = 15_000;
const MAX_TOOLS: usize = 16;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoordinatorError {
    #[error("invalid node identity or advertised capabilities")]
    Invalid,
    #[error("persistent coordinator database is unavailable")]
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRoute {
    pub node_id: String,
    pub expires_at_ms: u64,
}

#[derive(Clone)]
pub struct CoordinatorStore {
    connection: Arc<Mutex<Connection>>,
}

fn sql_time(value: u64) -> Result<i64, CoordinatorError> {
    i64::try_from(value).map_err(|_| CoordinatorError::Invalid)
}

pub fn validate_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

pub fn validate_capabilities(tools: &[String]) -> Result<(), CoordinatorError> {
    if tools.is_empty()
        || tools.len() > MAX_TOOLS
        || tools.iter().any(|name| !validate_tool_name(name))
        || tools.iter().collect::<HashSet<_>>().len() != tools.len()
    {
        return Err(CoordinatorError::Invalid);
    }
    Ok(())
}

impl CoordinatorStore {
    pub fn open(path: &Path) -> Result<Self, CoordinatorError> {
        if path.as_os_str().is_empty() {
            return Err(CoordinatorError::Invalid);
        }
        #[cfg(unix)]
        {
            use std::fs::{OpenOptions, Permissions};
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() {
                    return Err(CoordinatorError::Storage);
                }
            }
            let _file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(path)
                .map_err(|_| CoordinatorError::Storage)?;
            std::fs::set_permissions(path, Permissions::from_mode(0o600))
                .map_err(|_| CoordinatorError::Storage)?;
        }
        let conn = Connection::open(path).map_err(|_| CoordinatorError::Storage)?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|_| CoordinatorError::Storage)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS coordinator_nodes_v1 (
                 node_id TEXT PRIMARY KEY NOT NULL,
                 expires_at_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS coordinator_tools_v1 (
                 node_id TEXT NOT NULL,
                 tool TEXT NOT NULL,
                 PRIMARY KEY (node_id, tool),
                 FOREIGN KEY(node_id) REFERENCES coordinator_nodes_v1(node_id)
                     ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_coordinator_tool
                 ON coordinator_tools_v1(tool, node_id);
             CREATE TABLE IF NOT EXISTS coordinator_cursor_v1 (
                 singleton INTEGER PRIMARY KEY CHECK (singleton=1),
                 last_node TEXT NOT NULL
             );
             INSERT OR IGNORE INTO coordinator_cursor_v1(singleton, last_node)
                 VALUES (1, '');",
        )
        .map_err(|_| CoordinatorError::Storage)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(conn)),
        })
    }

    async fn run<T, F>(&self, op: F) -> Result<T, CoordinatorError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, CoordinatorError> + Send + 'static,
    {
        let conn = Arc::clone(&self.connection);
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().map_err(|_| CoordinatorError::Storage)?;
            op(&mut conn)
        })
        .await
        .map_err(|_| CoordinatorError::Storage)?
    }

    /// Upsert a heartbeat and *replace* the previous capability set in one
    /// transaction. Peer identity and capability ACL must be verified by the
    /// RPC gateway; callers using this direct API must be trusted.
    pub async fn heartbeat(
        &self,
        node_id: &str,
        tools: &[String],
        now_ms: u64,
    ) -> Result<NodeRoute, CoordinatorError> {
        if validate_node_id(node_id).is_err() {
            return Err(CoordinatorError::Invalid);
        }
        validate_capabilities(tools)?;
        let expires_at_ms = now_ms
            .checked_add(NODE_LEASE_MS)
            .ok_or(CoordinatorError::Invalid)?;
        let expires = sql_time(expires_at_ms)?;
        let node_id = node_id.to_owned();
        let tools = tools.to_vec();
        self.run(move |conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| CoordinatorError::Storage)?;
            tx.execute(
                "INSERT INTO coordinator_nodes_v1(node_id, expires_at_ms)
                 VALUES (?1, ?2)
                 ON CONFLICT(node_id) DO UPDATE SET expires_at_ms=excluded.expires_at_ms",
                params![node_id, expires],
            )
            .map_err(|_| CoordinatorError::Storage)?;
            tx.execute(
                "DELETE FROM coordinator_tools_v1 WHERE node_id=?1",
                params![node_id],
            )
            .map_err(|_| CoordinatorError::Storage)?;
            for tool in tools {
                tx.execute(
                    "INSERT INTO coordinator_tools_v1(node_id, tool) VALUES (?1, ?2)",
                    params![node_id, tool],
                )
                .map_err(|_| CoordinatorError::Storage)?;
            }
            tx.commit().map_err(|_| CoordinatorError::Storage)?;
            Ok(NodeRoute {
                node_id,
                expires_at_ms,
            })
        })
        .await
    }

    /// Atomically rotate through live nodes advertising the exact tool.
    /// A returned node is a routing *hint*, not a reserved execution slot.
    pub async fn resolve(
        &self,
        tool: &str,
        now_ms: u64,
    ) -> Result<Option<NodeRoute>, CoordinatorError> {
        if !validate_tool_name(tool) {
            return Err(CoordinatorError::Invalid);
        }
        let now = sql_time(now_ms)?;
        let tool = tool.to_owned();
        self.run(move |conn| {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| CoordinatorError::Storage)?;
            let last: String = tx
                .query_row(
                    "SELECT last_node FROM coordinator_cursor_v1 WHERE singleton=1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| CoordinatorError::Storage)?
                .ok_or(CoordinatorError::Storage)?;
            let routes: Vec<NodeRoute> = {
                let mut stmt = tx
                    .prepare(
                        "SELECT n.node_id, n.expires_at_ms
                         FROM coordinator_nodes_v1 n
                         JOIN coordinator_tools_v1 t ON n.node_id=t.node_id
                         WHERE t.tool=?1 AND n.expires_at_ms>?2
                         ORDER BY n.node_id",
                    )
                    .map_err(|_| CoordinatorError::Storage)?;
                let rows = stmt
                    .query_map(params![tool, now], |row| {
                        Ok(NodeRoute {
                            node_id: row.get(0)?,
                            expires_at_ms: row.get::<_, i64>(1)? as u64,
                        })
                    })
                    .map_err(|_| CoordinatorError::Storage)?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|_| CoordinatorError::Storage)?
            };
            let selected = routes
                .iter()
                .find(|route| route.node_id > last)
                .or_else(|| routes.first())
                .cloned();
            if let Some(route) = &selected {
                tx.execute(
                    "UPDATE coordinator_cursor_v1 SET last_node=?1 WHERE singleton=1",
                    params![route.node_id],
                )
                .map_err(|_| CoordinatorError::Storage)?;
            }
            tx.commit().map_err(|_| CoordinatorError::Storage)?;
            Ok(selected)
        })
        .await
    }
}
