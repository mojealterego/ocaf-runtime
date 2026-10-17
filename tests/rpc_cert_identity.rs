//! Priority 0.36 — signed-RPC certificate-key possession regression.
//! mTLS TLS-exporter channel binding remains a separate production gate.
use std::collections::HashMap;

use ocaf_runtime::rpc::{
    PeerPolicy, RpcAction, RpcError, RpcGateway, RpcReply, RpcRequest, RpcStatus, SignedFrame,
};
use ocaf_runtime::runtime::OcafRuntime;
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::x509::{X509NameBuilder, X509};
use sha2::{Digest, Sha256};

const KEY: [u8; 32] = [0x36; 32];
const NOW: u64 = 1_800_000_000_000;

fn credential() -> (Vec<u8>, Vec<u8>, String) {
    let ec = EcKey::generate(&EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap()).unwrap();
    let key = PKey::from_ec_key(ec).unwrap();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", "ocaf-test-agent").unwrap();
    let name = name.build();
    let serial = openssl::asn1::Asn1Integer::from_bn(&BigNum::from_u32(1).unwrap()).unwrap();
    let mut cert = X509::builder().unwrap();
    cert.set_version(2).unwrap();
    cert.set_serial_number(&serial).unwrap();
    cert.set_subject_name(&name).unwrap();
    cert.set_issuer_name(&name).unwrap();
    cert.set_pubkey(&key).unwrap();
    cert.set_not_before(Asn1Time::days_from_now(0).unwrap().as_ref())
        .unwrap();
    cert.set_not_after(Asn1Time::days_from_now(1).unwrap().as_ref())
        .unwrap();
    cert.sign(&key, MessageDigest::sha256()).unwrap();
    let certificate = cert.build();
    let pin = format!("{:x}", Sha256::digest(certificate.to_der().unwrap()));
    (
        certificate.to_pem().unwrap(),
        key.private_key_to_pem_pkcs8().unwrap(),
        pin,
    )
}

fn gateway(cert: &[u8], pin: &str) -> RpcGateway {
    let peer = PeerPolicy::new(KEY, vec![])
        .with_pinned_certificate(cert, pin)
        .unwrap();
    RpcGateway::new(
        "node-b",
        OcafRuntime::new(0),
        HashMap::from([("node-a".to_string(), peer)]),
        4,
    )
    .unwrap()
}

fn request() -> RpcRequest {
    RpcRequest::new("node-a", "node-b", NOW, 5_000, RpcAction::Discover).unwrap()
}

async fn send(gateway: &RpcGateway, request: &RpcRequest) -> Result<Vec<u8>, RpcError> {
    let frame = SignedFrame::seal(request, &KEY).unwrap();
    gateway.handle_at(&frame, NOW).await
}

#[tokio::test]
async fn signed_certificate_private_key_and_hmac_both_required() {
    let (pem, key, pin) = credential();
    let g = gateway(&pem, &pin);
    let signed = request().with_certificate_proof(&key).unwrap();
    let reply = send(&g, &signed).await.unwrap();
    let response: RpcReply = SignedFrame::open(&reply, &KEY).unwrap();
    assert_eq!(response.status, RpcStatus::Ok);
}

#[tokio::test]
async fn valid_hmac_without_certificate_signature_fails_closed() {
    let (pem, _, pin) = credential();
    let g = gateway(&pem, &pin);
    assert!(matches!(
        send(&g, &request()).await,
        Err(RpcError::Unauthorized)
    ));
}

#[tokio::test]
async fn another_cert_private_key_is_not_authenticated() {
    let (pem, _, pin) = credential();
    let (_, wrong_key, _) = credential();
    let g = gateway(&pem, &pin);
    let candidate = request().with_certificate_proof(&wrong_key).unwrap();
    assert!(matches!(
        send(&g, &candidate).await,
        Err(RpcError::Unauthorized)
    ));
}

#[tokio::test]
async fn signature_cannot_be_reused_with_mutated_action_or_recipient() {
    let (pem, key, pin) = credential();
    let g = gateway(&pem, &pin);
    let mut altered = request().with_certificate_proof(&key).unwrap();
    altered.action = RpcAction::Resolve {
        tool: "demo_echo".into(),
    };
    assert!(matches!(
        send(&g, &altered).await,
        Err(RpcError::Unauthorized)
    ));
    let mut altered = request().with_certificate_proof(&key).unwrap();
    altered.recipient = "node-c".into();
    assert!(send(&g, &altered).await.is_err());
}

#[tokio::test]
async fn certificate_fingerprint_mismatch_refused_at_policy_creation() {
    let (pem, _, _) = credential();
    assert!(matches!(
        PeerPolicy::new(KEY, vec![]).with_pinned_certificate(&pem, &"0".repeat(64)),
        Err(RpcError::Unauthorized)
    ));
}

#[tokio::test]
async fn forged_signature_does_not_consume_replay_nonce() {
    let (pem, key, pin) = credential();
    let (_, wrong_key, _) = credential();
    let g = gateway(&pem, &pin);
    let initial = request();
    let wrong = initial.clone().with_certificate_proof(&wrong_key).unwrap();
    assert!(matches!(
        send(&g, &wrong).await,
        Err(RpcError::Unauthorized)
    ));
    let right = initial.with_certificate_proof(&key).unwrap();
    assert!(send(&g, &right).await.is_ok());
    assert!(matches!(send(&g, &right).await, Err(RpcError::Replay)));
}

