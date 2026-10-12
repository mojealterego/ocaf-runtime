use std::collections::HashMap;

use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::rpc::{
    PeerPolicy, RpcAction, RpcError, RpcGateway, RpcReply, RpcRequest, RpcStatus, SignedFrame,
    MAX_FRAME_BYTES,
};
use ocaf_runtime::runtime::OcafRuntime;
use serde_json::{json, Value};

const KEY: [u8; 32] = [7u8; 32];
const BAD_KEY: [u8; 32] = [8u8; 32];
const NOW: u64 = 1_800_000_000_000;

struct Echo;

#[async_trait::async_trait]
impl ToolExecutor for Echo {
    async fn execute(&self, args: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        Ok(json!({"echo": args["message"]}))
    }
}

fn manifest(name: &str, price: u64, permissions: Vec<String>) -> ToolManifest {
    ToolManifest {
        name: name.into(),
        description: "safe test tool".into(),
        input_schema: json!({
            "type":"object",
            "properties":{"message":{"type":"string"}},
            "required":["message"],
            "additionalProperties":false
        }),
        output_schema: json!({
            "type":"object",
            "properties":{"echo":{"type":"string"}},
            "required":["echo"]
        }),
        cost_sats: price,
        permissions,
        timeout_ms: 200,
        enabled: true,
    }
}

async fn fixture() -> RpcGateway {
    let runtime = OcafRuntime::new(500);
    runtime
        .register(manifest("echo", 0, vec![]), Echo)
        .await
        .unwrap();
    runtime
        .register(manifest("paid", 25, vec![]), Echo)
        .await
        .unwrap();
    runtime
        .register(manifest("restricted", 0, vec!["fs:write".into()]), Echo)
        .await
        .unwrap();

    let peers = HashMap::from([(
        "agent-a".to_string(),
        PeerPolicy::new(KEY, vec!["echo".into(), "paid".into(), "restricted".into()]),
    )]);
    RpcGateway::new("agent-b", runtime, peers, 3).unwrap()
}

fn request(tool: &str) -> RpcRequest {
    RpcRequest::new(
        "agent-a",
        "agent-b",
        NOW,
        5_000,
        RpcAction::Execute {
            tool: tool.into(),
            args: json!({"message":"hi"}),
        },
    )
    .unwrap()
}

async fn invoke(gateway: &RpcGateway, request: &RpcRequest) -> Result<RpcReply, RpcError> {
    let bytes = SignedFrame::seal(request, &KEY).unwrap();
    let response = gateway.handle_at(&bytes, NOW).await?;
    SignedFrame::open(&response, &KEY)
}

#[tokio::test]
async fn signed_remote_execution_runs_whitelisted_free_tool() {
    let gateway = fixture().await;
    let result = invoke(&gateway, &request("echo")).await.unwrap();
    assert_eq!(result.status, RpcStatus::Ok);
    assert_eq!(result.output, json!({"echo":"hi"}));
    assert_eq!(result.sender, "agent-b");
    assert_eq!(result.recipient, "agent-a");
}

#[tokio::test]
async fn tampered_hmac_is_rejected_before_execution() {
    let gateway = fixture().await;
    let bytes = SignedFrame::seal(&request("echo"), &BAD_KEY).unwrap();
    assert!(matches!(
        gateway.handle_at(&bytes, NOW).await,
        Err(RpcError::Unauthorized)
    ));
    assert_eq!(gateway.runtime().metrics.snapshot().await.total_calls, 0);
}

#[tokio::test]
async fn a_reused_request_id_is_rejected() {
    let gateway = fixture().await;
    let bytes = SignedFrame::seal(&request("echo"), &KEY).unwrap();
    assert!(gateway.handle_at(&bytes, NOW).await.is_ok());
    assert!(matches!(
        gateway.handle_at(&bytes, NOW).await,
        Err(RpcError::Replay)
    ));
    assert_eq!(gateway.runtime().metrics.snapshot().await.total_calls, 1);
}

#[tokio::test]
async fn expired_and_future_requests_are_rejected() {
    let gateway = fixture().await;
    let expired = SignedFrame::seal(&request("echo"), &KEY).unwrap();
    assert!(matches!(
        gateway.handle_at(&expired, NOW + 5_001).await,
        Err(RpcError::Expired)
    ));
    let future = SignedFrame::seal(&request("echo"), &KEY).unwrap();
    assert!(matches!(
        gateway.handle_at(&future, NOW - 5_001).await,
        Err(RpcError::Expired)
    ));
}

#[tokio::test]
async fn sender_must_be_allowlisted_for_tool() {
    let gateway = fixture().await;
    let mut req = request("echo");
    req.action = RpcAction::Execute {
        tool: "not-listed".into(),
        args: json!({}),
    };
    let bytes = SignedFrame::seal(&req, &KEY).unwrap();
    assert!(matches!(
        gateway.handle_at(&bytes, NOW).await,
        Err(RpcError::Unauthorized)
    ));
}

#[tokio::test]
async fn remote_paid_and_permissioned_tools_are_blocked_even_with_valid_peer_signature() {
    let gateway = fixture().await;
    for tool in ["paid", "restricted"] {
        let bytes = SignedFrame::seal(&request(tool), &KEY).unwrap();
        assert!(matches!(
            gateway.handle_at(&bytes, NOW).await,
            Err(RpcError::Unauthorized)
        ));
    }
    assert_eq!(gateway.runtime().remaining_budget_sats(), 500);
}

