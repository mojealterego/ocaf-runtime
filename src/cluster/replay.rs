//! Shared PostgreSQL replay ledger. Each authenticated request is admitted
//! at most once across all coordinator instances using the same database.
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_postgres::Client;

use super::leadership::{open_database, PgLeader};
use crate::rpc::RpcError;

const MAX_REPLAY_ENTRIES: i64 = 4096;
const ADMISSION_LOCK: i64 = 0x4F43414652504C59;

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
pub struct PgReplay {
    state: Arc<Connection>,
    cluster_id: String,
    leader: Option<Arc<PgLeader>>,
}

impl PgReplay {
    pub async fn connect(dsn: &str, allow_loopback: bool) -> Result<Self, RpcError> {
        let (client, task) = open_database(dsn, allow_loopback)
            .await
            .map_err(|_| RpcError::Unavailable)?;
        client
            .batch_execute(
                "BEGIN;
            SELECT pg_advisory_xact_lock(574123456);
            CREATE TABLE IF NOT EXISTS ocaf_pg_replay_v2 (
                cluster_id TEXT NOT NULL,
                node TEXT NOT NULL,
                sender TEXT NOT NULL,
                request_id TEXT NOT NULL,
                expires_at_ms BIGINT NOT NULL,
                PRIMARY KEY(cluster_id, node, sender, request_id)
            );
            CREATE INDEX IF NOT EXISTS ocaf_pg_replay_expiry_v2
                ON ocaf_pg_replay_v2(cluster_id,expires_at_ms);
            COMMIT;",
            )
            .await
            .map_err(|_| RpcError::Unavailable)?;
        Ok(Self {
            state: Arc::new(Connection {
                client: Mutex::new(client),
                task,
            }),
            cluster_id: String::new(),
            leader: None,
        })
    }

    /// Attach the current leader trust domain. No cross-cluster replay
    /// nonce or capacity is shared in protocol v2.
    pub fn with_leader(mut self, leader: Arc<PgLeader>) -> Self {
        self.cluster_id = leader.cluster_id().to_owned();
        self.leader = Some(leader);
        self
    }

    pub async fn reserve(
        &self,
        node: &str,
        sender: &str,
        request_id: &str,
        expires_at_ms: u64,
        now_ms: u64,
    ) -> Result<(), RpcError> {
        let expiry = i64::try_from(expires_at_ms).map_err(|_| RpcError::Unavailable)?;
        let now = i64::try_from(now_ms).map_err(|_| RpcError::Unavailable)?;
        let guard = match &self.leader {
            Some(leader) => Some(
                leader
                    .fence_transaction()
                    .await
                    .map_err(|_| RpcError::Unavailable)?,
            ),
            None => None,
        };
        let epoch = guard.as_ref().map_or(0, |g| g.epoch());
        let mut client = self.state.client.lock().await;
        let tx = client
            .transaction()
            .await
            .map_err(|_| RpcError::Unavailable)?;
        if self.leader.is_some() {
            if epoch <= 0 {
                return Err(RpcError::Unavailable);
            }
            let current = tx
                .query_opt(
                    "SELECT epoch FROM ocaf_pg_leader_epochs_v1
                     WHERE cluster_id=$1 FOR SHARE",
                    &[&self.cluster_id],
                )
                .await
                .map_err(|_| RpcError::Unavailable)?;
            if current.is_none_or(|row| row.get::<_, i64>(0) != epoch) {
                return Err(RpcError::Unavailable);
            }
        }
        // Serialize the purge, duplicate admission and capacity bound
        // across concurrent connections. This is transaction-scoped.
        tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&ADMISSION_LOCK])
            .await
            .map_err(|_| RpcError::Unavailable)?;
        tx.execute(
            "DELETE FROM ocaf_pg_replay_v2
             WHERE cluster_id=$1 AND expires_at_ms < $2",
            &[&self.cluster_id, &now],
        )
        .await
        .map_err(|_| RpcError::Unavailable)?;
        let duplicate = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM ocaf_pg_replay_v2
             WHERE cluster_id=$1 AND node=$2 AND sender=$3 AND request_id=$4)",
                &[&self.cluster_id, &node, &sender, &request_id],
            )
            .await
            .map_err(|_| RpcError::Unavailable)?;
        if duplicate.get::<_, bool>(0) {
            return Err(RpcError::Replay);
        }
        let count = tx
            .query_one(
                "SELECT count(*) FROM ocaf_pg_replay_v2 WHERE cluster_id=$1",
                &[&self.cluster_id],
            )
            .await
            .map_err(|_| RpcError::Unavailable)?;
        if count.get::<_, i64>(0) >= MAX_REPLAY_ENTRIES {
            return Err(RpcError::Busy);
        }
        tx.execute(
            "INSERT INTO ocaf_pg_replay_v2(cluster_id,node,sender,request_id,expires_at_ms)
             VALUES ($1,$2,$3,$4,$5)",
            &[&self.cluster_id, &node, &sender, &request_id, &expiry],
        )
        .await
        .map_err(|_| RpcError::Unavailable)?;
        tx.commit().await.map_err(|_| RpcError::Unavailable)?;
        drop(guard);
        Ok(())
    }
}