#[tokio::test]
async fn extra_certificate_proof_without_policy_is_rejected() {
    let (_, key, _) = credential();
    let g = RpcGateway::new(
        "node-b",
        OcafRuntime::new(0),
        HashMap::from([("node-a".into(), PeerPolicy::new(KEY, vec![]))]),
        4,
    )
    .unwrap();
    let signed = request().with_certificate_proof(&key).unwrap();
    assert!(matches!(
        send(&g, &signed).await,
        Err(RpcError::Unauthorized)
    ));
}

#[test]
fn client_preparation_attaches_certificate_proof_when_enabled() {
    let (_, key, _) = credential();
    let client = ocaf_runtime::rpc::RpcClient::new("node-a", "node-b", KEY)
        .unwrap()
        .with_certificate_private_key(key)
        .unwrap();
    let (req, signed) = client.prepare(RpcAction::Discover, 5_000).unwrap();
    assert!(req.certificate_proof_b64.is_some());
    let parsed: RpcRequest = SignedFrame::open(&signed, &KEY).unwrap();
    assert_eq!(parsed.request_id, req.request_id);
    assert!(parsed.certificate_proof_b64.is_some());
}

#[test]
fn invalid_public_certificate_is_rejected() {
    assert!(matches!(
        PeerPolicy::new(KEY, vec![]).with_pinned_certificate(b"not a certificate", &"a".repeat(64)),
        Err(RpcError::Malformed)
    ));
}

#[test]
fn omas_tenant_policy_rejects_invalid_tenant_identifiers() {
    for invalid in ["", "tenant/a", "tenant a", "tenant.a", "tenant:a"] {
        assert!(
            matches!(
                PeerPolicy::new(KEY, vec![]).with_omas_tenant(invalid),
                Err(RpcError::Malformed)
            ),
            "unexpectedly accepted tenant: {invalid:?}"
        );
    }
    assert!(matches!(
        PeerPolicy::new(KEY, vec![]).with_omas_tenant(&"a".repeat(129)),
        Err(RpcError::Malformed)
    ));
}

#[test]
fn omas_tenant_policy_accepts_explicit_scoped_identity() {
    let (pem, _, pin) = credential();
    let peer = PeerPolicy::new(KEY, vec![])
        .with_pinned_certificate(&pem, &pin)
        .unwrap()
        .with_omas_tenant("tenant_a-1")
        .unwrap();
    assert!(RpcGateway::new(
        "node-b",
        OcafRuntime::new(0),
        HashMap::from([("node-a".into(), peer)]),
        4,
    )
    .is_ok());
}

#[tokio::test]
async fn signed_omas_request_requires_matching_authenticated_peer_tenant() {
    use ocaf_runtime::omas_signed::SignedOperation;
    use ocaf_runtime::runtime::ToolCallRequest;
    use serde_json::json;

    let (pem, key, pin) = credential();
    let call = ToolCallRequest {
        call_id: "tenant-bound-call".into(),
        tool: "demo_echo".into(),
        args: json!({}),
        auth: None,
    };
    let mut request = RpcRequest::new(
        "node-a",
        "node-b",
        NOW,
        5_000,
        RpcAction::Execute {
            tool: "demo_echo".into(),
            args: json!({}),
        },
    )
    .unwrap();
    request.omas_operation = Some(SignedOperation {
        tenant: "tenant_b".into(),
        namespace: "tenant_b/tools".into(),
        target: "tenant_b/tools/demo_echo".into(),
        opcode: 1,
        fuel: 100,
        issued_at: 1_800_000_000,
        nonce: "tenant_bound_nonce_001".into(),
        payload: serde_json::to_vec(&call).unwrap(),
        signature: vec![0; 64],
    });
    let signed = request.with_certificate_proof(&key).unwrap();

    // A matching tenant ACL alone must not authorize an OMAS request
    // without a pinned certificate and proof of private-key possession.
    let tenant_only = PeerPolicy::new(KEY, vec!["demo_echo".into()])
        .with_omas_tenant("tenant_b")
        .unwrap();
    let no_certificate_gateway = RpcGateway::new(
        "node-b",
        OcafRuntime::new(0),
        HashMap::from([("node-a".into(), tenant_only)]),
        4,
    )
    .unwrap();
    assert!(matches!(
        send(&no_certificate_gateway, &signed).await,
        Err(RpcError::Unauthorized)
    ));

    for configured_tenant in [None, Some("tenant_a")] {
        let mut policy = PeerPolicy::new(KEY, vec!["demo_echo".into()])
            .with_pinned_certificate(&pem, &pin)
            .unwrap();
        if let Some(tenant) = configured_tenant {
            policy = policy.with_omas_tenant(tenant).unwrap();
        }
        let gateway = RpcGateway::new(
            "node-b",
            OcafRuntime::new(0),
            HashMap::from([("node-a".into(), policy)]),
            4,
        )
        .unwrap();
        assert!(
            matches!(send(&gateway, &signed).await, Err(RpcError::Unauthorized)),
            "OMAS request must fail closed for peer tenant {configured_tenant:?}"
        );
    }
}
