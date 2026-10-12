#![cfg(feature = "zenoh")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::rpc::{RemoteJobOutcome, RpcAction};
use ocaf_runtime::transport::rpc::{can_retry_rpc_action, ReconnectPolicy};

#[test]
fn automatic_retry_is_strictly_limited_to_read_only_or_idempotent_jobs() {
    let id = uuid::Uuid::new_v4().to_string();
    for action in [
        RpcAction::Discover,
        RpcAction::Resolve {
            tool: "demo_echo".into(),
        },
        RpcAction::GetJob { id: id.clone() },
        RpcAction::GetJobAudit {
            id: id.clone(),
            limit: 20,
        },
        RpcAction::SubmitJob {
            id: id.clone(),
            tool: "demo_echo".into(),
            args: serde_json::json!({"message":"stable"}),
        },
    ] {
        assert!(
            can_retry_rpc_action(&action),
            "safe operation must be retryable"
        );
    }
    for action in [
        RpcAction::Execute {
            tool: "demo_echo".into(),
            args: serde_json::json!({}),
        },
        RpcAction::ClaimJob,
        RpcAction::FinishJob {
            lease: ocaf_runtime::jobs::LeasedJob {
                id: id.clone(),
                tool: "demo_echo".into(),
                args: serde_json::json!({}),
                attempt: 1,
                worker_id: "worker".into(),
                lease_token: "token".into(),
                lease_expires_at_ms: 100,
            },
            outcome: RemoteJobOutcome::Uncertain,
        },
        RpcAction::Heartbeat {
            tools: vec!["demo_echo".into()],
        },
    ] {
        assert!(
            !can_retry_rpc_action(&action),
            "side-effect/lease operation cannot retry"
        );
    }
}

#[test]
fn backoff_policy_rejects_unbounded_or_invalid_configurations() {
    assert!(ReconnectPolicy::new(
        0,
        Duration::from_millis(1),
        Duration::from_millis(1),
        Duration::from_secs(1)
    )
    .is_err());
    assert!(ReconnectPolicy::new(
        9,
        Duration::from_millis(1),
        Duration::from_millis(1),
        Duration::from_secs(1)
    )
    .is_err());
    assert!(ReconnectPolicy::new(
        3,
        Duration::ZERO,
        Duration::from_millis(1),
        Duration::from_secs(1)
    )
    .is_err());
    assert!(ReconnectPolicy::new(
        3,
        Duration::from_secs(2),
        Duration::from_secs(1),
        Duration::from_secs(1)
    )
    .is_err());
    assert!(ReconnectPolicy::new(
        3,
        Duration::from_millis(1),
        Duration::from_secs(2),
        Duration::ZERO
    )
    .is_err());
    assert!(ReconnectPolicy::new(
        3,
        Duration::from_millis(10),
        Duration::from_millis(100),
        Duration::from_secs(1)
    )
    .is_ok());
}

#[tokio::test]
async fn ambiguous_transport_failure_retries_safe_operation_with_a_fresh_attempt() {
    let p = ReconnectPolicy::new(
        4,
        Duration::from_millis(1),
        Duration::from_millis(2),
        Duration::from_millis(100),
    )
    .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let answer = p
        .run(
            &RpcAction::GetJob {
                id: uuid::Uuid::new_v4().to_string(),
            },
            move || {
                let nth = counter.fetch_add(1, Ordering::SeqCst) + 1;
                async move {
                    if nth < 3 {
                        anyhow::bail!("standby promotion in progress")
                    }
                    Ok::<usize, anyhow::Error>(nth)
                }
            },
        )
        .await
        .unwrap();
    assert_eq!(answer, 3);
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn unsafe_execute_may_not_be_automatically_reissued_after_lost_reply() {
    let p = ReconnectPolicy::new(
        8,
        Duration::from_millis(1),
        Duration::from_millis(2),
        Duration::from_millis(100),
    )
    .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let action = RpcAction::Execute {
        tool: "demo_echo".into(),
        args: serde_json::json!({}),
    };
    let result: anyhow::Result<()> = p
        .run(&action, move || {
            counter.fetch_add(1, Ordering::SeqCst);
            async { anyhow::bail!("result might have committed") }
        })
        .await;
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn retry_count_and_per_attempt_timeout_are_enforced() {
    let p = ReconnectPolicy::new(
        3,
        Duration::from_millis(1),
        Duration::from_millis(2),
        Duration::from_millis(10),
    )
    .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let start = tokio::time::Instant::now();
    let result: anyhow::Result<()> = p
        .run(&RpcAction::Discover, move || {
            counter.fetch_add(1, Ordering::SeqCst);
            async {
                tokio::time::sleep(Duration::from_millis(150)).await;
                Ok(())
            }
        })
        .await;
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert!(start.elapsed() < Duration::from_secs(1));
}
