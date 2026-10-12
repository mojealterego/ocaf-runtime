use std::sync::Arc;

use serde_json::Value;

use crate::auth::AuthContext;
use crate::executor::{ExecutionContext, ToolError, ToolExecutor};
use crate::registry::ToolManifest;

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
