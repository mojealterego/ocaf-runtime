//! Stage 0.44: durable, fail-closed OAuth authorization-code/PKCE core.
//!
//! The authenticated subject and explicit consent MUST come from a separately
//! authenticated, CSRF-protected operator/identity gateway, never from an
//! untrusted OAuth query string or an LLM. This is not an HTTP listener.
//! Opaque tokens are bound to one tenant, one user, one client and one resource.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use openssl::{memcmp, rand::rand_bytes};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

const CODE_LIFETIME: i64 = 90;
const ACCESS_LIFETIME: i64 = 300;
const REFRESH_LIFETIME: i64 = 86_400;
const MAX_CLIENTS: usize = 4096;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OAuthError {
    #[error("invalid client configuration")]
    Configuration,
    #[error("authorization request rejected")]
    InvalidRequest,
    #[error("unauthorized scope or resource")]
    AccessDenied,
    #[error("recent step-up authentication required")]
    StepUpRequired,
    #[error("authorization grant invalid, expired or already consumed")]
    InvalidGrant,
    #[error("refresh token replay detected; token family revoked")]
    ReplayDetected,
    #[error("authorization database unavailable")]
    Storage,
    #[error("cryptographic randomness unavailable")]
    Entropy,
}

#[derive(Clone, Debug)]
pub struct OAuthClient {
    pub client_id: String,
    pub tenant: String,
    pub redirect_uri: String,
    pub resource: String,
    pub scopes: BTreeSet<String>,
    pub step_up_scopes: BTreeSet<String>,
}

impl OAuthClient {
    fn validate(&self) -> Result<(), OAuthError> {
        if !identifier(&self.client_id)
            || !identifier(&self.tenant)
            || !https_url(&self.redirect_uri)
            || !https_url(&self.resource)
            || self.scopes.is_empty()
            || self.scopes.len() > 128
            || self.scopes.iter().any(|s| !scope_name(s))
            || !self.step_up_scopes.is_subset(&self.scopes)
        {
            return Err(OAuthError::Configuration);
        }
        Ok(())
    }
}

/// Construct only from an authenticated gateway session. Neither tenant nor
/// user may be accepted from request body, path, JWT without verification, or
/// model-produced tool arguments.
#[derive(Clone, Debug)]
pub struct AuthenticatedSubject {
    pub tenant: String,
    pub user: String,
    pub authenticated_at: i64,
    pub assurance_level: u8,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub resource: String,
    pub scope: BTreeSet<String>,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

/// Contains a one-time code: do not log, serialize into telemetry or put in an
/// LLM context. The caller redirects only to the pre-registered exact URI.
pub struct AuthorizationResponse {
    pub redirect_uri: String,
    pub code: String,
    pub state: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeExchange {
    pub client_id: String,
    pub code: String,
    pub code_verifier: String,
    pub resource: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshRequest {
    pub client_id: String,
    pub refresh_token: String,
    pub resource: String,
}

#[derive(Serialize, PartialEq, Eq)]
pub struct TokenResponse {
    pub token_type: &'static str,
    pub access_token: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub refresh_token: String,
    pub expires_in: i64,
    pub scope: String,
}

impl std::fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenResponse")
            .field("token_type", &self.token_type)
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// A server-validated, immutable authorization context; must be passed into
/// the OMAt policy guard, not replaced by caller-provided tenant fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedGrant {
    tenant: String,
    user: String,
    client_id: String,
    resource: String,
    scopes: BTreeSet<String>,
    assurance_level: u8,
}
impl AuthorizedGrant {
    pub fn tenant(&self) -> &str { &self.tenant }
    pub fn user(&self) -> &str { &self.user }
    pub fn resource(&self) -> &str { &self.resource }
    pub fn client_id(&self) -> &str { &self.client_id }
    pub fn scopes(&self) -> &BTreeSet<String> { &self.scopes }
    pub fn assurance_level(&self) -> u8 { self.assurance_level }
}


trait Clock: Send + Sync {
    fn now(&self) -> Result<i64, OAuthError>;
}

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Result<i64, OAuthError> {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| OAuthError::Storage)?;
        i64::try_from(time.as_secs()).map_err(|_| OAuthError::Storage)
    }
}

