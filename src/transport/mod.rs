pub mod rpc;

use serde_json::Value;
use std::sync::Arc;

use crate::executor::{ExecutionContext, ToolError, ToolExecutor};
use crate::registry::ToolManifest;

/// Only opens a Zenoh session. Tool discovery, pub/sub and remote execution
/// still require explicit protocol integration.
#[derive(Debug, Clone)]
pub struct ZenohTransport {
    session: Arc<zenoh::Session>,
}

impl ZenohTransport {
    pub async fn connect() -> anyhow::Result<Self> {
        Self::connect_with_config(zenoh::Config::default()).await
    }

    pub async fn connect_with_config(config: zenoh::Config) -> anyhow::Result<Self> {
        let session = Arc::new(
            zenoh::open(config)
                .await
                .map_err(|error| anyhow::anyhow!("failed to open Zenoh session: {error}"))?,
        );
        Ok(Self { session })
    }

    pub fn session(&self) -> Arc<zenoh::Session> {
        Arc::clone(&self.session)
    }
}

pub struct EchoTool;

#[async_trait::async_trait]
impl ToolExecutor for EchoTool {
    async fn execute(&self, args: Value, _ctx: &ExecutionContext) -> Result<Value, ToolError> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("message must be a string".into()))?;
        Ok(serde_json::json!({
            "status": "ok",
            "message": format!("processed: {}", message),
            "received": args,
        }))
    }
}

/// This is a FREE local demonstration. It does not request, issue or validate
/// a Lightning invoice or an L402 credential.
pub fn pricing_tool_manifest() -> ToolManifest {
    ToolManifest {
        name: "demo_echo".to_string(),
        description: "Returns a deterministic echo payload in the local runtime".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"message": {"type": "string"}},
            "required": ["message"],
            "additionalProperties": false
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "status": {"type": "string"},
                "message": {"type": "string"},
                "received": {"type": "object"}
            },
            "required": ["status", "message", "received"]
        }),
        cost_sats: 0,
        permissions: vec![],
        timeout_ms: 250,
        enabled: true,
    }
}
