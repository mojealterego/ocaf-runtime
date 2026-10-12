use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::executor::{ToolExecutor, ToolHandle};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub cost_sats: u64,
    pub permissions: Vec<String>,
    pub timeout_ms: u64,
    pub enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("invalid tool manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid or unsafe JSON schema: {0}")]
    InvalidSchema(String),
    #[error("tool already registered: {0}")]
    DuplicateTool(String),
}

/// Block remote references in untrusted schemas. Validator compilation happens
/// inside an async runtime, where network reference resolution may panic or
/// trigger unintended outbound requests. Local fragment refs are supported.
fn schema_uses_external_refs(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => map.iter().any(|(key, value)| {
            if key == "$ref" || key == "$dynamicRef" || key == "$recursiveRef" {
                value
                    .as_str()
                    .is_none_or(|reference| !reference.starts_with('#'))
            } else {
                schema_uses_external_refs(value)
            }
        }),
        Value::Array(items) => items.iter().any(schema_uses_external_refs),
        _ => false,
    }
}

pub fn compile_schema(schema: &Value) -> Result<jsonschema::Validator, RegistryError> {
    if schema_uses_external_refs(schema) {
        return Err(RegistryError::InvalidSchema(
            "external schema references are not permitted".into(),
        ));
    }

    jsonschema::validator_for(schema)
        .map_err(|error| RegistryError::InvalidSchema(error.to_string()))
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolHandle>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register<T>(
        &self,
        manifest: ToolManifest,
        executor: T,
    ) -> Result<(), RegistryError>
    where
        T: ToolExecutor + 'static,
    {
        if manifest.name.trim().is_empty() {
            return Err(RegistryError::InvalidManifest(
                "name must not be empty".into(),
            ));
        }
        if manifest.timeout_ms == 0 {
            return Err(RegistryError::InvalidManifest(
                "timeout_ms must be greater than zero".into(),
            ));
        }
        if manifest
            .permissions
            .iter()
            .any(|permission| permission.trim().is_empty())
        {
            return Err(RegistryError::InvalidManifest(
                "permissions must not contain empty entries".into(),
            ));
        }

        let input_validator = Arc::new(compile_schema(&manifest.input_schema)?);
        let output_validator = Arc::new(compile_schema(&manifest.output_schema)?);
        let name = manifest.name.clone();
        let handle = ToolHandle::new(manifest, executor, input_validator, output_validator);
        let mut tools = self.tools.write().await;
        if tools.contains_key(&name) {
            return Err(RegistryError::DuplicateTool(name));
        }
        tools.insert(name, handle);
        Ok(())
    }

    pub async fn resolve(&self, name: &str) -> Option<ToolHandle> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<ToolManifest> {
        let mut tools: Vec<_> = self
            .tools
            .read()
            .await
            .values()
            .map(|entry| entry.manifest.clone())
            .collect();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }
}
