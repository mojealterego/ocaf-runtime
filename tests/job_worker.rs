use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::jobs::worker::JobWorker;
use ocaf_runtime::jobs::{JobSpec, JobStatus, JobStore};
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::{json, Value};
use uuid::Uuid;

struct Echo(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl ToolExecutor for Echo {
    async fn execute(&self, args: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(json!({"echo":args["message"]}))
    }
}

fn manifest(name: &str, cost: u64, permissions: Vec<String>, timeout_ms: u64) -> ToolManifest {
    ToolManifest {
        name: name.into(),
        description: "job worker fixture".into(),
        input_schema: json!({"type":"object","required":["message"],
            "properties":{"message":{"type":"string"}}}),
        output_schema: json!({"type":"object","required":["echo"],
            "properties":{"echo":{"type":"string"}}}),
        cost_sats: cost,
        permissions,
        timeout_ms,
        enabled: true,
    }
}

fn job(name: &str) -> JobSpec {
    JobSpec {
        id: Uuid::new_v4().to_string(),
        tool: name.to_string(),
        args: json!({"message":"one"}),
        retry_safe: false,
        max_attempts: 1,
    }
}

#[tokio::test]
async fn worker_executes_validated_local_tool_and_persists_output() {
    let dir = tempfile::tempdir().unwrap();
    let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
    let runtime = OcafRuntime::new(0);
    let calls = Arc::new(AtomicUsize::new(0));
    runtime
        .register(manifest("echo", 0, vec![], 100), Echo(Arc::clone(&calls)))
        .await
        .unwrap();
    let work = job("echo");
    store.submit(work.clone()).await.unwrap();

    let worker = JobWorker::new(
        store.clone(),
        runtime,
        "agent-worker",
        10_000,
        HashSet::from(["echo".to_string()]),
    )
    .unwrap();
    assert_eq!(worker.run_once().await.unwrap(), Some(JobStatus::Succeeded));
    let record = store.get(&work.id).await.unwrap().unwrap();
    assert_eq!(record.result, Some(json!({"echo":"one"})));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(worker.run_once().await.unwrap(), None);
}

#[tokio::test]
async fn worker_cannot_bypass_local_authorization_for_paid_or_privileged_tools() {
    let dir = tempfile::tempdir().unwrap();
    let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
    let runtime = OcafRuntime::new(100);
    let calls = Arc::new(AtomicUsize::new(0));
    runtime
        .register(manifest("paid", 5, vec![], 100), Echo(Arc::clone(&calls)))
        .await
        .unwrap();
    runtime
        .register(
            manifest("privileged", 0, vec!["network:write".into()], 100),
            Echo(Arc::clone(&calls)),
        )
        .await
        .unwrap();
    let first = job("paid");
    let second = job("privileged");
    store.submit(first.clone()).await.unwrap();
    store.submit(second.clone()).await.unwrap();

    let worker = JobWorker::new(
        store.clone(),
        runtime.clone(),
        "agent-worker",
        10_000,
        HashSet::from(["paid".into(), "privileged".into()]),
    )
    .unwrap();
    assert_eq!(worker.run_once().await.unwrap(), Some(JobStatus::Failed));
    assert_eq!(worker.run_once().await.unwrap(), Some(JobStatus::Failed));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.remaining_budget_sats(), 100);
}

#[tokio::test]
async fn worker_rejects_tool_timeout_longer_than_lease_safety_margin() {
    let dir = tempfile::tempdir().unwrap();
    let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
    let runtime = OcafRuntime::new(0);
    let calls = Arc::new(AtomicUsize::new(0));
    runtime
        .register(manifest("slow", 0, vec![], 9_500), Echo(Arc::clone(&calls)))
        .await
        .unwrap();
    store.submit(job("slow")).await.unwrap();

    let worker = JobWorker::new(
        store,
        runtime,
        "agent-worker",
        10_000,
        HashSet::from(["slow".into()]),
    )
    .unwrap();
    assert_eq!(worker.run_once().await.unwrap(), Some(JobStatus::Failed));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn worker_rejects_empty_allowlist_or_invalid_worker_identity() {
    let dir = tempfile::tempdir().unwrap();
    let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
    let runtime = OcafRuntime::new(0);
    assert!(JobWorker::new(
        store.clone(),
        runtime.clone(),
        "valid",
        10_000,
        HashSet::new()
    )
    .is_err());
    assert!(JobWorker::new(
        store,
        runtime,
        "../../worker",
        10_000,
        HashSet::from(["echo".into()])
    )
    .is_err());
}

#[tokio::test]
async fn worker_recovers_expired_retry_safe_job_without_manual_supervisor_step() {
    let dir = tempfile::tempdir().unwrap();
    let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
    let runtime = OcafRuntime::new(0);
    let calls = Arc::new(AtomicUsize::new(0));
    runtime
        .register(manifest("echo", 0, vec![], 100), Echo(Arc::clone(&calls)))
        .await
        .unwrap();
    let mut work = job("echo");
    work.retry_safe = true;
    work.max_attempts = 2;
    store.submit(work.clone()).await.unwrap();
    let now = ocaf_runtime::rpc::now_ms().unwrap();
    let _abandoned = store
        .claim("crashed-agent", now - 5_000, 100)
        .await
        .unwrap()
        .unwrap();
    let worker = JobWorker::new(
        store.clone(),
        runtime,
        "replacement-agent",
        10_000,
        HashSet::from(["echo".into()]),
    )
    .unwrap();
    assert_eq!(worker.run_once().await.unwrap(), Some(JobStatus::Succeeded));
    assert_eq!(store.get(&work.id).await.unwrap().unwrap().attempts, 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

struct PartialSideEffect(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl ToolExecutor for PartialSideEffect {
    async fn execute(&self, _: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(ToolError::Execution(
            "effect happened before failure".into(),
        ))
    }
}

#[tokio::test]
async fn ambiguous_tool_failure_is_uncertain_for_non_retry_safe_job() {
    let dir = tempfile::tempdir().unwrap();
    let store = JobStore::open(&dir.path().join("jobs.sqlite")).unwrap();
    let runtime = OcafRuntime::new(0);
    let effects = Arc::new(AtomicUsize::new(0));
    runtime
        .register(
            manifest("side_effect", 0, vec![], 100),
            PartialSideEffect(Arc::clone(&effects)),
        )
        .await
        .unwrap();
    let task = job("side_effect");
    store.submit(task.clone()).await.unwrap();
    let worker = JobWorker::new(
        store.clone(),
        runtime,
        "worker-a",
        10_000,
        HashSet::from(["side_effect".into()]),
    )
    .unwrap();
    assert_eq!(worker.run_once().await.unwrap(), Some(JobStatus::Uncertain));
    assert_eq!(
        store.get(&task.id).await.unwrap().unwrap().status,
        JobStatus::Uncertain
    );
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    assert_eq!(worker.run_once().await.unwrap(), None);
}
