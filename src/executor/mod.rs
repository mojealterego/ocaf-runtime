use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use thiserror::Error;

use crate::registry::ToolManifest;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub agent_id: String,
    pub budget_sats: Arc<tokio::sync::RwLock<u64>>,
    pub deadline_ms: u64,
    pub correlation_id: String,
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("tool timed out")]
    Timeout,
    #[error("input validation failed: {0}")]
    Validation(String),
}

#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(
        &self,
        args: Value,
        ctx: &ExecutionContext,
    ) -> Result<Value, ToolError>;
}

pub struct ToolHandle {
    pub manifest: ToolManifest,
    pub executor: Arc<dyn ToolExecutor>,
}

impl ToolHandle {
    pub fn new<T>(manifest: ToolManifest, executor: T) -> Self
    where
        T: ToolExecutor + 'static,
    {
        Self {
            manifest,
            executor: Arc::new(executor),
        }
    }
}

pub async fn with_timeout<F, T>(duration: Duration, f: F) -> Result<T, ToolError>
where
    F: std::future::Future<Output = Result<T, ToolError>>,
{
    tokio::time::timeout(duration, f)
        .await
        .map_err(|_| ToolError::Timeout)?
}
