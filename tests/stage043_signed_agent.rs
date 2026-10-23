//! Stage 0.43 — cryptographic agent admission, sender binding and persistent replay.
use ocaf_runtime::omas_server::{AgentRegistrationRequest, OmasServer, OmasServerConfig};
use ocaf_runtime::omas_signed::SignedOperation;
use ocaf_runtime::stage043_signed_agent::{
    admit_signed_registration, admit_signed_registration_for_sender, SignedAgentError,
};
use openssl::pkey::PKey;
use openssl::sign::Signer;
use rusqlite::Connection;

const NOW: i64 = 1_800_000_000;

fn request() -> AgentRegistrationRequest {
    AgentRegistrationRequest {
        request_id: "staged-01".into(),
        agent_id: "OCAF-AGENT-001".into(),
        tenant: "tenant_a".into(),
        manifest_digest: format!("sha256:{}", "a".repeat(64)),
        declared_state: "STAGED".into(),
        readiness: false,
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}

fn sign(request: &AgentRegistrationRequest, key: &PKey<openssl::pkey::Private>) -> SignedOperation {
    let mut op = SignedOperation {
        tenant: request.tenant.clone(),
        namespace: format!("{}/agents", request.tenant),
        target: format!("{}/agents/{}", request.tenant, request.agent_id),
        opcode: 5,
        fuel: 1,
        issued_at: NOW,
        nonce: "stage043_unique_nonce_001".into(),
        payload: serde_json::to_vec(request).unwrap(),
        rpc_sender: Some("authenticated-agent-node".into()),
        signature: Vec::new(),
    };
    op.signature = Signer::new_without_digest(key)
        .unwrap()
        .sign_oneshot_to_vec(&op.canonical_bytes())
        .unwrap();
    op
}

#[test]
fn signed_registration_stages_agent_once_with_durable_replay_protection() {
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("replay.sqlite");
    Connection::open(&db).unwrap().close().unwrap();
    let server = OmasServer::new(OmasServerConfig::default()).unwrap();
    let req = request();
    let signed = sign(&req, &key);
    let admitted =
        admit_signed_registration(&server, req.clone(), &signed, &public, &db, NOW).unwrap();
    assert!(admitted.admitted);
    let duplicate = admit_signed_registration(&server, req, &signed, &public, &db, NOW);
    assert!(matches!(duplicate, Err(SignedAgentError::Replay)));
}

#[test]
fn forged_registration_identity_is_denied_before_replay_mutation() {
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("replay.sqlite");
    Connection::open(&db).unwrap().close().unwrap();
    let server = OmasServer::new(OmasServerConfig::default()).unwrap();
    let valid = request();
    let signed = sign(&valid, &key);
    let mut forged = valid.clone();
    forged.agent_id = "OCAF-AGENT-002".into();
    assert!(matches!(
        admit_signed_registration(&server, forged, &signed, &public, &db, NOW),
        Err(SignedAgentError::Binding)
    ));
    assert!(
        admit_signed_registration(&server, valid, &signed, &public, &db, NOW)
            .unwrap()
            .admitted
    );
}

#[test]
fn unsigned_or_downgraded_rpc_sender_is_rejected() {
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("replay.sqlite");
    Connection::open(&db).unwrap().close().unwrap();
    let server = OmasServer::new(OmasServerConfig::default()).unwrap();
    let req = request();
    let mut signed = sign(&req, &key);
    signed.rpc_sender = None;
    assert!(matches!(
        admit_signed_registration(&server, req.clone(), &signed, &public, &db, NOW),
        Err(SignedAgentError::Binding)
    ));
    signed.rpc_sender = Some("impersonated-node".into());
    assert!(matches!(
        admit_signed_registration(&server, req, &signed, &public, &db, NOW),
        Err(SignedAgentError::Signature)
    ));
}

#[test]
fn authenticated_transport_sender_must_match_signed_sender() {
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("replay.sqlite");
    Connection::open(&db).unwrap().close().unwrap();
    let server = OmasServer::new(OmasServerConfig::default()).unwrap();
    let req = request();
    let signed = sign(&req, &key);
    assert!(matches!(
        admit_signed_registration_for_sender(
            &server, req.clone(), &signed, "attacker-node", &public, &db, NOW
        ),
        Err(SignedAgentError::Binding)
    ));
    assert!(admit_signed_registration_for_sender(
        &server, req, &signed, "authenticated-agent-node", &public, &db, NOW
    ).unwrap().admitted);
}