pub struct OAuthService {
    db_path: PathBuf,
    clients: HashMap<String, OAuthClient>,
    clock: Arc<dyn Clock>,
}

impl OAuthService {
    /// Requires an already-provisioned regular SQLite file in a protected
    /// operator-controlled directory; refuses a missing or symlinked file.
    pub fn open(db_path: &Path, clients: Vec<OAuthClient>) -> Result<Self, OAuthError> {
        Self::open_inner(db_path, clients, Arc::new(SystemClock))
    }

    fn open_inner(
        db_path: &Path,
        clients: Vec<OAuthClient>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, OAuthError> {
        if !private_database(db_path) || clients.is_empty() || clients.len() > MAX_CLIENTS {
            return Err(OAuthError::Configuration);
        }
        let mut registry = HashMap::new();
        for client in clients {
            client.validate()?;
            if registry.insert(client.client_id.clone(), client).is_some() {
                return Err(OAuthError::Configuration);
            }
        }
        let service = Self { db_path: db_path.to_owned(), clients: registry, clock };
        let conn = service.connection()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS oauth_codes (
                digest TEXT PRIMARY KEY, client TEXT NOT NULL, tenant TEXT NOT NULL,
                user_id TEXT NOT NULL, redirect_uri TEXT NOT NULL, resource TEXT NOT NULL,
                scopes TEXT NOT NULL, challenge TEXT NOT NULL, assurance INTEGER NOT NULL,
                expires_at INTEGER NOT NULL, consumed INTEGER NOT NULL DEFAULT 0
                CHECK(consumed IN (0,1))
            );
            CREATE TABLE IF NOT EXISTS oauth_families (
                family TEXT PRIMARY KEY, tenant TEXT NOT NULL, user_id TEXT NOT NULL,
                client TEXT NOT NULL, revoked INTEGER NOT NULL DEFAULT 0
                CHECK(revoked IN (0,1))
            );
            CREATE TABLE IF NOT EXISTS oauth_tokens (
                digest TEXT PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('access','refresh')),
                family TEXT NOT NULL REFERENCES oauth_families(family),
                client TEXT NOT NULL, tenant TEXT NOT NULL, user_id TEXT NOT NULL,
                resource TEXT NOT NULL, scopes TEXT NOT NULL, assurance INTEGER NOT NULL,
                expires_at INTEGER NOT NULL, consumed INTEGER NOT NULL DEFAULT 0
                CHECK(consumed IN (0,1))
            );
            CREATE INDEX IF NOT EXISTS oauth_token_family ON oauth_tokens(family);
            CREATE INDEX IF NOT EXISTS oauth_code_expiry ON oauth_codes(expires_at);
            CREATE INDEX IF NOT EXISTS oauth_token_expiry ON oauth_tokens(expires_at);"
        ).map_err(|_| OAuthError::Storage)?;
        Ok(service)
    }

    fn connection(&self) -> Result<Connection, OAuthError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let conn = Connection::open_with_flags(&self.db_path, flags).map_err(|_| OAuthError::Storage)?;
        conn.busy_timeout(Duration::from_secs(5)).map_err(|_| OAuthError::Storage)?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|_| OAuthError::Storage)?;
        Ok(conn)
    }

    /// Endpoint is logical, not an assertion of identity. An HTTP gateway must
    /// verify the live session and require exact route/subject equality.
    pub fn user_endpoint(tenant: &str, user: &str) -> Result<String, OAuthError> {
        if !identifier(tenant) || !identifier(user) { return Err(OAuthError::InvalidRequest); }
        Ok(format!("/oauth/{tenant}/users/{user}/authorize"))
    }

    pub fn authorize(
        &self,
        endpoint: &str,
        subject: &AuthenticatedSubject,
        consent_granted: bool,
        request: &AuthorizationRequest,
    ) -> Result<AuthorizationResponse, OAuthError> {
        if endpoint != Self::user_endpoint(&subject.tenant, &subject.user)?
            || !identifier(&subject.user)
            || !consent_granted
            || request.response_type != "code"
            || request.code_challenge_method != "S256"
            || !valid_challenge(&request.code_challenge)
            || !valid_state(&request.state)
        { return Err(OAuthError::InvalidRequest); }
        let client = self.clients.get(&request.client_id).ok_or(OAuthError::AccessDenied)?;
        if client.tenant != subject.tenant
            || client.redirect_uri != request.redirect_uri
            || client.resource != request.resource
            || request.scope.is_empty()
            || !request.scope.is_subset(&client.scopes)
        { return Err(OAuthError::AccessDenied); }
        let now = self.clock.now()?;
        if subject.authenticated_at > now || now - subject.authenticated_at > 3600 {
            return Err(OAuthError::InvalidRequest);
        }
        if !client.step_up_scopes.is_disjoint(&request.scope)
            && (subject.assurance_level < 2 || now - subject.authenticated_at > 300)
        { return Err(OAuthError::StepUpRequired); }
        let code = random_secret()?;
        let digest = hash_secret(&code);
        let scopes = encode_scopes(&request.scope)?;
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO oauth_codes(digest,client,tenant,user_id,redirect_uri,resource,scopes,challenge,assurance,expires_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![digest, client.client_id, subject.tenant, subject.user, request.redirect_uri,
                    request.resource, scopes, request.code_challenge, i64::from(subject.assurance_level), now + CODE_LIFETIME]
        ).map_err(|_| OAuthError::Storage)?;
        Ok(AuthorizationResponse { redirect_uri: client.redirect_uri.clone(), code, state: request.state.clone() })
    }

    /// Atomic single-use exchange; PKCE verification happens before mutation.
    pub fn exchange(&self, request: &CodeExchange) -> Result<TokenResponse, OAuthError> {
        if !valid_verifier(&request.code_verifier) || !valid_token(&request.code) {
            return Err(OAuthError::InvalidGrant);
        }
        let now = self.clock.now()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|_| OAuthError::Storage)?;
        type CodeRow = (String, String, String, String, String, String, String, i64, i64, i64);
        let row: Option<CodeRow> = tx.query_row(
            "SELECT client,tenant,user_id,redirect_uri,resource,scopes,challenge,assurance,expires_at,consumed
             FROM oauth_codes WHERE digest=?1", params![hash_secret(&request.code)],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?))
        ).optional().map_err(|_| OAuthError::Storage)?;
        let (client, tenant, user, _redirect, resource, scopes, challenge, assurance, expiry, used) =
            row.ok_or(OAuthError::InvalidGrant)?;
        let computed = URL_SAFE_NO_PAD.encode(Sha256::digest(request.code_verifier.as_bytes()));
        if used != 0 || expiry <= now || client != request.client_id
            || resource != request.resource
            || !memcmp::eq(challenge.as_bytes(), computed.as_bytes())
        { return Err(OAuthError::InvalidGrant); }
        let updated = tx.execute("UPDATE oauth_codes SET consumed=1 WHERE digest=?1 AND consumed=0",
            params![hash_secret(&request.code)]).map_err(|_| OAuthError::Storage)?;
        if updated != 1 { return Err(OAuthError::InvalidGrant); }
        let family = hash_secret(&random_secret()?);
        tx.execute("INSERT INTO oauth_families(family,tenant,user_id,client) VALUES(?1,?2,?3,?4)",
            params![family,tenant,user,client]).map_err(|_| OAuthError::Storage)?;
        let allowed = self.clients.get(&client).ok_or(OAuthError::InvalidGrant)?;
        let parsed_scopes = decode_scopes(&scopes)?;
        if allowed.tenant != tenant || allowed.resource != resource
            || !parsed_scopes.is_subset(&allowed.scopes) { return Err(OAuthError::InvalidGrant); }
        let step_up_grant = !allowed.step_up_scopes.is_disjoint(&parsed_scopes);
        let issued = insert_token_pair(&tx, &family, &client, &tenant, &user, &resource,
            &scopes, assurance, now, !step_up_grant)?;
        tx.commit().map_err(|_| OAuthError::Storage)?;
        Ok(issued)
    }

    /// Refresh tokens are one-time use. Reuse revokes the entire token family,
    /// including previously issued access tokens, in a single transaction.
    pub fn refresh(&self, request: &RefreshRequest) -> Result<TokenResponse, OAuthError> {
        if !valid_token(&request.refresh_token) { return Err(OAuthError::InvalidGrant); }
        let now = self.clock.now()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|_| OAuthError::Storage)?;
        type RefreshRow = (String, String, String, String, String, String, i64, i64, i64, i64);
        let row: Option<RefreshRow> = tx.query_row(
            "SELECT t.family,t.client,t.tenant,t.user_id,t.resource,t.scopes,t.assurance,t.expires_at,t.consumed,f.revoked
             FROM oauth_tokens t JOIN oauth_families f ON t.family=f.family
             WHERE t.digest=?1 AND t.kind='refresh'", params![hash_secret(&request.refresh_token)],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?))
        ).optional().map_err(|_| OAuthError::Storage)?;
        let (family, client, tenant, user, resource, scopes, assurance, expiry, used, revoked) =
            row.ok_or(OAuthError::InvalidGrant)?;
        if client != request.client_id || resource != request.resource || revoked != 0 {
            return Err(OAuthError::InvalidGrant);
        }
        if used != 0 {
            tx.execute("UPDATE oauth_families SET revoked=1 WHERE family=?1", params![family])
                .map_err(|_| OAuthError::Storage)?;
            tx.commit().map_err(|_| OAuthError::Storage)?;
            return Err(OAuthError::ReplayDetected);
        }
        if expiry <= now { return Err(OAuthError::InvalidGrant); }
        let updated = tx.execute("UPDATE oauth_tokens SET consumed=1 WHERE digest=?1 AND consumed=0",
            params![hash_secret(&request.refresh_token)]).map_err(|_| OAuthError::Storage)?;
        if updated != 1 { return Err(OAuthError::InvalidGrant); }
        let allowed = self.clients.get(&client).ok_or(OAuthError::InvalidGrant)?;
        let parsed_scopes = decode_scopes(&scopes)?;
        if allowed.tenant != tenant || allowed.resource != resource
            || !parsed_scopes.is_subset(&allowed.scopes)
            || !allowed.step_up_scopes.is_disjoint(&parsed_scopes) {
            return Err(OAuthError::InvalidGrant);
        }
        // Refreshed tokens cannot inherit a past step-up authentication.
        let issued = insert_token_pair(&tx, &family, &client, &tenant, &user, &resource,
            &scopes, assurance.min(1), now, true)?;
        tx.commit().map_err(|_| OAuthError::Storage)?;
        Ok(issued)
    }

    /// Checks audience, tenant, user, scope and assurance on EVERY invocation.
    pub fn validate_access(
        &self, token: &str, tenant: &str, user: &str, resource: &str,
        required_scopes: &BTreeSet<String>, min_assurance: u8,
    ) -> Result<AuthorizedGrant, OAuthError> {
        if !valid_token(token) { return Err(OAuthError::AccessDenied); }
        let now = self.clock.now()?;
        let conn = self.connection()?;
        type AccessRow = (String, String, String, String, String, String, i64, i64, i64, i64);
        let row: Option<AccessRow> = conn.query_row(
            "SELECT t.client,t.tenant,t.user_id,t.resource,t.scopes,t.family,t.assurance,t.expires_at,t.consumed,f.revoked
             FROM oauth_tokens t JOIN oauth_families f ON t.family=f.family
             WHERE t.digest=?1 AND t.kind='access'", params![hash_secret(token)],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?))
        ).optional().map_err(|_| OAuthError::Storage)?;
        let (client, token_tenant, token_user, token_resource, encoded_scopes, _family,
            assurance, expiry, used, revoked) = row.ok_or(OAuthError::AccessDenied)?;
        let scopes = decode_scopes(&encoded_scopes)?;
        let client_config = self.clients.get(&client).ok_or(OAuthError::AccessDenied)?;
        if client_config.tenant != token_tenant || client_config.resource != token_resource
            || !scopes.is_subset(&client_config.scopes) { return Err(OAuthError::AccessDenied); }
        if expiry <= now || used != 0 || revoked != 0 || token_tenant != tenant
            || token_user != user || token_resource != resource
            || assurance < i64::from(min_assurance) || !required_scopes.is_subset(&scopes)
        { return Err(OAuthError::AccessDenied); }
        Ok(AuthorizedGrant { tenant: token_tenant, user: token_user, client_id: client,
            resource: token_resource, scopes, assurance_level: u8::try_from(assurance).map_err(|_| OAuthError::Storage)? })
    }

    /// Atomic user-wide emergency revocation. No partial success is reported.
    pub fn revoke_user(&self, tenant: &str, user: &str) -> Result<(), OAuthError> {
        if !identifier(tenant) || !identifier(user) { return Err(OAuthError::InvalidRequest); }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|_| OAuthError::Storage)?;
        tx.execute("UPDATE oauth_codes SET consumed=1 WHERE tenant=?1 AND user_id=?2", params![tenant,user])
            .map_err(|_| OAuthError::Storage)?;
        tx.execute("UPDATE oauth_families SET revoked=1 WHERE tenant=?1 AND user_id=?2", params![tenant,user])
            .map_err(|_| OAuthError::Storage)?;
        tx.commit().map_err(|_| OAuthError::Storage)
    }
}

