//! Shared PostgreSQL replay ledger. Each authenticated request is admitted
//! at most once across all coordinator instances using the same database.
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_postgres::Client;

use super::leadership::open_database;
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
}

impl PgReplay {
    pub async fn connect(dsn: &str, allow_loopback: bool) -> Result<Self, RpcError> {
        let (client, task) = open_database(dsn, allow_loopback)
            .await
            .map_err(|_| RpcError::Unavailable)?;
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS ocaf_pg_replay_v1 (
                node TEXT NOT NULL,
                sender TEXT NOT NULL,
                request_id TEXT NOT NULL,
                expires_at_ms BIGINT NOT NULL,
                PRIMARY KEY(node, sender, request_id)
            );
            CREATE INDEX IF NOT EXISTS ocaf_pg_replay_expiry
                ON ocaf_pg_replay_v1(expires_at_ms);",
            )
            .await
            .map_err(|_| RpcError::Unavailable)?;
        Ok(Self {
            state: Arc::new(Connection {
                client: Mutex::new(client),
                task,
            }),
        })
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
        let mut client = self.state.client.lock().await;
        let tx = client
            .transaction()
            .await
            .map_err(|_| RpcError::Unavailable)?;
        // Serialize the purge, duplicate admission and capacity bound
        // across concurrent connections. This is transaction-scoped.
        tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&ADMISSION_LOCK])
            .await
            .map_err(|_| RpcError::Unavailable)?;
        tx.execute(
            "DELETE FROM ocaf_pg_replay_v1 WHERE expires_at_ms < $1",
            &[&now],
        )
        .await
        .map_err(|_| RpcError::Unavailable)?;
        let duplicate = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM ocaf_pg_replay_v1
             WHERE node=$1 AND sender=$2 AND request_id=$3)",
                &[&node, &sender, &request_id],
            )
            .await
            .map_err(|_| RpcError::Unavailable)?;
        if duplicate.get::<_, bool>(0) {
            return Err(RpcError::Replay);
        }
        let count = tx
            .query_one("SELECT count(*) FROM ocaf_pg_replay_v1", &[])
            .await
            .map_err(|_| RpcError::Unavailable)?;
        if count.get::<_, i64>(0) >= MAX_REPLAY_ENTRIES {
            return Err(RpcError::Busy);
        }
        tx.execute(
            "INSERT INTO ocaf_pg_replay_v1(node,sender,request_id,expires_at_ms)
             VALUES ($1,$2,$3,$4)",
            &[&node, &sender, &request_id, &expiry],
        )
        .await
        .map_err(|_| RpcError::Unavailable)?;
        tx.commit().await.map_err(|_| RpcError::Unavailable)?;
        Ok(())
    }
}
