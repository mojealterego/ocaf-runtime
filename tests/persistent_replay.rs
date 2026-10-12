use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcError, RpcGateway, RpcRequest, SignedFrame};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::{json, Value};

const KEY: [u8; 32] = [13; 32];
const WRONG_KEY: [u8; 32] = [14; 32];
const NOW: u64 = 1_800_000_000_000;

struct CountingEcho(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl ToolExecutor for CountingEcho {
    async fn execute(&self, args: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(json!({ "echo": args["message"] }))
    }
}

fn manifest() -> ToolManifest {
    ToolManifest {
        name: "echo".into(),
        description: "persistent replay regression tool".into(),
        input_schema: json!({
            "type": "object",
            "required": ["message"],
            "properties": {"message": {"type": "string"}}
        }),
        output_schema: json!({
            "type": "object",
            "required": ["echo"],
            "properties": {"echo": {"type": "string"}}
        }),
        cost_sats: 0,
        permissions: vec![],
        timeout_ms: 1000,
        enabled: true,
    }
}

async fn gateway(path: &Path, counter: Arc<AtomicUsize>) -> Result<RpcGateway, RpcError> {
    let runtime = OcafRuntime::new(0);
    runtime
        .register(manifest(), CountingEcho(counter))
        .await
        .unwrap();
    let peers = HashMap::from([("client-a".into(), PeerPolicy::new(KEY, vec!["echo".into()]))]);
    RpcGateway::with_sqlite_replay("server-b", runtime, peers, 8, path)
}

fn signed_request() -> Vec<u8> {
    let req = RpcRequest::new(
        "client-a",
        "server-b",
        NOW,
        5000,
        RpcAction::Execute {
            tool: "echo".into(),
            args: json!({"message":"once"}),
        },
    )
    .unwrap();
    SignedFrame::seal(&req, &KEY).unwrap()
}

#[tokio::test]
async fn persistent_replay_survives_gateway_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("rpc.sqlite");
    let count = Arc::new(AtomicUsize::new(0));
    let request = signed_request();

    let first = gateway(&db, Arc::clone(&count)).await.unwrap();
    assert!(first.handle_at(&request, NOW).await.is_ok());
    drop(first);

    let restarted = gateway(&db, Arc::clone(&count)).await.unwrap();
    assert!(matches!(
        restarted.handle_at(&request, NOW).await,
        Err(RpcError::Replay)
    ));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn two_gateways_sharing_one_database_execute_a_request_at_most_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("shared.sqlite");
    let count = Arc::new(AtomicUsize::new(0));
    let first = gateway(&db, Arc::clone(&count)).await.unwrap();
    let second = gateway(&db, Arc::clone(&count)).await.unwrap();
    let request = signed_request();

    let (a, b) = tokio::join!(
        first.handle_at(&request, NOW),
        second.handle_at(&request, NOW)
    );
    let accepted = usize::from(a.is_ok()) + usize::from(b.is_ok());
    let rejected = usize::from(matches!(a, Err(RpcError::Replay)))
        + usize::from(matches!(b, Err(RpcError::Replay)));
    assert_eq!(accepted, 1);
    assert_eq!(rejected, 1);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn invalid_database_directory_fails_closed_at_startup() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("nonexistent").join("replay.sqlite");
    let result = gateway(&db, Arc::new(AtomicUsize::new(0))).await;
    assert!(matches!(result, Err(RpcError::Unavailable)));
}

#[tokio::test]
async fn bad_signature_cannot_poison_the_persistent_replay_journal() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("replay.sqlite");
    let count = Arc::new(AtomicUsize::new(0));
    let gateway = gateway(&db, Arc::clone(&count)).await.unwrap();
    let request = RpcRequest::new(
        "client-a",
        "server-b",
        NOW,
        5000,
        RpcAction::Execute {
            tool: "echo".into(),
            args: json!({"message":"legitimate"}),
        },
    )
    .unwrap();
    let forged = SignedFrame::seal(&request, &WRONG_KEY).unwrap();
    let legitimate = SignedFrame::seal(&request, &KEY).unwrap();

    assert!(matches!(
        gateway.handle_at(&forged, NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert!(gateway.handle_at(&legitimate, NOW).await.is_ok());
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn rebooted_gateway_still_denies_replay_after_remote_tool_failed() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("replay.sqlite");
    let count = Arc::new(AtomicUsize::new(0));
    let mut req = RpcRequest::new(
        "client-a",
        "server-b",
        NOW,
        5000,
        RpcAction::Execute {
            tool: "echo".into(),
            args: json!({"message":7}),
        },
    )
    .unwrap();
    let payload = SignedFrame::seal(&req, &KEY).unwrap();
    let first = gateway(&db, Arc::clone(&count)).await.unwrap();
    let reply = first.handle_at(&payload, NOW).await.unwrap();
    let reply: ocaf_runtime::rpc::RpcReply = SignedFrame::open(&reply, &KEY).unwrap();
    assert_eq!(reply.status, ocaf_runtime::rpc::RpcStatus::InvalidInput);
    drop(first);

    let restarted = gateway(&db, Arc::clone(&count)).await.unwrap();
    assert!(matches!(
        restarted.handle_at(&payload, NOW).await,
        Err(RpcError::Replay)
    ));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    // A new UUID is a distinct request, even if its tool and arguments match.
    req.request_id = uuid::Uuid::new_v4().to_string();
    let distinct = SignedFrame::seal(&req, &KEY).unwrap();
    assert!(restarted.handle_at(&distinct, NOW).await.is_ok());
}