fn insert_token_pair(
    tx: &rusqlite::Transaction<'_>, family: &str, client: &str, tenant: &str,
    user: &str, resource: &str, scopes: &str, assurance: i64, now: i64, allow_refresh: bool,
) -> Result<TokenResponse, OAuthError> {
    let access = random_secret()?;
    let refresh = if allow_refresh { random_secret()? } else { String::new() };
    let tokens = if allow_refresh { vec![(&access, "access", ACCESS_LIFETIME), (&refresh, "refresh", REFRESH_LIFETIME)] }
        else { vec![(&access, "access", ACCESS_LIFETIME)] };
    for (token, kind, lifetime) in tokens {
        tx.execute(
            "INSERT INTO oauth_tokens(digest,kind,family,client,tenant,user_id,resource,scopes,assurance,expires_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![hash_secret(token),kind,family,client,tenant,user,resource,scopes,assurance,now+lifetime]
        ).map_err(|_| OAuthError::Storage)?;
    }
    let parsed = decode_scopes(scopes)?;
    Ok(TokenResponse { token_type: "Bearer", access_token: access, refresh_token: refresh,
        expires_in: ACCESS_LIFETIME, scope: parsed.into_iter().collect::<Vec<_>>().join(" ") })
}

/// The DB and its WAL directory are operator-private. SQLite NOFOLLOW also
/// protects against final-component symlink swaps at connection time.
pub(crate) fn private_database(path: &Path) -> bool {
    if !path.is_absolute() { return false; }
    let Ok(file) = path.symlink_metadata() else { return false; };
    let Some(parent) = path.parent() else { return false; };
    let Ok(directory) = parent.symlink_metadata() else { return false; };
    if !file.file_type().is_file() || !directory.file_type().is_dir() { return false; }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if file.permissions().mode() & 0o077 != 0
            || directory.permissions().mode() & 0o077 != 0 { return false; }
    }
    true
}

