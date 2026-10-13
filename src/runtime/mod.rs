use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use thiserror::Error;

use crate::auth::{AuthContext, AuthPolicy, AuthVerifier, DenyByDefaultVerifier};
use crate::executor::{ExecutionContext, ToolError, ToolExecutor};
use crate::metrics::MetricsStore;
use crate::registry::{RegistryError, ToolManifest, ToolRegistry};

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
    #[error("tool is disabled: {0}")]
    ToolDisabled(String),
    #[error("budget exceeded")]
    BudgetExceeded,
    #[error("authorization required or rejected: {0}")]
    AuthRequired(String),
    #[error("invalid tool input: {0}")]
    InvalidInput(String),
    #[error("invalid tool output: {0}")]
    InvalidOutput(String),
    #[error("tool execution failed: {0}")]
    ToolExecution(String),
    #[error("timeout while executing tool or authorizer")]
    Timeout,
}

impl From<ToolError> for RuntimeError {
    fn from(value: ToolError) -> Self {
        match value {
            ToolError::Timeout => Self::Timeout,
            _ => Self::ToolExecution(value.to_string()),
        }
    }
}

/// An atomic budget reservation that refunds on errors, task cancellation,
/// panics during unwinding, and invalid tool output.
struct BudgetReservation {
    budget: Arc<AtomicU64>,
    cost_sats: u64,
    committed: bool,
}

impl BudgetReservation {
    fn reserve(budget: Arc<AtomicU64>, cost_sats: u64) -> Result<Self, RuntimeError> {
        if cost_sats != 0 {
            budget
                .try_update(Ordering::AcqRel, Ordering::Acquire, |available| {
                    available.checked_sub(cost_sats)
                })
                .map_err(|_| RuntimeError::BudgetExceeded)?;
        }
        Ok(Self {
            budget,
            cost_sats,
            committed: false,
        })
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for BudgetReservation {
    fn drop(&mut self) {
        if !self.committed && self.cost_sats != 0 {
            self.budget.fetch_add(self.cost_sats, Ordering::AcqRel);
        }
    }
}

fn valid_principal_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-.".contains(&c))
}

#[derive(Clone)]
pub struct OcafRuntime {
    pub registry: ToolRegistry,
    budget_sats: Arc<AtomicU64>,
    pub metrics: MetricsStore,
    auth_verifier: Arc<dyn AuthVerifier>,
    tenant_namespace: Option<String>,
}

impl OcafRuntime {
    pub fn new(initial_budget_sats: u64) -> Self {
        Self {
            registry: ToolRegistry::new(),
            budget_sats: Arc::new(AtomicU64::new(initial_budget_sats)),
            metrics: MetricsStore::new(),
            auth_verifier: Arc::new(DenyByDefaultVerifier),
            tenant_namespace: None,
        }
    }

    /// Supply a production L402 and permission verifier. The default remains
    /// fail-closed for paid and restricted tools.
    pub fn with_auth_verifier<V: AuthVerifier + 'static>(mut self, verifier: V) -> Self {
        self.auth_verifier = Arc::new(verifier);
        self
    }

    pub fn remaining_budget_sats(&self) -> u64 {
        self.budget_sats.load(Ordering::Acquire)
    }

    pub async fn register<T>(
        &self,
        manifest: ToolManifest,
        executor: T,
    ) -> Result<(), RegistryError>
    where
        T: ToolExecutor + 'static,
    {
        self.registry.register(manifest, executor).await
    }

    /// Install the trusted deployment namespace (not a field from the
    /// inbound JSON request). Required for signed WASI tenant isolation.
    pub fn with_tenant_namespace(mut self, tenant: &str) -> Result<Self, RuntimeError> {
        if !valid_principal_part(tenant) {
            return Err(RuntimeError::AuthRequired(
                "invalid tenant namespace".into(),
            ));
        }
        self.tenant_namespace = Some(tenant.to_owned());
        Ok(self)
    }

