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
