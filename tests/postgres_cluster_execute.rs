//! Regression: a clustered coordinator must never run unfenced direct tools.
use std::collections::HashMap;
use std::sync::Arc;

use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcError, RpcGateway, RpcRequest, SignedFrame};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::{json, Value};

const KEY: [u8; 32] = [0x77; 32];
const NOW: u64 = 1_800_000_000_000;

struct UnsafeMock;
#[async_trait::async_trait]
impl ToolExecutor for UnsafeMock {
    async fn execute(&self, _: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        Ok(json!({"side_effect":"executed"}))
    }
}

fn dsn() -> Option<String> {
    let v = std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(v.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    v
}

#[tokio::test]
async fn valid_signed_execute_is_denied_in_cluster_before_tool_invocation() {
    let Some(url) = dsn() else { return };
    let cluster = format!("direct_{}", uuid::Uuid::new_v4().simple());
    let leader = Arc::new(PgLeader::connect(&url, &cluster, true).await.unwrap());
    assert!(leader.try_acquire().await.unwrap());
    let runtime = OcafRuntime::new(0);
    runtime
        .register(
            ToolManifest {
                name: "unsafe_mock".into(),
                description: "intentional mock of an effectful tool".into(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                cost_sats: 0,
                permissions: vec![],
                timeout_ms: 1000,
                enabled: true,
            },
            UnsafeMock,
        )
        .await
        .unwrap();
    let gateway = RpcGateway::new(
        "coord",
        runtime,
        HashMap::from([(
            "remote".into(),
            PeerPolicy::new(KEY, vec!["unsafe_mock".into()]),
        )]),
        4,
    )
    .unwrap()
    .with_pg_leader(Arc::clone(&leader));

    let req = RpcRequest::new(
        "remote",
        "coord",
        NOW,
        5000,
        RpcAction::Execute {
            tool: "unsafe_mock".into(),
            args: json!({}),
        },
    )
    .unwrap();
    let frame = SignedFrame::seal(&req, &KEY).unwrap();
    assert!(matches!(
        gateway.handle_at(&frame, NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert_eq!(gateway.runtime().metrics.snapshot().await.total_calls, 0);
    leader.release().await.unwrap();
}