#[tokio::test]
async fn request_for_different_node_is_rejected() {
    let gateway = fixture().await;
    let mut req = request("echo");
    req.recipient = "agent-c".into();
    let bytes = SignedFrame::seal(&req, &KEY).unwrap();
    assert!(matches!(
        gateway.handle_at(&bytes, NOW).await,
        Err(RpcError::WrongRecipient)
    ));
}

#[tokio::test]
async fn signed_discovery_only_exposes_permitted_safe_tools() {
    let gateway = fixture().await;
    let req = RpcRequest::new("agent-a", "agent-b", NOW, 5_000, RpcAction::Discover).unwrap();
    let reply = invoke(&gateway, &req).await.unwrap();
    assert_eq!(reply.status, RpcStatus::Ok);
    let tools = reply.output.as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "echo");
}

#[test]
fn changing_signed_response_fails_verification() {
    let req = RpcReply {
        version: 1,
        request_id: "test-123".into(),
        sender: "agent-b".into(),
        recipient: "agent-a".into(),
        status: RpcStatus::Ok,
        output: json!({"data":"original"}),
    };
    let mut signed = SignedFrame::seal(&req, &KEY).unwrap();
    let i = signed.iter().position(|b| *b == b'=').unwrap_or(1);
    signed[i] = if signed[i] == b'a' { b'b' } else { b'a' };
    assert!(SignedFrame::open::<RpcReply>(&signed, &KEY).is_err());
}

#[test]
fn oversized_envelope_is_rejected_before_json_parsing() {
    let oversized = vec![b'x'; MAX_FRAME_BYTES + 1];
    assert!(matches!(
        SignedFrame::open::<RpcRequest>(&oversized, &KEY),
        Err(RpcError::FrameTooLarge)
    ));
}

#[tokio::test]
async fn unknown_sender_is_rejected_without_fallback_key() {
    let gateway = fixture().await;
    let mut req = request("echo");
    req.sender = "intruder".into();
    let bytes = SignedFrame::seal(&req, &KEY).unwrap();
    assert!(matches!(
        gateway.handle_at(&bytes, NOW).await,
        Err(RpcError::Unauthorized)
    ));
}

#[test]
fn forbidden_node_identifiers_are_rejected() {
    assert!(RpcRequest::new("agent/../../a", "agent-b", NOW, 1000, RpcAction::Discover).is_err());
    assert!(RpcRequest::new("agent-a", "agent*", NOW, 1000, RpcAction::Discover).is_err());
}

#[test]
fn reply_verification_checks_caller_target_and_correlation() {
    use ocaf_runtime::rpc::RpcClient;
    let client = RpcClient::new("agent-a", "agent-b", KEY).unwrap();
    let request = request("echo");
    let response = RpcReply {
        version: 1,
        request_id: uuid::Uuid::new_v4().to_string(),
        sender: "agent-b".into(),
        recipient: "agent-a".into(),
        status: RpcStatus::Ok,
        output: json!({}),
    };
    let signed = SignedFrame::seal(&response, &KEY).unwrap();
    assert!(matches!(
        client.read_reply(&signed, &request),
        Err(RpcError::InvalidReply)
    ));
}

#[tokio::test]
async fn authenticated_peer_is_rate_limited_without_affecting_other_peers() {
    let runtime = OcafRuntime::new(0);
    runtime.register(manifest("echo", 0, vec![]), Echo).await.unwrap();
    let policy = PeerPolicy::new(KEY, vec!["echo".into()])
        .with_rate_limit(2)
        .unwrap();
    let peers = HashMap::from([
        ("agent-a".to_string(), policy),
        ("agent-c".to_string(), PeerPolicy::new(KEY, vec!["echo".into()])
            .with_rate_limit(2).unwrap()),
    ]);
    let gateway = RpcGateway::new("agent-b", runtime, peers, 8).unwrap();

    for _ in 0..2 {
        assert!(invoke(&gateway, &request("echo")).await.is_ok());
    }
    assert!(matches!(
        invoke(&gateway, &request("echo")).await,
        Err(RpcError::Busy)
    ));

    // A separate authenticated peer has an independent quota.
    let other = RpcRequest::new(
        "agent-c", "agent-b", NOW, 5000,
        RpcAction::Execute {
            tool: "echo".into(),
            args: json!({"message":"peer-c"})
        }
    ).unwrap();
    assert!(invoke(&gateway, &other).await.is_ok());

    // The original peer can send again in the next rate window.
    let next = RpcRequest::new(
        "agent-a", "agent-b", NOW + 1000, 5000,
        RpcAction::Execute {
            tool: "echo".into(),
            args: json!({"message":"next"})
        }
    ).unwrap();
    let next_bytes = SignedFrame::seal(&next, &KEY).unwrap();
    assert!(gateway.handle_at(&next_bytes, NOW + 1000).await.is_ok());
    assert_eq!(gateway.runtime().metrics.snapshot().await.total_calls, 4);
}

#[test]
fn invalid_rate_limit_configuration_is_rejected() {
    assert!(matches!(
        PeerPolicy::new(KEY, vec!["echo".into()]).with_rate_limit(0),
        Err(RpcError::Malformed)
    ));
}
