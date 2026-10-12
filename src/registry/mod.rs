use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::executor::{ToolError, ToolExecutor, ToolHandle};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub cost_sats: u64,
    pub permissions: Vec<String>,
    pub timeout_ms: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolHandle>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register<T>(&self, manifest: ToolManifest, executor: T)
    where
        T: ToolExecutor + 'static,
    {
        let tool = ToolHandle::new(manifest.clone(), executor);
        self.tools.write().await.insert(manifest.name.clone(), tool);
    }

    pub async fn resolve(&self, name: &str) -> Option<ToolHandle> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<ToolManifest> {
        self.tools
            .read()
            .await
            .values()
            .map(|entry| entry.manifest.clone())
            .collect()
    }
}
