use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ocaf_runtime::builder_cloud::{BuilderCloudError, BuilderCloudService};
use ocaf_runtime::builder_core::{from_dsl, BuilderProject};
use ocaf_runtime::builder_store::{BuilderStoreError, SqliteBuilderStore};
use ocaf_runtime::stage044_oauth::{
    AuthenticatedSubject, AuthorizationRequest, CodeExchange, OAuthClient, OAuthError, OAuthService,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

const RESOURCE: &str = "https://builder.ocaf.example";

fn scope(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn fixture() -> (TempDir, OAuthService, SqliteBuilderStore) {
    let dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let oauth_path = dir.path().join("oauth.sqlite");
    let builder_path = dir.path().join("builder.sqlite");
    File::create(&oauth_path).unwrap();
    File::create(&builder_path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&oauth_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::set_permissions(&builder_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let clients = ["local", "another"]
        .iter()
        .map(|tenant| OAuthClient {
            client_id: format!("builder-{tenant}"),
            tenant: (*tenant).into(),
            redirect_uri: "https://client.ocaf.example/callback".into(),
            resource: RESOURCE.into(),
            scopes: scope(&["builder:read", "builder:write"]),
            step_up_scopes: BTreeSet::new(),
        })
        .collect();
    let oauth = OAuthService::open(&oauth_path, clients).unwrap();
    let store = SqliteBuilderStore::open_existing(&builder_path).unwrap();
    (dir, oauth, store)
}

fn subject(tenant: &str) -> AuthenticatedSubject {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    AuthenticatedSubject {
        tenant: tenant.into(),
        user: "alice".into(),
        authenticated_at: now,
        assurance_level: 2,
    }
}

fn token(oauth: &OAuthService, tenant: &str, scopes: &[&str]) -> String {
    let verifier = "a".repeat(64);
    let request = AuthorizationRequest {
        response_type: "code".into(),
        client_id: format!("builder-{tenant}"),
        redirect_uri: "https://client.ocaf.example/callback".into(),
        resource: RESOURCE.into(),
        scope: scope(scopes),
        state: "s".repeat(32),
        code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
        code_challenge_method: "S256".into(),
    };
    let response = oauth
        .authorize(
            &OAuthService::user_endpoint(tenant, "alice").unwrap(),
            &subject(tenant),
            true,
            &request,
        )
        .unwrap();
    oauth
        .exchange(&CodeExchange {
            client_id: format!("builder-{tenant}"),
            code: response.code,
            code_verifier: verifier,
            resource: RESOURCE.into(),
        })
        .unwrap()
        .access_token
}

fn project() -> BuilderProject {
    from_dsl(
        include_str!("../apps/agents-builder-web/fixtures/hello.json"),
        "local",
    )
    .unwrap()
}

#[test]
fn scoped_cloud_admission_and_durable_cas() {
    let (_dir, oauth, store) = fixture();
    let api = BuilderCloudService::new(&oauth, &store, RESOURCE).unwrap();
    let auth = subject("local");
    let credential = token(&oauth, "local", &["builder:read", "builder:write"]);
    let original = project();
    let plan = api.preview(&credential, &auth, &original).unwrap();
    assert_eq!(plan.ordered_nodes, vec!["input", "output"]);
    assert!(store.load("local", "hello", None).unwrap().is_none());
    let first = api.save(&credential, &auth, &original, None).unwrap();
    let (loaded, loaded_head) = api
        .load(&credential, &auth, "hello", Some(&first))
        .unwrap()
        .unwrap();
    assert_eq!(loaded, original);
    assert_eq!(loaded_head, first);
    let mut next = original.clone();
    next.revision = 2;
    let second = api.save(&credential, &auth, &next, Some(&first)).unwrap();
    assert_eq!(second.revision, 2);
    assert!(matches!(
        api.save(&credential, &auth, &next, Some(&first)),
        Err(BuilderCloudError::Store(BuilderStoreError::Conflict))
    ));
    assert_eq!(
        api.history(&credential, &auth, "hello").unwrap(),
        vec![first, second]
    );
}

#[test]
fn read_only_tokens_cannot_write_and_write_only_cannot_read() {
    let (_dir, oauth, store) = fixture();
    let api = BuilderCloudService::new(&oauth, &store, RESOURCE).unwrap();
    let auth = subject("local");
    let read = token(&oauth, "local", &["builder:read"]);
    let write = token(&oauth, "local", &["builder:write"]);
    assert!(matches!(
        api.save(&read, &auth, &project(), None),
        Err(BuilderCloudError::OAuth(OAuthError::AccessDenied))
    ));
    api.save(&write, &auth, &project(), None).unwrap();
    assert!(matches!(
        api.load(&write, &auth, "hello", None),
        Err(BuilderCloudError::OAuth(OAuthError::AccessDenied))
    ));
    assert!(api.load(&read, &auth, "hello", None).unwrap().is_some());
}

#[test]
fn cross_tenant_subject_and_payload_spoofing_fail_closed() {
    let (_dir, oauth, store) = fixture();
    let api = BuilderCloudService::new(&oauth, &store, RESOURCE).unwrap();
    let credential = token(&oauth, "local", &["builder:read", "builder:write"]);
    assert!(matches!(
        api.save(&credential, &subject("another"), &project(), None),
        Err(BuilderCloudError::OAuth(OAuthError::AccessDenied))
    ));
    let mut forged = project();
    forged.tenant_id = "another".into();
    assert!(matches!(
        api.save(&credential, &subject("local"), &forged, None),
        Err(BuilderCloudError::Store(BuilderStoreError::Denied))
    ));
    assert!(store.load("local", "hello", None).unwrap().is_none());
    assert!(store.load("another", "hello", None).unwrap().is_none());
}

#[test]
fn revoked_tokens_and_wrong_audience_are_rejected() {
    let (_dir, oauth, store) = fixture();
    let api = BuilderCloudService::new(&oauth, &store, RESOURCE).unwrap();
    let wrong = BuilderCloudService::new(&oauth, &store, "https://other.example").unwrap();
    let credential = token(&oauth, "local", &["builder:read"]);
    assert!(matches!(
        wrong.preview(&credential, &subject("local"), &project()),
        Err(BuilderCloudError::OAuth(OAuthError::AccessDenied))
    ));
    oauth.revoke_user("local", "alice").unwrap();
    assert!(matches!(
        api.preview(&credential, &subject("local"), &project()),
        Err(BuilderCloudError::OAuth(OAuthError::AccessDenied))
    ));
    assert!(BuilderCloudService::new(&oauth, &store, "http://insecure").is_err());
}

const HTTP_ORIGIN: &str = "https://editor.ocaf.example";

fn http_headers(credential: &str) -> [(&str, &str); 3] {
    [
        ("Origin", HTTP_ORIGIN),
        ("Authorization", credential),
        ("Content-Type", "application/json"),
    ]
}

#[test]
fn http_boundary_browser_preview_save_and_read_use_trusted_subject() {
    use ocaf_runtime::builder_http::{BuilderHttpBoundary, BuilderHttpRequest};
    let (_dir, oauth, store) = fixture();
    let gateway = BuilderHttpBoundary::new(&oauth, &store, RESOURCE, HTTP_ORIGIN).unwrap();
    let auth = subject("local");
    let credential = format!(
        "Bearer {}",
        token(&oauth, "local", &["builder:read", "builder:write"])
    );
    let headers = http_headers(&credential);
    let body = serde_json::to_vec(&project()).unwrap();
    let preview = gateway.handle(
        BuilderHttpRequest {
            method: "POST",
            path: "/v1/projects/preview",
            headers: &headers,
            body: &body,
        },
        &auth,
    );
    assert_eq!(preview.status, 200);
    let preview_json: serde_json::Value = serde_json::from_slice(&preview.body).unwrap();
    assert_eq!(
        preview_json["ordered_nodes"],
        serde_json::json!(["input", "output"])
    );
    assert!(store.load("local", "hello", None).unwrap().is_none());

    let save_body = serde_json::to_vec(&serde_json::json!({
        "project": project(), "expected": null
    }))
    .unwrap();
    let saved = gateway.handle(
        BuilderHttpRequest {
            method: "PUT",
            path: "/v1/projects/hello",
            headers: &headers,
            body: &save_body,
        },
        &auth,
    );
    assert_eq!(saved.status, 200);
    let saved_json: serde_json::Value = serde_json::from_slice(&saved.body).unwrap();
    assert_eq!(saved_json["revision"], 1);
    assert_eq!(saved_json["sha256"].as_str().unwrap().len(), 64);
    let duplicate = gateway.handle(
        BuilderHttpRequest {
            method: "PUT",
            path: "/v1/projects/hello",
            headers: &headers,
            body: &save_body,
        },
        &auth,
    );
    assert_eq!(duplicate.status, 409);
    let loaded = gateway.handle(
        BuilderHttpRequest {
            method: "GET",
            path: "/v1/projects/hello",
            headers: &headers,
            body: &[],
        },
        &auth,
    );
    assert_eq!(loaded.status, 200);
    let loaded_json: serde_json::Value = serde_json::from_slice(&loaded.body).unwrap();
    assert_eq!(loaded_json["project"]["project_id"], "hello");
}

#[test]
fn http_boundary_rejects_cross_origin_cookies_duplicate_auth_and_spoofing() {
    use ocaf_runtime::builder_http::{BuilderHttpBoundary, BuilderHttpRequest};
    let (_dir, oauth, store) = fixture();
    let gateway = BuilderHttpBoundary::new(&oauth, &store, RESOURCE, HTTP_ORIGIN).unwrap();
    let credential = format!(
        "Bearer {}",
        token(&oauth, "local", &["builder:read", "builder:write"])
    );
    let auth = subject("local");
    let payload = serde_json::to_vec(&project()).unwrap();
    for headers in [
        vec![
            ("Origin", "https://attacker.example"),
            ("Authorization", &credential),
            ("Content-Type", "application/json"),
        ],
        vec![
            ("Authorization", &credential),
            ("Content-Type", "application/json"),
        ],
        vec![
            ("Origin", HTTP_ORIGIN),
            ("Authorization", &credential),
            ("Cookie", "session=evil"),
            ("Content-Type", "application/json"),
        ],
    ] {
        let result = gateway.handle(
            BuilderHttpRequest {
                method: "POST",
                path: "/v1/projects/preview",
                headers: &headers,
                body: &payload,
            },
            &auth,
        );
        assert_eq!(result.status, 403);
    }
    let duplicate = gateway.handle(
        BuilderHttpRequest {
            method: "POST",
            path: "/v1/projects/preview",
            headers: &[
                ("Origin", HTTP_ORIGIN),
                ("Authorization", &credential),
                ("authorization", &credential),
                ("Content-Type", "application/json"),
            ],
            body: &payload,
        },
        &auth,
    );
    assert_eq!(duplicate.status, 400);
    let forged = gateway.handle(
        BuilderHttpRequest {
            method: "POST",
            path: "/v1/projects/preview",
            headers: &http_headers(&credential),
            body: &payload,
        },
        &subject("another"),
    );
    assert_eq!(forged.status, 403);
    assert!(store.load("local", "hello", None).unwrap().is_none());
}

#[test]
fn http_boundary_rejects_oversize_unknown_paths_invalid_methods_and_payloads() {
    use ocaf_runtime::builder_http::{BuilderHttpBoundary, BuilderHttpRequest};
    let (_dir, oauth, store) = fixture();
    let gateway = BuilderHttpBoundary::new(&oauth, &store, RESOURCE, HTTP_ORIGIN).unwrap();
    let credential = format!(
        "Bearer {}",
        token(&oauth, "local", &["builder:read", "builder:write"])
    );
    let headers = http_headers(&credential);
    let auth = subject("local");
    let oversized = vec![b'x'; 128 * 1024 + 1];
    let result = gateway.handle(
        BuilderHttpRequest {
            method: "POST",
            path: "/v1/projects/preview",
            headers: &headers,
            body: &oversized,
        },
        &auth,
    );
    assert_eq!(result.status, 413);
    for (method, path, expected) in [
        ("POST", "/v1/projects/../preview", 404),
        ("DELETE", "/v1/projects/hello", 405),
        ("GET", "/v1/projects/hello?tenant=another", 404),
        ("PUT", "/v1/projects/preview", 405),
    ] {
        let result = gateway.handle(
            BuilderHttpRequest {
                method,
                path,
                headers: &headers,
                body: &[],
            },
            &auth,
        );
        assert_eq!(result.status, expected);
    }
    let mismatch = serde_json::to_vec(&serde_json::json!({
        "project": project(), "expected": null, "unexpected": true
    }))
    .unwrap();
    let result = gateway.handle(
        BuilderHttpRequest {
            method: "PUT",
            path: "/v1/projects/hello",
            headers: &headers,
            body: &mismatch,
        },
        &auth,
    );
    assert_eq!(result.status, 400);
    assert!(store.load("local", "hello", None).unwrap().is_none());
}

#[test]
fn http_boundary_rejects_duplicate_content_type_as_malformed_request() {
    use ocaf_runtime::builder_http::{BuilderHttpBoundary, BuilderHttpRequest};
    let (_dir, oauth, store) = fixture();
    let gateway = BuilderHttpBoundary::new(&oauth, &store, RESOURCE, HTTP_ORIGIN).unwrap();
    let credential = format!(
        "Bearer {}",
        token(&oauth, "local", &["builder:read", "builder:write"])
    );
    let payload = serde_json::to_vec(&project()).unwrap();
    let response = gateway.handle(
        BuilderHttpRequest {
            method: "POST",
            path: "/v1/projects/preview",
            headers: &[
                ("Origin", HTTP_ORIGIN),
                ("Authorization", &credential),
                ("Content-Type", "application/json"),
                ("content-type", "application/json"),
            ],
            body: &payload,
        },
        &subject("local"),
    );
    assert_eq!(response.status, 400);
    assert!(store.load("local", "hello", None).unwrap().is_none());
}

#[test]
fn http_boundary_browser_get_without_origin_requires_bearer_and_trusted_subject() {
    use ocaf_runtime::builder_http::{BuilderHttpBoundary, BuilderHttpRequest};
    let (_dir, oauth, store) = fixture();
    let gateway = BuilderHttpBoundary::new(&oauth, &store, RESOURCE, HTTP_ORIGIN).unwrap();
    let auth = subject("local");
    let credential = format!(
        "Bearer {}",
        token(&oauth, "local", &["builder:read", "builder:write"])
    );
    let body = serde_json::to_vec(&serde_json::json!({
        "project": project(), "expected": null
    }))
    .unwrap();
    let saved = gateway.handle(
        BuilderHttpRequest {
            method: "PUT",
            path: "/v1/projects/hello",
            headers: &http_headers(&credential),
            body: &body,
        },
        &auth,
    );
    assert_eq!(saved.status, 200);

    let no_origin = [("Authorization", credential.as_str())];
    let loaded = gateway.handle(
        BuilderHttpRequest {
            method: "GET",
            path: "/v1/projects/hello",
            headers: &no_origin,
            body: &[],
        },
        &auth,
    );
    assert_eq!(loaded.status, 200);

    let forged = gateway.handle(
        BuilderHttpRequest {
            method: "GET",
            path: "/v1/projects/hello",
            headers: &no_origin,
            body: &[],
        },
        &subject("another"),
    );
    assert_eq!(forged.status, 403);

    let cross_origin = [
        ("Origin", "https://attacker.example"),
        ("Authorization", credential.as_str()),
    ];
    let rejected = gateway.handle(
        BuilderHttpRequest {
            method: "GET",
            path: "/v1/projects/hello",
            headers: &cross_origin,
            body: &[],
        },
        &auth,
    );
    assert_eq!(rejected.status, 403);

    let missing_origin_write = [
        ("Authorization", credential.as_str()),
        ("Content-Type", "application/json"),
    ];
    let rejected_write = gateway.handle(
        BuilderHttpRequest {
            method: "PUT",
            path: "/v1/projects/hello",
            headers: &missing_origin_write,
            body: &body,
        },
        &auth,
    );
    assert_eq!(rejected_write.status, 403);
}

#[test]
fn http_boundary_rejects_ambiguous_http_framing_before_project_write() {
    use ocaf_runtime::builder_http::{BuilderHttpBoundary, BuilderHttpRequest};
    let (_dir, oauth, store) = fixture();
    let gateway = BuilderHttpBoundary::new(&oauth, &store, RESOURCE, HTTP_ORIGIN).unwrap();
    let credential = format!(
        "Bearer {}",
        token(&oauth, "local", &["builder:read", "builder:write"])
    );
    let auth = subject("local");
    let body = serde_json::to_vec(&serde_json::json!({
        "project": project(), "expected": null
    }))
    .unwrap();
    for extra in [
        vec![("Transfer-Encoding", "chunked")],
        vec![("Content-Length", "0"), ("content-length", "0")],
        vec![("Content-Length", "invalid")],
        vec![("Content-Length", "1")],
        vec![("Content-Length", "0"), ("Transfer-Encoding", "chunked")],
    ] {
        let mut headers = http_headers(&credential).to_vec();
        headers.extend(extra);
        let response = gateway.handle(
            BuilderHttpRequest {
                method: "PUT",
                path: "/v1/projects/hello",
                headers: &headers,
                body: &body,
            },
            &auth,
        );
        assert_eq!(response.status, 400);
        assert!(store.load("local", "hello", None).unwrap().is_none());
    }
}

#[test]
fn http_boundary_rejects_malformed_configured_https_origins() {
    use ocaf_runtime::builder_http::BuilderHttpBoundary;
    let (_dir, oauth, store) = fixture();
    for origin in [
        "https://:443",
        "https://example.com:",
        "https://example.com:abc",
        "https://example.com:0",
        "https://example.com:65536",
        "https://example..com",
        "https://-example.com",
        "https://example.com.",
        "https://[::1",
        "https://example.com:443:444",
    ] {
        assert!(
            BuilderHttpBoundary::new(&oauth, &store, RESOURCE, origin).is_err(),
            "invalid configured origin accepted: {origin}"
        );
    }
    for origin in [
        "https://example.com",
        "https://example.com:443",
        "https://sub.example.com:8443",
        "https://localhost:3000",
        "https://127.0.0.1:443",
        "https://[::1]:443",
    ] {
        assert!(
            BuilderHttpBoundary::new(&oauth, &store, RESOURCE, origin).is_ok(),
            "valid configured origin rejected: {origin}"
        );
    }
}
