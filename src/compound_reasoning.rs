//! Bounded multi-model deliberation with strict-majority early exit.
//! This is an inference orchestration primitive, never a truth oracle or an
//! authorization boundary. External model calls require separate tool policy.
use async_trait::async_trait;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::{task::JoinSet, time::timeout};

#[derive(Clone, Debug)]
pub struct DeliberationRequest {
    pub tenant: String,
    pub prompt: String,
    pub models: Vec<String>,
    pub quorum: usize,
    pub max_parallel: usize,
    pub per_model_timeout_ms: u64,
    pub total_timeout_ms: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeliberationError {
    #[error("invalid tenant, model allowlist or quorum")]
    Invalid,
    #[error("resource budget exceeded")]
    Limit,
    #[error("deliberation deadline expired")]
    Timeout,
    #[error("no strict majority was established")]
    NoConsensus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliberationResult {
    pub answer: String,
    pub votes: usize,
    pub required_quorum: usize,
}

#[async_trait]
pub trait CandidateEngine: Send + Sync {
    async fn infer(&self, tenant: &str, model: &str, prompt: &str) -> Result<String, String>;
}

pub struct CompoundReasoningEngine {
    adapter: Arc<dyn CandidateEngine>,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
}

impl CompoundReasoningEngine {
    pub fn new(adapter: Arc<dyn CandidateEngine>) -> Self {
        Self { adapter }
    }

    pub async fn deliberate(
        &self,
        req: DeliberationRequest,
    ) -> Result<DeliberationResult, DeliberationError> {
        if !valid_id(&req.tenant)
            || req.models.is_empty()
            || req.models.iter().any(|id| !valid_id(id))
            || req.models.iter().collect::<BTreeSet<_>>().len() != req.models.len()
            || req.quorum <= req.models.len() / 2
            || req.quorum > req.models.len()
        {
            return Err(DeliberationError::Invalid);
        }
        if req.models.len() > 16
            || req.prompt.is_empty()
            || req.prompt.len() > 8192
            || req.max_parallel == 0
            || req.max_parallel > 4
            || req.total_timeout_ms == 0
            || req.total_timeout_ms > 60000
            || req.per_model_timeout_ms == 0
            || req.per_model_timeout_ms > req.total_timeout_ms
        {
            return Err(DeliberationError::Limit);
        }
        let deadline = Duration::from_millis(req.total_timeout_ms);
        let per_model = Duration::from_millis(req.per_model_timeout_ms);
        let adapter = self.adapter.clone();
        let future = async move {
            let mut running = JoinSet::new();
            let mut pending = req.models.into_iter();
            let mut votes: BTreeMap<String, usize> = BTreeMap::new();
            loop {
                while running.len() < req.max_parallel {
                    let Some(model) = pending.next() else { break };
                    let tenant = req.tenant.clone();
                    let prompt = req.prompt.clone();
                    let backend = adapter.clone();
                    running.spawn(async move {
                        timeout(per_model, backend.infer(&tenant, &model, &prompt)).await
                    });
                }
                let Some(result) = running.join_next().await else {
                    break;
                };
                if let Ok(Ok(Ok(answer))) = result {
                    if answer.is_empty()
                        || answer.len() > 8192
                        || answer.chars().any(char::is_control)
                    {
                        continue;
                    }
                    let count = votes.entry(answer.clone()).or_default();
                    *count += 1;
                    if *count >= req.quorum {
                        running.abort_all();
                        return Ok(DeliberationResult {
                            answer,
                            votes: *count,
                            required_quorum: req.quorum,
                        });
                    }
                }
            }
            Err(DeliberationError::NoConsensus)
        };
        timeout(deadline, future)
            .await
            .map_err(|_| DeliberationError::Timeout)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Engine {
        active: AtomicUsize,
        peak: AtomicUsize,
    }
    #[async_trait]
    impl CandidateEngine for Engine {
        async fn infer(&self, _tenant: &str, model: &str, _prompt: &str) -> Result<String, String> {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            tokio::task::yield_now().await;
            self.active.fetch_sub(1, Ordering::SeqCst);
            if model == "dissent" {
                Ok("no".into())
            } else {
                Ok("yes".into())
            }
        }
    }
    fn request() -> DeliberationRequest {
        DeliberationRequest {
            tenant: "tenant-a".into(),
            prompt: "query".into(),
            models: vec!["agree1".into(), "dissent".into(), "agree2".into()],
            quorum: 2,
            max_parallel: 2,
            per_model_timeout_ms: 100,
            total_timeout_ms: 1000,
        }
    }
    #[tokio::test]
    async fn quorum_and_parallel_budget() {
        let backend = Arc::new(Engine {
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        });
        let service = CompoundReasoningEngine::new(backend.clone());
        let out = service.deliberate(request()).await.unwrap();
        assert_eq!(out.answer, "yes");
        assert_eq!(out.votes, 2);
        assert!(backend.peak.load(Ordering::SeqCst) <= 2);
    }
    #[tokio::test]
    async fn rejects_duplicate_or_nonmajority_configs() {
        let backend = Arc::new(Engine {
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        });
        let service = CompoundReasoningEngine::new(backend);
        let mut req = request();
        req.models[1] = "agree1".into();
        assert_eq!(
            service.deliberate(req).await,
            Err(DeliberationError::Invalid)
        );
        let mut req = request();
        req.quorum = 1;
        assert_eq!(
            service.deliberate(req).await,
            Err(DeliberationError::Invalid)
        );
        let mut req = request();
        req.models = vec!["agree1".into(), "dissent".into()];
        req.quorum = 2;
        assert_eq!(
            service.deliberate(req).await,
            Err(DeliberationError::NoConsensus)
        );
    }
}
