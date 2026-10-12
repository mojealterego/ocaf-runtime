//! Cluster-scoped PostgreSQL capability directory. Legacy v1 tables remain
//! untouched: their original namespace cannot be inferred safely.
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_postgres::{Client, Transaction};

use super::leadership::{open_database, PgLeader};
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
    cluster_id: String,
    leader: Option<Arc<PgLeader>>,
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
             CREATE TABLE IF NOT EXISTS ocaf_pg_nodes_v2(
                 cluster_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 expires_at_ms BIGINT NOT NULL,
                 PRIMARY KEY(cluster_id,node_id)
             );
             CREATE TABLE IF NOT EXISTS ocaf_pg_tools_v2(
                 cluster_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 tool TEXT NOT NULL,
                 PRIMARY KEY(cluster_id,node_id,tool),
                 FOREIGN KEY(cluster_id,node_id)
                   REFERENCES ocaf_pg_nodes_v2(cluster_id,node_id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS ocaf_pg_tools_lookup_v2
               ON ocaf_pg_tools_v2(cluster_id,tool,node_id);
             CREATE TABLE IF NOT EXISTS ocaf_pg_cursor_v2(
                 cluster_id TEXT PRIMARY KEY NOT NULL,
                 last_node TEXT NOT NULL
             );
             COMMIT;",
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        Ok(Self {
            state: Arc::new(Connection {
                client: Mutex::new(client),
                task,
            }),
            cluster_id: String::new(),
            leader: None,
        })
    }

    /// Assign this privileged directory to a verified leadership namespace.
    /// All writes and cursor changes are fenced by that leadership epoch.
    pub fn with_leader(mut self, leader: Arc<PgLeader>) -> Self {
        self.cluster_id = leader.cluster_id().to_owned();
        self.leader = Some(leader);
        self
    }

    async fn check_epoch_in_transaction(
        &self,
        tx: &Transaction<'_>,
        epoch: i64,
    ) -> Result<(), CoordinatorError> {
        if self.leader.is_none() {
            return if epoch == 0 {
                Ok(())
            } else {
                Err(CoordinatorError::Storage)
            };
        }
        if epoch <= 0 {
            return Err(CoordinatorError::Storage);
        }
        let row = tx
            .query_opt(
                "SELECT epoch FROM ocaf_pg_leader_epochs_v1
                 WHERE cluster_id=$1 FOR SHARE",
                &[&self.cluster_id],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        if row.is_none_or(|value| value.get::<_, i64>(0) != epoch) {
            return Err(CoordinatorError::Storage);
        }
        Ok(())
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
        let guard = match &self.leader {
            Some(leader) => Some(
                leader
                    .fence_transaction()
                    .await
                    .map_err(|_| CoordinatorError::Storage)?,
            ),
            None => None,
        };
        let generation = guard.as_ref().map_or(0, |g| g.epoch());
        let mut client = self.state.client.lock().await;
        let tx = client
            .transaction()
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        self.check_epoch_in_transaction(&tx, generation).await?;
        tx.execute(
            "INSERT INTO ocaf_pg_nodes_v2(cluster_id,node_id,expires_at_ms)
             VALUES($1,$2,$3)
             ON CONFLICT(cluster_id,node_id) DO UPDATE
             SET expires_at_ms=EXCLUDED.expires_at_ms",
            &[&self.cluster_id, &node_id, &expires],
        )
        .await
        .map_err(|_| CoordinatorError::Storage)?;
        tx.execute(
            "DELETE FROM ocaf_pg_tools_v2 WHERE cluster_id=$1 AND node_id=$2",
            &[&self.cluster_id, &node_id],
        )
        .await
        .map_err(|_| CoordinatorError::Storage)?;
        for tool in tools {
            tx.execute(
                "INSERT INTO ocaf_pg_tools_v2(cluster_id,node_id,tool)
                 VALUES($1,$2,$3)",
                &[&self.cluster_id, &node_id, tool],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        }
        tx.commit().await.map_err(|_| CoordinatorError::Storage)?;
        drop(guard);
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
        let guard = match &self.leader {
            Some(leader) => Some(
                leader
                    .fence_transaction()
                    .await
                    .map_err(|_| CoordinatorError::Storage)?,
            ),
            None => None,
        };
        let generation = guard.as_ref().map_or(0, |g| g.epoch());
        let mut client = self.state.client.lock().await;
        let tx = client
            .transaction()
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        self.check_epoch_in_transaction(&tx, generation).await?;
        tx.execute(
            "INSERT INTO ocaf_pg_cursor_v2(cluster_id,last_node)
             VALUES($1,'') ON CONFLICT(cluster_id) DO NOTHING",
            &[&self.cluster_id],
        )
        .await
        .map_err(|_| CoordinatorError::Storage)?;
        let current = tx
            .query_one(
                "SELECT last_node FROM ocaf_pg_cursor_v2
                 WHERE cluster_id=$1 FOR UPDATE",
                &[&self.cluster_id],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        let last: String = current.get(0);
        let candidate = tx
            .query_opt(
                "SELECT n.node_id,n.expires_at_ms
                 FROM ocaf_pg_nodes_v2 n
                 JOIN ocaf_pg_tools_v2 t
                   ON n.cluster_id=t.cluster_id AND n.node_id=t.node_id
                 WHERE n.cluster_id=$1 AND t.tool=$2
                   AND n.expires_at_ms>$3
                 ORDER BY CASE WHEN n.node_id>$4 THEN 0 ELSE 1 END,n.node_id
                 LIMIT 1",
                &[&self.cluster_id, &tool, &now, &last],
            )
            .await
            .map_err(|_| CoordinatorError::Storage)?;
        let output = if let Some(row) = candidate {
            let name: String = row.get(0);
            let expires: i64 = row.get(1);
            tx.execute(
                "UPDATE ocaf_pg_cursor_v2 SET last_node=$2 WHERE cluster_id=$1",
                &[&self.cluster_id, &name],
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
        drop(guard);
        Ok(output)
    }
}