    /// The authenticated sender must be supplied ONLY by a trusted RPC
    /// handler after HMAC, certificate PoP, TLS binding and replay gates.
    /// This method does not authenticate a raw caller-provided string.
    pub async fn execute_authenticated_peer(
        &self,
        request: ToolCallRequest,
        authenticated_sender: &str,
    ) -> Result<ToolCallResult, RuntimeError> {
        if !valid_principal_part(authenticated_sender) {
            return Err(RuntimeError::AuthRequired(
                "invalid authenticated sender".into(),
            ));
        }
        let principal = self
            .tenant_namespace
            .as_ref()
            .map(|tenant| format!("{tenant}:{authenticated_sender}"))
            .unwrap_or_else(|| "unattributed".to_owned());
        self.execute_attributed(request, &principal).await
    }

    /// Register only the manifest actually verified by the Ed25519 WASI
    /// publisher signature. Unsigned local executors are not accepted here.
    #[cfg(target_os = "linux")]
    pub async fn register_signed_wasi(
        &self,
        executor: crate::executor::wasi::WasmToolExecutor,
    ) -> Result<(), RegistryError> {
        let manifest = executor.signed_tool_manifest().cloned().ok_or_else(|| {
            RegistryError::InvalidManifest(
                "WASI registration requires a signed publisher manifest".into(),
            )
        })?;
        self.registry.register(manifest, executor).await
    }

    pub async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult, RuntimeError> {
        self.execute_attributed(request, "unattributed").await
    }

    async fn execute_attributed(
        &self,
        request: ToolCallRequest,
        principal: &str,
    ) -> Result<ToolCallResult, RuntimeError> {
        let start = Instant::now();
        let result = self.execute_inner(&request, start, principal).await;
        let elapsed_us = start.elapsed().as_micros().min(u64::MAX as u128) as u64;
        if result.is_ok() {
            self.metrics.record_success(elapsed_us).await;
        } else {
            self.metrics.record_failure(elapsed_us).await;
        }
        result
    }

    async fn execute_inner(
        &self,
        request: &ToolCallRequest,
        start: Instant,
        principal: &str,
    ) -> Result<ToolCallResult, RuntimeError> {
        let tool = self
            .registry
            .resolve(&request.tool)
            .await
            .ok_or_else(|| RuntimeError::ToolNotFound(request.tool.clone()))?;

        let manifest = &tool.manifest;
        if !manifest.enabled {
            return Err(RuntimeError::ToolDisabled(request.tool.clone()));
        }

        if !tool.input_validator.is_valid(&request.args) {
            return Err(RuntimeError::InvalidInput(
                "JSON value does not conform to the tool input schema".into(),
            ));
        }

        let timeout = Duration::from_millis(manifest.timeout_ms);
        let auth_result = tokio::time::timeout(
            timeout,
            AuthPolicy::authorize(
                &request.tool,
                manifest.cost_sats,
                &manifest.permissions,
                request.auth.as_ref(),
                self.auth_verifier.as_ref(),
            ),
        )
        .await
        .map_err(|_| RuntimeError::Timeout)?;

        auth_result.map_err(|error| RuntimeError::AuthRequired(error.to_string()))?;

        let mut reservation =
            BudgetReservation::reserve(Arc::clone(&self.budget_sats), manifest.cost_sats)?;

        let ctx = ExecutionContext {
            agent_id: principal.to_owned(),
            deadline_ms: manifest.timeout_ms,
            correlation_id: request.call_id.clone(),
        };

        let output =
            tokio::time::timeout(timeout, tool.executor.execute(request.args.clone(), &ctx))
                .await
                .map_err(|_| RuntimeError::Timeout)??;

        if !tool.output_validator.is_valid(&output) {
            return Err(RuntimeError::InvalidOutput(
                "JSON value does not conform to the tool output schema".into(),
            ));
        }

        reservation.commit();
        Ok(ToolCallResult {
            call_id: request.call_id.clone(),
            status: "SUCCESS".to_string(),
            output,
            execution_time_us: start.elapsed().as_micros().min(u64::MAX as u128) as u64,
            cost_sats: manifest.cost_sats,
        })
    }
}
