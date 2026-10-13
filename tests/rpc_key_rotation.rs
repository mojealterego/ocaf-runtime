//! Priority 0.35: authenticated per-peer HMAC key cutover, without
//! pretending the Zenoh transport exposes a verified certificate identity.
use ocaf_runtime::rpc::{
    PeerPolicy, RpcAction, RpcError, RpcGateway, RpcReply, RpcRequest, RpcStatus, SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use std::collections::HashMap;

const OLD: [u8; 32] = [0x31; 32];
const NEW: [u8; 32] = [0x32; 32];
const OTHER: [u8; 32] = [0x33; 32];
const ACTIVATE: u64 = 1_800_000_000_000;
const RETIRE: u64 = ACTIVATE + 60_000;

fn gateway() -> RpcGateway {
    let policy = PeerPolicy::new(OLD, vec!["echo".into()])
        .with_key_rotation(NEW, ACTIVATE, RETIRE)
        .expect("valid bounded rotation");
    RpcGateway::new(
        "coordinator",
        OcafRuntime::new(0),
        HashMap::from([("agent-a".to_string(), policy)]),
        8,
    )
    .expect("valid gateway")
}

fn request(now: u64) -> RpcRequest {
    RpcRequest::new("agent-a", "coordinator", now, 5_000, RpcAction::Discover).unwrap()
}

async fn challenge(gateway: &RpcGateway, key: &[u8; 32], now: u64) -> Result<Vec<u8>, RpcError> {
    gateway
        .handle_at(&SignedFrame::seal(&request(now), key).unwrap(), now)
        .await
}

#[tokio::test]
async fn prior_to_activation_only_old_key_authenticates() {
    let gateway = gateway();
    let before = ACTIVATE - 1;
    let reply = challenge(&gateway, &OLD, before).await.unwrap();
    let parsed: RpcReply = SignedFrame::open(&reply, &OLD).unwrap();
    assert_eq!(parsed.status, RpcStatus::Ok);
    assert!(matches!(
        challenge(&gateway, &NEW, before).await,
        Err(RpcError::Unauthorized)
    ));
}

#[tokio::test]
async fn during_overlap_both_keys_work_but_replies_match_request_key() {
    let gateway = gateway();
    let now = ACTIVATE + 10;
    let old_reply = challenge(&gateway, &OLD, now).await.unwrap();
    let new_reply = challenge(&gateway, &NEW, now).await.unwrap();
    let _: RpcReply = SignedFrame::open(&old_reply, &OLD).unwrap();
    let _: RpcReply = SignedFrame::open(&new_reply, &NEW).unwrap();
    assert!(SignedFrame::open::<RpcReply>(&old_reply, &NEW).is_err());
    assert!(SignedFrame::open::<RpcReply>(&new_reply, &OLD).is_err());
}

#[tokio::test]
async fn retired_key_is_rejected_at_exact_deadline_and_new_key_remains_valid() {
    let gateway = gateway();
    assert!(challenge(&gateway, &OLD, RETIRE - 1).await.is_ok());
    assert!(matches!(
        challenge(&gateway, &OLD, RETIRE).await,
        Err(RpcError::Unauthorized)
    ));
    assert!(challenge(&gateway, &NEW, RETIRE).await.is_ok());
    assert!(challenge(&gateway, &NEW, RETIRE + 1).await.is_ok());
}

#[tokio::test]
async fn arbitrary_key_never_authenticates_or_consumes_replay_nonce() {
    let gateway = gateway();
    let now = ACTIVATE + 20;
    let request = request(now);
    let unknown = SignedFrame::seal(&request, &OTHER).unwrap();
    assert!(matches!(
        gateway.handle_at(&unknown, now).await,
        Err(RpcError::Unauthorized)
    ));
    let authorized = SignedFrame::seal(&request, &NEW).unwrap();
    assert!(gateway.handle_at(&authorized, now).await.is_ok());
    // Both old and new use the same per-sender replay domain.
    assert!(matches!(
        gateway
            .handle_at(&SignedFrame::seal(&request, &OLD).unwrap(), now)
            .await,
        Err(RpcError::Replay)
    ));
}

#[test]
fn refuses_rotation_reusing_old_secret_or_zero_key() {
    for next in [OLD, [0; 32]] {
        assert!(matches!(
            PeerPolicy::new(OLD, vec!["echo".into()]).with_key_rotation(next, ACTIVATE, RETIRE),
            Err(RpcError::Malformed)
        ));
    }
}

#[test]
fn refuses_invalid_or_unbounded_credential_overlap() {
    for (start, end) in [
        (0, RETIRE),
        (ACTIVATE, ACTIVATE),
        (ACTIVATE + 1, ACTIVATE),
        (ACTIVATE, ACTIVATE + 1_800_001),
        (u64::MAX, 1),
    ] {
        assert!(matches!(
            PeerPolicy::new(OLD, vec!["echo".into()]).with_key_rotation(NEW, start, end),
            Err(RpcError::Malformed)
        ));
    }
}

#[tokio::test]
async fn no_rotation_keeps_previous_hmac_protocol_behavior() {
    let gateway = RpcGateway::new(
        "coordinator",
        OcafRuntime::new(0),
        HashMap::from([("agent-a".to_string(), PeerPolicy::new(OLD, vec![]))]),
        8,
    )
    .unwrap();
    assert!(challenge(&gateway, &OLD, RETIRE + 1).await.is_ok());
    assert!(matches!(
        challenge(&gateway, &NEW, RETIRE + 1).await,
        Err(RpcError::Unauthorized)
    ));
}
