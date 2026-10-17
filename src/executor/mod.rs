use crate::registry::ToolManifest;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Identity is not authenticated until a production auth provider is added.
    pub agent_id: String,
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
    async fn execute(&self, args: Value, ctx: &ExecutionContext) -> Result<Value, ToolError>;

    /// Signed OMAS dispatch must explicitly enforce the authenticated fuel cap.
    /// Generic executors are denied by default rather than ignoring the cap.
    async fn execute_signed_fuel(&self, _args: Value, _ctx: &ExecutionContext, _fuel: u64) -> Result<Value, ToolError> {
        Err(ToolError::Execution("signed fuel is not supported by this executor".into()))
    }
}

#[derive(Clone)]
pub struct ToolHandle {
    pub manifest: ToolManifest,
    pub executor: Arc<dyn ToolExecutor>,
    pub input_validator: Arc<jsonschema::Validator>,
    pub output_validator: Arc<jsonschema::Validator>,
}

impl ToolHandle {
    pub fn new<T>(
        manifest: ToolManifest,
        executor: T,
        input_validator: Arc<jsonschema::Validator>,
        output_validator: Arc<jsonschema::Validator>,
    ) -> Self
    where
        T: ToolExecutor + 'static,
    {
        Self {
            manifest,
            executor: Arc::new(executor),
            input_validator,
            output_validator,
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

// Stage 0.38: Linux-only, operator-pinned, deny-by-default WASI adapter.
#[cfg(target_os = "linux")]
pub mod wasi;
