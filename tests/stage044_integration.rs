//! Stage 0.44 contract/integration tests. No mock OAuth provider is used.
//! Tests exercise the real SQLite transaction and the OMAt policy boundary.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ocaf_runtime::omat_server::{
    ExecutorKind, OmatPolicy, OmatRegistry, ToolCall, ToolManifest, ToolRisk,
};
use ocaf_runtime::stage043_omat_route::{AuthenticatedPeer, OmatRouteGuard};
use ocaf_runtime::stage044_oauth::{
    AuthenticatedSubject, AuthorizationRequest, CodeExchange, OAuthClient, OAuthService,
};
use ocaf_runtime::stage044_omat_authz::{
    BoundAgent, OAuthOmatGate, OmatDispatchContext, ToolAuthorization,
};
use rusqlite::Connection;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
fn scopes() -> BTreeSet<String> {
    ["tool:read".to_string()].into_iter().collect()
}
fn client(tenant: &str) -> OAuthClient {
    OAuthClient {
        client_id: format!("client-{tenant}"),
        tenant: tenant.into(),
        redirect_uri: "https://client.example/callback".into(),
        resource: "https://omat.example".into(),
        scopes: scopes(),
        step_up_scopes: BTreeSet::new(),
    }
}
fn token(s: &OAuthService, tenant: &str, user: &str) -> String {
    let verifier = "v".repeat(64);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let subject = AuthenticatedSubject {
        tenant: tenant.into(),
        user: user.into(),
        authenticated_at: now(),
        assurance_level: 1,
    };
    let request = AuthorizationRequest {
        response_type: "code".into(),
        client_id: format!("client-{tenant}"),
        redirect_uri: "https://client.example/callback".into(),
        resource: "https://omat.example".into(),
        scope: scopes(),
        state: "q".repeat(32),
        code_challenge: challenge,
        code_challenge_method: "S256".into(),
    };
    let path = OAuthService::user_endpoint(tenant, user).unwrap();
    let code = s.authorize(&path, &subject, true, &request).unwrap().code;
    s.exchange(&CodeExchange {
        client_id: format!("client-{tenant}"),
        code,
        code_verifier: verifier,
        resource: "https://omat.example".into(),
    })
    .unwrap()
    .access_token
}
fn gate() -> OAuthOmatGate {
    OAuthOmatGate::new(
        "https://omat.example".into(),
        vec![
            BoundAgent {
                tenant: "tenant-a".into(),
                agent_id: "agent-a".into(),
                user: "alice".into(),
            },
            BoundAgent {
                tenant: "tenant-b".into(),
                agent_id: "agent-b".into(),
                user: "bob".into(),
            },
        ],
        vec![ToolAuthorization {
            tool_id: "echo".into(),
            scopes: scopes(),
            min_assurance: 1,
        }],
    )
    .unwrap()
}
fn call(tenant: &str, agent: &str) -> ToolCall {
    ToolCall {
        request_id: "req-1".into(),
        tenant_id: tenant.into(),
        agent_id: agent.into(),
        tool_id: "echo".into(),
        arguments: json!({}),
        idempotency_key: "idem-1".into(),
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}
#[test]
fn two_tenants_two_users_and_confused_deputy_are_isolated() {
    let dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.path().join("oauth.sqlite");
    Connection::open(&path).unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let oauth = OAuthService::open(&path, vec![client("tenant-a"), client("tenant-b")]).unwrap();
    let alice = token(&oauth, "tenant-a", "alice");
    let bob = token(&oauth, "tenant-b", "bob");
    let guard = gate();
    let peer_a = AuthenticatedPeer {
        principal: "agent-a".into(),
        tenant: "tenant-a".into(),
    };
    let peer_b = AuthenticatedPeer {
        principal: "agent-b".into(),
        tenant: "tenant-b".into(),
    };
    assert!(guard
        .authorize(&oauth, &alice, &peer_a, &call("tenant-a", "agent-a"))
        .is_ok());
    assert!(guard
        .authorize(&oauth, &bob, &peer_b, &call("tenant-b", "agent-b"))
        .is_ok());
    assert!(guard
        .authorize(&oauth, &alice, &peer_b, &call("tenant-b", "agent-b"))
        .is_err());
    assert!(guard
        .authorize(&oauth, &alice, &peer_a, &call("tenant-b", "agent-a"))
        .is_err());
    assert!(guard
        .authorize(&oauth, &alice, &peer_a, &call("tenant-a", "agent-b"))
        .is_err());
    oauth.revoke_user("tenant-a", "alice").unwrap();
    assert!(guard
        .authorize(&oauth, &alice, &peer_a, &call("tenant-a", "agent-a"))
        .is_err());
    assert!(guard
        .authorize(&oauth, &bob, &peer_b, &call("tenant-b", "agent-b"))
        .is_ok());
}
#[test]
fn valid_oauth_never_bypasses_omat_execution_and_ostc() {
    let dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.path().join("oauth.sqlite");
    Connection::open(&path).unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let oauth = OAuthService::open(&path, vec![client("tenant-a")]).unwrap();
    let alice = token(&oauth, "tenant-a", "alice");
    let peer = AuthenticatedPeer {
        principal: "agent-a".into(),
        tenant: "tenant-a".into(),
    };
    let mut peers = HashMap::new();
    peers.insert("agent-a".into(), "tenant-a".into());
    let mut route = OmatRouteGuard::new(peers).unwrap();
    route.set_transport_connected(true);
    let policy = OmatPolicy {
        tenant_id: "tenant-a".into(),
        allowed_tools: ["echo".into()].into_iter().collect(),
        allowed_capabilities: scopes(),
        max_timeout_ms: 1000,
        max_output_bytes: 4096,
        allow_external_side_effects: false,
    };
    let mut registry = OmatRegistry::default();
    registry
        .register(ToolManifest {
            tenant_id: "tenant-a".into(),
            tool_id: "echo".into(),
            version: "1".into(),
            description: "bounded echo".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            capabilities: scopes(),
            risk: ToolRisk::ReadOnly,
            executor: ExecutorKind::Wasi,
            timeout_ms: 500,
            max_output_bytes: 1024,
            enabled: true,
        })
        .unwrap();
    let result = gate().dispatch(
        &oauth,
        &alice,
        &peer,
        "ocaf/omat/tenant-a/requests/req-1",
        &call("tenant-a", "agent-a"),
        OmatDispatchContext {
            route: &route,
            policy: &policy,
            registry: &registry,
        },
    );
    assert!(
        result.is_err(),
        "OAuth cannot grant OSTc source access or a missing sandbox executor"
    );
}
