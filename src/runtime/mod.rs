use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use thiserror::Error;

use crate::auth::{AuthContext, AuthPolicy};
use crate::executor::{ExecutionContext, ToolError, ToolExecutor};
use crate::metrics::MetricsStore;
use crate::registry::{ToolManifest, ToolRegistry};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallRequest {
    pub call_id: String,
    pub tool: String,
    pub args: Value,
    pub auth: Option<AuthContext>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallResult {
    pub call_id: String,
    pub status: String,
    pub output: Value,
    pub execution_time_us: u64,
    pub cost_sats: u64,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("budget exceeded")]
    BudgetExceeded,
    #[error("required authentication: {0}")]
    AuthRequired(String),
    #[error("tool execution failed: {0}")]
    ToolExecution(String),
    #[error("timeout while executing tool")]
    Timeout,
}

impl From<ToolError> for RuntimeError {
    fn from(value: ToolError) -> Self {
        match value {
            ToolError::Timeout => RuntimeError::Timeout,
            _ => RuntimeError::ToolExecution(value.to_string()),
        }
    }
}

#[derive(Clone)]
pub struct OcafRuntime {
    pub registry: ToolRegistry,
    pub budget_sats: Arc<tokio::sync::RwLock<u64>>,
    pub metrics: MetricsStore,
}

impl OcafRuntime {
    pub fn new(initial_budget_sats: u64) -> Self {
        Self {
            registry: ToolRegistry::new(),
            budget_sats: Arc::new(tokio::sync::RwLock::new(initial_budget_sats)),
            metrics: MetricsStore::new(),
        }
    }

    pub async fn register<T>(&self, manifest: ToolManifest, executor: T)
    where
        T: ToolExecutor + 'static,
    {
        self.registry.register(manifest, executor).await;
    }

    pub async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult, RuntimeError> {
        let start = std::time::Instant::now();

        let tool = self
            .registry
            .resolve(&request.tool)
            .await
            .ok_or_else(|| RuntimeError::ToolNotFound(request.tool.clone()))?;

        let manifest = tool.manifest.clone();

        if let Err(_) = AuthPolicy::validate(&request.tool, request.auth.as_ref(), manifest.cost_sats) {
            let challenge = AuthPolicy::challenge_for(&request.tool, manifest.cost_sats);
            return Err(RuntimeError::AuthRequired(serde_json::to_string(&challenge).unwrap_or_default()));
        }

        if manifest.cost_sats > 0 {
            let mut budget = self.budget_sats.write().await;
            if *budget < manifest.cost_sats {
                return Err(RuntimeError::BudgetExceeded);
            }
            *budget -= manifest.cost_sats;
        }

        let ctx = ExecutionContext {
            agent_id: "agent-01".to_string(),
            budget_sats: self.budget_sats.clone(),
            deadline_ms: manifest.timeout_ms,
            correlation_id: request.call_id.clone(),
        };

        let output = match tokio::time::timeout(
            Duration::from_millis(manifest.timeout_ms),
            tool.executor.execute(request.args.clone(), &ctx),
        )
        .await
        {
            Ok(result) => result.map_err(RuntimeError::from)?,
            Err(_) => {
                self.metrics.record_failure(start.elapsed().as_micros() as u64).await;
                return Err(RuntimeError::Timeout);
            }
        };

        let elapsed_us = start.elapsed().as_micros() as u64;
        let result = ToolCallResult {
            call_id: request.call_id,
            status: "SUCCESS".to_string(),
            output,
            execution_time_us: elapsed_us,
            cost_sats: manifest.cost_sats,
        };

        self.metrics.record_success(elapsed_us).await;
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct ZenohTransport {
    session: Arc<zenoh::Session>,
}

impl ZenohTransport {
    pub async fn connect() -> anyhow::Result<Self> {
        let session = Arc::new(zenoh::open(zenoh::Config::default()).res().await?);
        Ok(Self { session })
    }

    pub fn session(&self) -> Arc<zenoh::Session> {
        self.session.clone()
    }
}

pub struct EchoTool;

#[async_trait::async_trait]
impl ToolExecutor for EchoTool {
    async fn execute(
        &self,
        args: Value,
        _ctx: &ExecutionContext,
    ) -> Result<Value, ToolError> {
        let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("hello");
        Ok(serde_json::json!({
            "status": "ok",
            "message": format!("processed: {}", message),
            "received": args,
        }))
    }
}

pub fn pricing_tool_manifest() -> ToolManifest {
    ToolManifest {
        name: "pricing_tool".to_string(),
        description: "Returns a deterministic echo payload for the runtime demo".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "status": { "type": "string" },
                "message": { "type": "string" },
                "received": { "type": "object" }
            }
        }),
        cost_sats: 25,
        permissions: vec!["tool:echo".to_string()],
        timeout_ms: 250,
        enabled: true,
    }
}

pub fn sample_auth() -> AuthContext {
    AuthContext {
        macaroon: "demo-macaroon".to_string(),
        preimage: "demo-preimage".to_string(),
        invoice: "lnbc_demo_invoice".to_string(),
        nonce: "demo-nonce".to_string(),
    }
}