fn random_secret() -> Result<String, OAuthError> {
    let mut bytes = [0u8; 32];
    rand_bytes(&mut bytes).map_err(|_| OAuthError::Entropy)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn hash_secret(value: &str) -> String { format!("{:x}", Sha256::digest(value.as_bytes())) }
fn identifier(value: &str) -> bool {
    !value.is_empty() && value != "." && value != ".." && value.len() <= 128 && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn scope_name(value: &str) -> bool {
    !value.is_empty() && value != "." && value != ".." && value.len() <= 128 && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn https_url(value: &str) -> bool {
    if !value.starts_with("https://") || value.len() > 2048 || value.bytes().any(|b| b <= 32 || b == b'\\' || b == b'#') { return false; }
    let authority = value[8..].split(['/', '?']).next().unwrap_or("");
    !authority.is_empty() && !authority.contains('@') && !authority.starts_with(':') && !authority.ends_with(':')
}
fn valid_challenge(value: &str) -> bool {
    value.len() == 43 && URL_SAFE_NO_PAD.decode(value).map(|b| b.len() == 32 && URL_SAFE_NO_PAD.encode(b) == value).unwrap_or(false)
}
fn valid_token(value: &str) -> bool { valid_challenge(value) }
fn valid_verifier(value: &str) -> bool {
    (43..=128).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}
fn valid_state(value: &str) -> bool {
    (16..=256).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}
fn encode_scopes(scopes: &BTreeSet<String>) -> Result<String, OAuthError> {
    serde_json::to_string(scopes).map_err(|_| OAuthError::Storage)
}
fn decode_scopes(value: &str) -> Result<BTreeSet<String>, OAuthError> {
    serde_json::from_str(value).map_err(|_| OAuthError::Storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};
    struct TestClock(AtomicI64);
    impl Clock for TestClock { fn now(&self) -> Result<i64, OAuthError> { Ok(self.0.load(Ordering::SeqCst)) } }
    fn client(tenant: &str) -> OAuthClient {
        OAuthClient { client_id: format!("client-{tenant}"), tenant: tenant.into(),
            redirect_uri: "https://client.example/callback".into(), resource: "https://omat.example".into(),
            scopes: ["tool:read".into(), "tool:write".into()].into_iter().collect(),
            step_up_scopes: ["tool:write".into()].into_iter().collect() }
    }
    fn fixture() -> (tempfile::TempDir, OAuthService, Arc<TestClock>) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("oauth.sqlite");
        Connection::open(&db).unwrap();
        let clock = Arc::new(TestClock(AtomicI64::new(2_000_000)));
        let service = OAuthService::open_inner(&db, vec![client("tenant-a"), client("tenant-b")], clock.clone()).unwrap();
        (dir, service, clock)
    }
    fn subject(tenant: &str) -> AuthenticatedSubject {
        AuthenticatedSubject { tenant: tenant.into(), user: "alice".into(), authenticated_at: 2_000_000, assurance_level: 2 }
    }
    fn auth(tenant: &str) -> AuthorizationRequest {
        let verifier = "a".repeat(64);
        AuthorizationRequest { response_type: "code".into(), client_id: format!("client-{tenant}"),
            redirect_uri: "https://client.example/callback".into(), resource: "https://omat.example".into(),
            scope: ["tool:read".into()].into_iter().collect(), state: "s".repeat(32),
            code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
            code_challenge_method: "S256".into() }
    }
    fn code(s: &OAuthService, tenant: &str) -> String {
        s.authorize(&OAuthService::user_endpoint(tenant,"alice").unwrap(), &subject(tenant), true, &auth(tenant)).unwrap().code
    }
    fn exchange(s: &OAuthService, tenant: &str, code: String) -> Result<TokenResponse,OAuthError> {
        s.exchange(&CodeExchange { client_id: format!("client-{tenant}"), code,
            code_verifier: "a".repeat(64), resource: "https://omat.example".into() })
    }
    fn read_scope() -> BTreeSet<String> { ["tool:read".into()].into_iter().collect() }

    #[test]
    fn code_pkce_token_refresh_and_family_replay_revocation() {
        let (_dir,s,_) = fixture();
        let code = code(&s,"tenant-a");
        let first = exchange(&s,"tenant-a",code.clone()).unwrap();
        assert_eq!(exchange(&s,"tenant-a",code), Err(OAuthError::InvalidGrant));
        assert!(s.validate_access(&first.access_token,"tenant-a","alice","https://omat.example",&read_scope(),1).is_ok());
        let second = s.refresh(&RefreshRequest {client_id:"client-tenant-a".into(),refresh_token:first.refresh_token.clone(),resource:"https://omat.example".into()}).unwrap();
        assert_eq!(s.refresh(&RefreshRequest {client_id:"client-tenant-a".into(),refresh_token:first.refresh_token,resource:"https://omat.example".into()}), Err(OAuthError::ReplayDetected));
        assert_eq!(s.validate_access(&second.access_token,"tenant-a","alice","https://omat.example",&read_scope(),1), Err(OAuthError::AccessDenied));
    }
    #[test]
    fn rejects_cross_tenant_cross_user_wrong_audience_and_scope() {
        let (_dir,s,_) = fixture();
        let token = exchange(&s,"tenant-a",code(&s,"tenant-a")).unwrap();
        assert_eq!(s.validate_access(&token.access_token,"tenant-b","alice","https://omat.example",&read_scope(),1),Err(OAuthError::AccessDenied));
        assert_eq!(s.validate_access(&token.access_token,"tenant-a","bob","https://omat.example",&read_scope(),1),Err(OAuthError::AccessDenied));
        assert_eq!(s.validate_access(&token.access_token,"tenant-a","alice","https://other.example",&read_scope(),1),Err(OAuthError::AccessDenied));
        assert_eq!(s.validate_access(&token.access_token,"tenant-a","alice","https://omat.example",&["tool:write".into()].into_iter().collect(),1),Err(OAuthError::AccessDenied));
    }
    #[test]
    fn rejects_pkce_plain_redirect_substitution_and_code_expiry() {
        let (_dir,s,clock) = fixture();
        let mut r=auth("tenant-a");r.code_challenge_method="plain".into();
        assert!(s.authorize(&OAuthService::user_endpoint("tenant-a","alice").unwrap(),&subject("tenant-a"),true,&r).is_err());
        let mut r=auth("tenant-a");r.redirect_uri="https://evil.example".into();
        assert!(s.authorize(&OAuthService::user_endpoint("tenant-a","alice").unwrap(),&subject("tenant-a"),true,&r).is_err());
        let code=code(&s,"tenant-a");
        assert_eq!(s.exchange(&CodeExchange{client_id:"client-tenant-a".into(),code:code.clone(),code_verifier:"b".repeat(64),resource:"https://omat.example".into()}),Err(OAuthError::InvalidGrant));
        clock.0.store(2_000_100,Ordering::SeqCst);
        assert_eq!(exchange(&s,"tenant-a",code),Err(OAuthError::InvalidGrant));
    }
    #[test]
    fn requires_verified_session_consent_and_step_up() {
        let (_dir,s,_) = fixture();
        let mut r=auth("tenant-a");r.scope.insert("tool:write".into());
        let endpoint=OAuthService::user_endpoint("tenant-a","alice").unwrap();
        assert!(s.authorize(&endpoint,&subject("tenant-a"),false,&r).is_err());
        assert!(s.authorize("/oauth/tenant-b/users/alice/authorize",&subject("tenant-a"),true,&r).is_err());
        let mut weak=subject("tenant-a");weak.assurance_level=1;
        assert_eq!(s.authorize(&endpoint,&weak,true,&r).err(),Some(OAuthError::StepUpRequired));
        let authorization=s.authorize(&endpoint,&subject("tenant-a"),true,&r).unwrap();
        let sensitive=exchange(&s,"tenant-a",authorization.code).unwrap();
        assert!(sensitive.refresh_token.is_empty(),"step-up grants must not silently refresh");
    }
    #[test]
    fn user_revoke_and_restart_are_durable() {
        let (dir,s,_) = fixture();
        let token=exchange(&s,"tenant-a",code(&s,"tenant-a")).unwrap();
        s.revoke_user("tenant-a","alice").unwrap();
        drop(s);
        let reopened=OAuthService::open_inner(&dir.path().join("oauth.sqlite"),vec![client("tenant-a")],Arc::new(TestClock(AtomicI64::new(2_000_000)))).unwrap();
        assert_eq!(reopened.validate_access(&token.access_token,"tenant-a","alice","https://omat.example",&read_scope(),1),Err(OAuthError::AccessDenied));
    }
    #[test]
    fn concurrent_code_redemption_is_exactly_once() {
        let (_dir,service,_) = fixture();
        let code = code(&service,"tenant-a");
        let service = Arc::new(service);
        let threads: Vec<_> = (0..12).map(|_| {
            let service=Arc::clone(&service);
            let code=code.clone();
            std::thread::spawn(move || exchange(&service,"tenant-a",code).is_ok())
        }).collect();
        let successes=threads.into_iter().map(|h|h.join().unwrap()).filter(|ok|*ok).count();
        assert_eq!(successes,1,"SQLite IMMEDIATE transaction must serialize redemption");
    }
    #[test]
    fn concurrent_refresh_rotation_detects_replay_and_revokes_family() {
        let (_dir,service,_) = fixture();
        let first = exchange(&service,"tenant-a",code(&service,"tenant-a")).unwrap();
        let service = Arc::new(service);
        let token = first.refresh_token;
        let threads: Vec<_> = (0..8).map(|_| {
            let service=Arc::clone(&service);
            let token=token.clone();
            std::thread::spawn(move || service.refresh(&RefreshRequest {
                client_id:"client-tenant-a".into(),refresh_token:token,
                resource:"https://omat.example".into() }))
        }).collect();
        let results: Vec<_> = threads.into_iter().map(|h|h.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r|r.is_ok()).count(),1);
        assert!(results.iter().any(|r|matches!(r,Err(OAuthError::ReplayDetected))));
        let issued = results.into_iter().find_map(Result::ok).unwrap();
        assert_eq!(service.validate_access(&issued.access_token,"tenant-a","alice",
            "https://omat.example",&read_scope(),1),Err(OAuthError::AccessDenied));
    }
    #[test]
    fn expires_access_tokens_and_rejects_cross_tenant_code_redemption() {
        let (_dir,service,clock)=fixture();
        let first=code(&service,"tenant-a");
        assert_eq!(exchange(&service,"tenant-b",first.clone()),Err(OAuthError::InvalidGrant));
        let token=exchange(&service,"tenant-a",first).unwrap();
        clock.0.store(2_000_301,Ordering::SeqCst);
        assert_eq!(service.validate_access(&token.access_token,"tenant-a","alice",
            "https://omat.example",&read_scope(),1),Err(OAuthError::AccessDenied));
    }
    #[test]
    fn rejects_missing_database_and_bad_client_configuration() {
        let dir=tempfile::tempdir().unwrap();
        assert!(matches!(OAuthService::open(&dir.path().join("missing"),vec![client("tenant-a")]),Err(OAuthError::Configuration)));
        let db=dir.path().join("oauth.sqlite");Connection::open(&db).unwrap();
        let mut bad=client("tenant-a");bad.redirect_uri="http://127.0.0.1/callback".into();
        assert!(matches!(OAuthService::open(&db,vec![bad]),Err(OAuthError::Configuration)));
    }
}
