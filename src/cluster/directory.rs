//! Shared transactional agent membership and round-robin capability directory.
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_postgres::Client;

use super::leadership::open_database;
use crate::coordinator::{
    validate_capabilities, validate_tool_name, CoordinatorError, NodeRoute, NODE_LEASE_MS,
};
use crate::rpc::validate_node_id;

struct Connection {
    client: Mutex<Client>,
    task: JoinHandle<()>,
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone)]
pub struct PgDirectory {
    state: Arc<Connection>,
}

impl PgDirectory {
    pub async fn connect(dsn: &str, allow_loopback: bool) -> Result<Self, CoordinatorError> {
        let (client, task) = open_database(dsn, allow_loopback)
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        client
            .batch_execute(
                "BEGIN;
            SELECT pg_advisory_xact_lock(574123456);
            CREATE TABLE IF NOT EXISTS ocaf_pg_nodes_v1(
                node_id TEXT PRIMARY KEY NOT NULL,
                expires_at_ms BIGINT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ocaf_pg_tools_v1(
                node_id TEXT NOT NULL REFERENCES ocaf_pg_nodes_v1(node_id) ON DELETE CASCADE,
                tool TEXT NOT NULL,
                PRIMARY KEY(node_id,tool)
            );
            CREATE INDEX IF NOT EXISTS ocaf_pg_tools_lookup ON ocaf_pg_tools_v1(tool,node_id);
            CREATE TABLE IF NOT EXISTS ocaf_pg_cursor_v1(
                singleton INTEGER PRIMARY KEY CHECK(singleton=1),
                last_node TEXT NOT NULL
            );
            INSERT INTO ocaf_pg_cursor_v1(singleton,last_node)
                VALUES (1,'') ON CONFLICT(singleton) DO NOTHING;
            COMMIT;",
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        Ok(Self {
            state: Arc::new(Connection {
                client: Mutex::new(client),
                task,
            }),
        })
    }

    pub async fn heartbeat(
        &self,
        node_id: &str,
        tools: &[String],
        now_ms: u64,
    ) -> Result<NodeRoute, CoordinatorError> {
        validate_node_id(node_id).map_err(|_| CoordinatorError::Invalid)?;
        validate_capabilities(tools)?;
        let expiry = now_ms
            .checked_add(NODE_LEASE_MS)
            .ok_or(CoordinatorError::Invalid)?;
        let expires = i64::try_from(expiry).map_err(|_| CoordinatorError::Invalid)?;
        let mut client = self.state.client.lock().await;
        let tx = client
            .transaction()
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        // Two coordinators receiving heartbeats for the same peer must
        // serialize the complete capability replacement.
        tx.execute(
            "INSERT INTO ocaf_pg_nodes_v1(node_id,expires_at_ms)
            VALUES($1,$2) ON CONFLICT(node_id) DO UPDATE SET expires_at_ms=EXCLUDED.expires_at_ms",
            &[&node_id, &expires],
        )
        .await
        .map_err(|_| CoordinatorError::Storage)?;
        tx.execute("DELETE FROM ocaf_pg_tools_v1 WHERE node_id=$1", &[&node_id])
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        for tool in tools {
            tx.execute(
                "INSERT INTO ocaf_pg_tools_v1(node_id,tool) VALUES($1,$2)",
                &[&node_id, tool],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        }
        tx.commit().await.map_err(|_| CoordinatorError::Storage)?;
        Ok(NodeRoute {
            node_id: node_id.into(),
            expires_at_ms: expiry,
        })
    }

    pub async fn resolve(
        &self,
        tool: &str,
        now_ms: u64,
    ) -> Result<Option<NodeRoute>, CoordinatorError> {
        if !validate_tool_name(tool) {
            return Err(CoordinatorError::Invalid);
        }
        let now = i64::try_from(now_ms).map_err(|_| CoordinatorError::Invalid)?;
        let mut client = self.state.client.lock().await;
        let tx = client
            .transaction()
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        let current = tx
            .query_one(
                "SELECT last_node FROM ocaf_pg_cursor_v1 WHERE singleton=1 FOR UPDATE",
                &[],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        let last: String = current.get(0);
        let candidate = tx
            .query_opt(
                "SELECT n.node_id,n.expires_at_ms
             FROM ocaf_pg_nodes_v1 n JOIN ocaf_pg_tools_v1 t ON n.node_id=t.node_id
             WHERE t.tool=$1 AND n.expires_at_ms>$2
             ORDER BY CASE WHEN n.node_id>$3 THEN 0 ELSE 1 END,n.node_id
             LIMIT 1",
                &[&tool, &now, &last],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        let output = if let Some(row) = candidate {
            let name: String = row.get(0);
            let expires: i64 = row.get(1);
            tx.execute(
                "UPDATE ocaf_pg_cursor_v1 SET last_node=$1 WHERE singleton=1",
                &[&name],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
            Some(NodeRoute {
                node_id: name,
                expires_at_ms: u64::try_from(expires).map_err(|_| CoordinatorError::Storage)?,
            })
        } else {
            None
        };
        tx.commit().await.map_err(|_| CoordinatorError::Storage)?;
        Ok(output)
    }
}
