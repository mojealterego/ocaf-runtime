//! Uniform RPC capability directory for local or shared backing stores.
use crate::cluster::directory::PgDirectory;
use crate::coordinator::{CoordinatorError, CoordinatorStore, NodeRoute};

pub(super) enum DirectoryBackend {
    Sqlite(CoordinatorStore),
    Postgres(PgDirectory),
}

impl DirectoryBackend {
    pub(super) async fn heartbeat(
        &self,
        node: &str,
        tools: &[String],
        now: u64,
    ) -> Result<NodeRoute, CoordinatorError> {
        match self {
            Self::Sqlite(db) => db.heartbeat(node, tools, now).await,
            Self::Postgres(db) => db.heartbeat(node, tools, now).await,
        }
    }

    pub(super) async fn resolve(
        &self,
        tool: &str,
        now: u64,
    ) -> Result<Option<NodeRoute>, CoordinatorError> {
        match self {
            Self::Sqlite(db) => db.resolve(tool, now).await,
            Self::Postgres(db) => db.resolve(tool, now).await,
        }
    }
}
