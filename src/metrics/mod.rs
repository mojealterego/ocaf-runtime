pub mod trace_context;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub total_latency_us: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MetricsStore {
    inner: Arc<RwLock<RuntimeMetrics>>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn record_success(&self, latency_us: u64) {
        let mut metrics = self.inner.write().await;
        metrics.total_calls = metrics.total_calls.saturating_add(1);
        metrics.successful_calls = metrics.successful_calls.saturating_add(1);
        metrics.total_latency_us = metrics.total_latency_us.saturating_add(latency_us);
    }

    pub async fn record_failure(&self, latency_us: u64) {
        let mut metrics = self.inner.write().await;
        metrics.total_calls = metrics.total_calls.saturating_add(1);
        metrics.failed_calls = metrics.failed_calls.saturating_add(1);
        metrics.total_latency_us = metrics.total_latency_us.saturating_add(latency_us);
    }

    pub async fn snapshot(&self) -> RuntimeMetrics {
        self.inner.read().await.clone()
    }
}
