//! Stage 0.52: private per-UID Unix socket bridge to BuilderHttpBoundary.
//! The public mTLS ingress is scripts/builder_cloud_ingress.py; do not expose
//! this socket or the service UID to untrusted workloads.
#[cfg(unix)]
mod unix {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use ocaf_runtime::builder_http::{BuilderHttpBoundary, BuilderHttpRequest};
    use ocaf_runtime::builder_store::SqliteBuilderStore;
    use ocaf_runtime::stage044_oauth::{AuthenticatedSubject, OAuthClient, OAuthService};
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeSet;
    use std::fs;
    use std::io;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::task::JoinSet;
    use tokio::time::{timeout, Duration};

    const MAX_FRAME: usize = 256 * 1024;
    const MAX_BODY: usize = 128 * 1024;
    const MAX_CLIENTS: usize = 32;
    const DEADLINE: Duration = Duration::from_secs(6);

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ClientConfig {
        client_id: String,
        tenant: String,
        redirect_uri: String,
        resource: String,
        scopes: BTreeSet<String>,
        step_up_scopes: BTreeSet<String>,
    }
    impl From<ClientConfig> for OAuthClient {
        fn from(c: ClientConfig) -> Self {
            Self {
                client_id: c.client_id,
                tenant: c.tenant,
                redirect_uri: c.redirect_uri,
                resource: c.resource,
                scopes: c.scopes,
                step_up_scopes: c.step_up_scopes,
            }
        }
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Config {
        origin: String,
        audience: String,
        socket: PathBuf,
        oauth_database: PathBuf,
        builder_database: PathBuf,
        clients: Vec<ClientConfig>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Subject {
        tenant: String,
        user: String,
        authenticated_at: i64,
        assurance_level: u8,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        subject: Subject,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body_b64: String,
    }
    #[derive(Serialize)]
    struct Response {
        status: u16,
        body_b64: String,
    }
    struct State {
        oauth: OAuthService,
        store: SqliteBuilderStore,
        audience: String,
        origin: String,
    }

    fn invalid() -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid bridge frame/configuration",
        )
    }
    fn uid() -> u32 {
        unsafe { libc::geteuid() }
    }
    fn private_parent(path: &Path) -> io::Result<()> {
        let parent = path.parent().ok_or_else(invalid)?;
        if !path.is_absolute() || parent.canonicalize()? != parent {
            return Err(invalid());
        }
        let meta = parent.symlink_metadata()?;
        if !meta.is_dir() || meta.uid() != uid() || meta.mode() & 0o077 != 0 {
            return Err(invalid());
        }
        Ok(())
    }
    fn private_file(path: &Path) -> io::Result<()> {
        private_parent(path)?;
        let meta = path.symlink_metadata()?;
        if !meta.is_file() || meta.uid() != uid() || meta.mode() & 0o077 != 0 || meta.nlink() != 1 {
            return Err(invalid());
        }
        Ok(())
    }
    fn identifier(s: &str) -> bool {
        !s.is_empty()
            && s.len() <= 64
            && s != "."
            && s != ".."
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    }
    fn now() -> io::Result<i64> {
        i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| invalid())?
                .as_secs(),
        )
        .map_err(|_| invalid())
    }
    fn error_response(status: u16) -> Response {
        Response {
            status,
            body_b64: STANDARD.encode(format!(r#"{{"error":"http_{status}"}}"#)),
        }
    }
    fn dispatch(state: &State, raw: &[u8]) -> Response {
        let Ok(envelope) = serde_json::from_slice::<Envelope>(raw) else {
            return error_response(400);
        };
        let s = envelope.subject;
        let Ok(current) = now() else {
            return error_response(503);
        };
        if !identifier(&s.tenant)
            || !identifier(&s.user)
            || s.assurance_level != 1
            || s.authenticated_at > current
            || current.saturating_sub(s.authenticated_at) > 30
        {
            return error_response(403);
        }
        let Ok(body) = STANDARD.decode(envelope.body_b64) else {
            return error_response(400);
        };
        if body.len() > MAX_BODY {
            return error_response(413);
        }
        let subject = AuthenticatedSubject {
            tenant: s.tenant,
            user: s.user,
            authenticated_at: s.authenticated_at,
            assurance_level: s.assurance_level,
        };
        let headers: Vec<(&str, &str)> = envelope
            .headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let Ok(api) =
            BuilderHttpBoundary::new(&state.oauth, &state.store, &state.audience, &state.origin)
        else {
            return error_response(503);
        };
        let result = api.handle(
            BuilderHttpRequest {
                method: &envelope.method,
                path: &envelope.path,
                headers: &headers,
                body: &body,
            },
            &subject,
        );
        if result.body.len() > MAX_BODY {
            return error_response(503);
        }
        Response {
            status: result.status,
            body_b64: STANDARD.encode(result.body),
        }
    }
    async fn handle(mut stream: UnixStream, state: Arc<State>) -> io::Result<()> {
        if stream.peer_cred()?.uid() != uid() {
            return Err(invalid());
        }
        let raw = timeout(DEADLINE, async {
            let n = stream.read_u32().await? as usize;
            if n == 0 || n > MAX_FRAME {
                return Err(invalid());
            }
            let mut raw = vec![0; n];
            stream.read_exact(&mut raw).await?;
            Ok::<Vec<u8>, io::Error>(raw)
        })
        .await
        .map_err(|_| invalid())??;
        let response = tokio::task::spawn_blocking(move || dispatch(&state, &raw))
            .await
            .map_err(|_| invalid())?;
        let raw = serde_json::to_vec(&response).map_err(|_| invalid())?;
        if raw.len() > MAX_FRAME {
            return Err(invalid());
        }
        timeout(DEADLINE, async {
            stream.write_u32(raw.len() as u32).await?;
            stream.write_all(&raw).await?;
            stream.shutdown().await
        })
        .await
        .map_err(|_| invalid())?
    }
    struct SocketGuard(PathBuf);
    impl Drop for SocketGuard {
        fn drop(&mut self) {
            if self
                .0
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_socket() && m.uid() == uid())
            {
                let _ = fs::remove_file(&self.0);
            }
        }
    }
    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from(
            std::env::args_os()
                .nth(1)
                .ok_or("usage: ocaf-builder-cloud-bridge /private/config.json")?,
        );
        private_file(&path)?;
        if path.metadata()?.len() > 2 * 1024 * 1024 {
            return Err(invalid().into());
        }
        let cfg: Config = serde_json::from_slice(&fs::read(path)?)?;
        private_parent(&cfg.socket)?;
        private_file(&cfg.oauth_database)?;
        private_file(&cfg.builder_database)?;
        if cfg.oauth_database == cfg.builder_database {
            return Err(invalid().into());
        }
        let oauth = OAuthService::open(
            &cfg.oauth_database,
            cfg.clients.into_iter().map(Into::into).collect(),
        )?;
        let store = SqliteBuilderStore::open_existing(&cfg.builder_database)?;
        BuilderHttpBoundary::new(&oauth, &store, &cfg.audience, &cfg.origin)?;
        let state = Arc::new(State {
            oauth,
            store,
            audience: cfg.audience,
            origin: cfg.origin,
        });
        // bind fails if a prior socket exists; never unlink arbitrary input at startup.
        let listener = UnixListener::bind(&cfg.socket)?;
        let _guard = SocketGuard(cfg.socket.clone());
        fs::set_permissions(&cfg.socket, fs::Permissions::from_mode(0o600))?;
        let mut tasks = JoinSet::new();
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let interrupt = tokio::signal::ctrl_c();
        tokio::pin!(interrupt);
        loop {
            tokio::select! {
                _=&mut interrupt=>break,
                _=term.recv()=>break,
                _=tasks.join_next(), if !tasks.is_empty()=>{},
                accepted=listener.accept(), if tasks.len()<MAX_CLIENTS=>{
                    let (stream,_)=accepted?;let state=Arc::clone(&state);
                    tasks.spawn(async move {let _=handle(stream,state).await;});
                }
            }
        }
        drop(listener);
        while tasks.join_next().await.is_some() {}
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use ocaf_runtime::stage044_oauth::{AuthorizationRequest, CodeExchange};
        use serde_json::json;
        use sha2::{Digest, Sha256};
        fn scopes() -> BTreeSet<String> {
            ["builder:read".into(), "builder:write".into()]
                .into_iter()
                .collect()
        }
        fn fixture() -> (tempfile::TempDir, State, String) {
            let dir = tempfile::tempdir().unwrap();
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
            let a = dir.path().join("oauth.sqlite");
            let b = dir.path().join("builder.sqlite");
            for p in [&a, &b] {
                fs::File::create(p).unwrap();
                fs::set_permissions(p, fs::Permissions::from_mode(0o600)).unwrap();
            }
            let resource = "https://builder.example";
            let oauth = OAuthService::open(
                &a,
                vec![OAuthClient {
                    client_id: "client".into(),
                    tenant: "local".into(),
                    redirect_uri: "https://client.example/cb".into(),
                    resource: resource.into(),
                    scopes: scopes(),
                    step_up_scopes: BTreeSet::new(),
                }],
            )
            .unwrap();
            let store = SqliteBuilderStore::open_existing(&b).unwrap();
            let verifier = "a".repeat(64);
            let auth = AuthenticatedSubject {
                tenant: "local".into(),
                user: "alice".into(),
                authenticated_at: now().unwrap(),
                assurance_level: 1,
            };
            let code = oauth
                .authorize(
                    &OAuthService::user_endpoint("local", "alice").unwrap(),
                    &auth,
                    true,
                    &AuthorizationRequest {
                        response_type: "code".into(),
                        client_id: "client".into(),
                        redirect_uri: "https://client.example/cb".into(),
                        resource: resource.into(),
                        scope: scopes(),
                        state: "s".repeat(32),
                        code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
                        code_challenge_method: "S256".into(),
                    },
                )
                .unwrap();
            let token = oauth
                .exchange(&CodeExchange {
                    client_id: "client".into(),
                    code: code.code,
                    code_verifier: verifier,
                    resource: resource.into(),
                })
                .unwrap()
                .access_token;
            (
                dir,
                State {
                    oauth,
                    store,
                    audience: resource.into(),
                    origin: resource.into(),
                },
                token,
            )
        }
        fn request(method: &str, path: &str, token: &str, tenant: &str, body: &[u8]) -> Vec<u8> {
            serde_json::to_vec(&json!({"subject":{"tenant":tenant,"user":"alice","authenticated_at":now().unwrap(),"assurance_level":1},"method":method,"path":path,"headers":[["Host","builder.example"],["Origin","https://builder.example"],["Authorization",format!("Bearer {token}")],["Content-Type","application/json"],["Content-Length",body.len().to_string()]],"body_b64":STANDARD.encode(body)})).unwrap()
        }
        #[test]
        fn authenticated_preview_save_load_and_cross_tenant_rejection() {
            let (_dir, state, token) = fixture();
            let project: serde_json::Value = serde_json::from_str(include_str!(
                "../../apps/agents-builder-web/fixtures/hello.json"
            ))
            .unwrap();
            let body = serde_json::to_vec(&project).unwrap();
            assert_eq!(
                dispatch(
                    &state,
                    &request("POST", "/v1/projects/preview", &token, "local", &body)
                )
                .status,
                200
            );
            let save = serde_json::to_vec(&json!({"project":project,"expected":null})).unwrap();
            assert_eq!(
                dispatch(
                    &state,
                    &request("PUT", "/v1/projects/hello", &token, "local", &save)
                )
                .status,
                200
            );
            assert_eq!(
                dispatch(
                    &state,
                    &request("GET", "/v1/projects/hello", &token, "local", b"")
                )
                .status,
                200
            );
            assert_eq!(
                dispatch(
                    &state,
                    &request("GET", "/v1/projects/hello", &token, "another", b"")
                )
                .status,
                403
            );
            assert_eq!(
                dispatch(
                    &state,
                    &request("GET", "/v1/projects/hello", "invalid", "local", b"")
                )
                .status,
                403
            );
            assert_eq!(
                dispatch(
                    &state,
                    &request("PUT", "/v1/projects/hello", &token, "local", &save)
                )
                .status,
                409
            );
        }
        #[tokio::test]
        async fn real_unix_socket_reuses_boundary() {
            let (dir, state, token) = fixture();
            let path = dir.path().join("bridge.sock");
            let server = UnixListener::bind(&path).unwrap();
            let state = Arc::new(state);
            let task = tokio::spawn(async move {
                let (stream, _) = server.accept().await.unwrap();
                handle(stream, state).await.unwrap();
            });
            let mut stream = UnixStream::connect(path).await.unwrap();
            let raw = request("GET", "/v1/projects/missing", &token, "local", b"");
            stream.write_u32(raw.len() as u32).await.unwrap();
            stream.write_all(&raw).await.unwrap();
            let size = stream.read_u32().await.unwrap() as usize;
            assert!(size <= MAX_FRAME);
            let mut buf = vec![0; size];
            stream.read_exact(&mut buf).await.unwrap();
            let response: serde_json::Value = serde_json::from_slice(&buf).unwrap();
            assert_eq!(response["status"], 404);
            task.await.unwrap();
        }
    }
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unix::run().await
}

#[cfg(not(unix))]
fn main() {
    eprintln!("Builder Cloud bridge requires a Unix server");
    std::process::exit(1);
}
