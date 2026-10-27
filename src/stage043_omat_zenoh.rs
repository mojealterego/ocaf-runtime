//! Authenticated Zenoh ingress for OMAt. Zenoh transport encryption is an
//! operator configuration responsibility; message authentication is mandatory.
use crate::omat_server::{OmatPolicy, OmatRegistry, ToolCall};
use crate::stage043_omat_route::{dispatch_tool_call, AuthenticatedPeer, OmatRouteGuard};
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

#[derive(Default)]
pub struct OmatSecurityMetrics {
    pub requests: AtomicU64,
    pub rejections: AtomicU64,
    pub replays: AtomicU64,
    pub connection_events: AtomicU64,
}

impl OmatSecurityMetrics {
    pub fn prometheus(&self) -> String {
        format!(
            "ocaf_omat_requests_total {}\nocaf_omat_rejections_total {}\nocaf_omat_replays_total {}\nocaf_omat_connection_events_total {}\n",
            self.requests.load(Ordering::Relaxed),
            self.rejections.load(Ordering::Relaxed),
            self.replays.load(Ordering::Relaxed),
            self.connection_events.load(Ordering::Relaxed),
        )
    }
}

const MAX_FRAME: usize = 256 * 1024;
const MAX_REPLAY: usize = 100_000;

/// SQLite also creates -wal/-shm sidecars. The containing directory is part
/// of the replay authority and must not be writable/readable by other users.
fn prepare_private_replay_file(path: &Path) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(
        path.is_absolute() && !path.components().any(|c| c == Component::ParentDir),
        "replay database path must be absolute and normalized"
    );
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("replay database parent missing"))?;
        // Reject a symlink as the final directory component, then resolve
        // possible symlinked ancestors once and use only that resolved path.
        anyhow::ensure!(parent.symlink_metadata()?.is_dir(), "invalid replay parent");
        let canonical_parent = parent.canonicalize()?;
        let parent_meta = canonical_parent.symlink_metadata()?;
        anyhow::ensure!(
            parent_meta.is_dir()
                && parent_meta.uid() == unsafe { libc::geteuid() }
                && parent_meta.permissions().mode() & 0o077 == 0,
            "replay database needs an owned private directory"
        );
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("replay database file name missing"))?;
        let canonical_path = canonical_parent.join(file_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&canonical_path)
        {
            Ok(file) => drop(file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let file_meta = canonical_path.symlink_metadata()?;
        anyhow::ensure!(
            file_meta.is_file()
                && file_meta.uid() == unsafe { libc::geteuid() }
                && file_meta.permissions().mode() & 0o077 == 0
                && file_meta.nlink() == 1,
            "replay database needs an owned private regular file"
        );
        Ok(canonical_path)
    }
    #[cfg(not(unix))]
    anyhow::bail!("private replay database requires a Unix host");
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedOmatRequest {
    pub principal: String,
    pub topic: String,
    pub nonce: String,
    pub call: ToolCall,
    pub mac_hex: String,
}

pub struct OmatZenohAdmission {
    metrics: Arc<OmatSecurityMetrics>,
    keys: HashMap<String, [u8; 32]>,
    guard: OmatRouteGuard,
    used: HashSet<(String, String)>,
    replay_db: Option<Connection>,
    registry: OmatRegistry,
    policies: HashMap<String, OmatPolicy>,
}

impl OmatZenohAdmission {
    pub fn new(
        keys: HashMap<String, [u8; 32]>,
        tenants: HashMap<String, String>,
        policies: HashMap<String, OmatPolicy>,
        registry: OmatRegistry,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !keys.is_empty() && keys.len() == tenants.len(),
            "invalid peer keys"
        );
        anyhow::ensure!(
            keys.keys().all(|k| tenants.contains_key(k)),
            "missing tenant binding"
        );
        anyhow::ensure!(keys.values().all(|k| *k != [0; 32]), "zero HMAC key");
        // A principal is an authority boundary. If two names share a secret,
        // either peer can sign as the other and cross its tenant binding.
        anyhow::ensure!(
            keys.values().collect::<HashSet<_>>().len() == keys.len(),
            "HMAC keys must be distinct per principal"
        );
        let guard =
            OmatRouteGuard::new(tenants).map_err(|_| anyhow::anyhow!("invalid tenant mapping"))?;
        Ok(Self {
            metrics: Arc::new(OmatSecurityMetrics::default()),
            keys,
            guard,
            used: HashSet::new(),
            replay_db: None,
            registry,
            policies,
        })
    }

    pub fn metrics(&self) -> Arc<OmatSecurityMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Persist replay identities across process restarts. SQLite uniqueness
    /// is the authority; an in-memory set is only a fast path.
    pub fn with_replay_database(mut self, path: &Path) -> anyhow::Result<Self> {
        let canonical_path = prepare_private_replay_file(path)?;
        let db = Connection::open_with_flags(
            &canonical_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS omat_replay (principal TEXT NOT NULL, nonce TEXT NOT NULL, PRIMARY KEY(principal, nonce));")?;
        self.replay_db = Some(db);
        Ok(self)
    }

    /// A session must be established before accepting requests. Never use a
    /// caller-provided tenant as the source of authorization authority.
    pub fn connected(&mut self, connected: bool) {
        self.guard.set_transport_connected(connected);
        self.metrics
            .connection_events
            .fetch_add(1, Ordering::Relaxed);
        tracing::info!(connected, "omat_zenoh_transport_state");
    }

    pub fn handle(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.handle_checked(bytes, None)
    }

    /// Bind the signed topic to the actual Zenoh selector before consuming
    /// the nonce. A frame routed to another listener must not be accepted.
    pub fn handle_for_topic(&mut self, bytes: &[u8], actual_topic: &str) -> anyhow::Result<()> {
        self.handle_checked(bytes, Some(actual_topic))
    }

    fn handle_checked(&mut self, bytes: &[u8], actual_topic: Option<&str>) -> anyhow::Result<()> {
        self.metrics.requests.fetch_add(1, Ordering::Relaxed);
        let outcome = self.handle_authenticated(bytes, actual_topic);
        if outcome.is_err() {
            self.metrics.rejections.fetch_add(1, Ordering::Relaxed);
        }
        outcome
    }

    fn handle_authenticated(
        &mut self,
        bytes: &[u8],
        actual_topic: Option<&str>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(bytes.len() <= MAX_FRAME, "frame too large");
        let request: SignedOmatRequest = serde_json::from_slice(bytes)?;
        if let Some(topic) = actual_topic {
            anyhow::ensure!(
                request.topic == topic,
                "signed topic does not match Zenoh selector"
            );
        }
        anyhow::ensure!(
            request.nonce.len() >= 16
                && request.nonce.len() <= 128
                && request.nonce.bytes().all(|b| b.is_ascii_alphanumeric()),
            "invalid nonce"
        );
        let key = self
            .keys
            .get(&request.principal)
            .ok_or_else(|| anyhow::anyhow!("unknown principal"))?;
        let mut mac =
            HmacSha256::new_from_slice(key).map_err(|_| anyhow::anyhow!("invalid key"))?;
        let canonical = serde_json::to_vec(&(
            &request.principal,
            &request.topic,
            &request.nonce,
            &request.call,
        ))?;
        mac.update(&canonical);
        let signature = decode_hex(&request.mac_hex)?;
        mac.verify_slice(&signature)
            .map_err(|_| anyhow::anyhow!("bad signature"))?;
        anyhow::ensure!(
            self.guard.tenant_for(&request.principal).is_some(),
            "unknown tenant"
        );
        let identity = (request.principal.clone(), request.nonce);
        if self.used.contains(&identity) {
            self.metrics.replays.fetch_add(1, Ordering::Relaxed);
            anyhow::bail!("replay denied");
        }
        anyhow::ensure!(
            self.used.len() < MAX_REPLAY || self.replay_db.is_some(),
            "replay store full"
        );
        if let Some(db) = &self.replay_db {
            let inserted = db.execute(
                "INSERT OR IGNORE INTO omat_replay (principal, nonce) VALUES (?1, ?2)",
                params![&identity.0, &identity.1],
            )?;
            if inserted != 1 {
                self.metrics.replays.fetch_add(1, Ordering::Relaxed);
                anyhow::bail!("replay denied");
            }
        }
        if self.used.len() < MAX_REPLAY {
            self.used.insert(identity);
        }
        let tenant = self
            .guard
            .tenant_for(&request.principal)
            .ok_or_else(|| anyhow::anyhow!("unknown tenant"))?
            .to_owned();
        let peer = AuthenticatedPeer {
            principal: request.principal,
            tenant: tenant.clone(),
        };
        let policy = self
            .policies
            .get(&tenant)
            .ok_or_else(|| anyhow::anyhow!("no tenant policy"))?;
        dispatch_tool_call(
            &self.guard,
            &peer,
            &request.topic,
            &request.call,
            policy,
            &self.registry,
        )
        .map(|_| ())
        .map_err(|_| anyhow::anyhow!("OMAt request denied"))
    }
}

fn decode_hex(value: &str) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(value.len() == 64, "invalid MAC length");
    (0..32)
        .map(|i| u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(Into::into))
        .collect()
}

/// Zenoh queryable: authenticated ingress; all requests remain fail-closed
/// until the separately attested OSTc executor is configured.
pub async fn serve_omat_zenoh(
    session: Arc<zenoh::Session>,
    admission: OmatZenohAdmission,
    key: &str,
) -> anyhow::Result<()> {
    serve_omat_zenoh_until(session, admission, key, std::future::pending()).await
}

/// Complete the current request/reply before honoring a graceful shutdown.
/// The operator must keep the old listener alive until its upstream routing
/// is drained; this only guarantees completion of the request being handled.
pub async fn serve_omat_zenoh_until<F>(
    session: Arc<zenoh::Session>,
    admission: OmatZenohAdmission,
    key: &str,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = ()> + Send,
{
    let state = tokio::sync::Mutex::new(admission);
    let queryable = session
        .declare_queryable(key)
        .await
        .map_err(|e| anyhow::anyhow!("declare OMAt queryable: {e}"))?;
    state.lock().await.connected(true);
    tokio::pin!(shutdown);
    loop {
        let query = tokio::select! {
            _ = &mut shutdown => break,
            result = queryable.recv_async() => match result {
                Ok(query) => query,
                Err(_) => break,
            },
        };
        let result = match query.payload() {
            Some(payload) => state
                .lock()
                .await
                .handle_for_topic(payload.to_bytes().as_ref(), key),
            None => Err(anyhow::anyhow!("missing payload")),
        };
        if result.is_err() {
            tracing::warn!(selector = %key, "omat_request_rejected");
            let _ = query.reply_err("rejected").await;
        } else {
            let _ = query.reply(key, b"accepted".as_slice()).await;
        }
    }
    state.lock().await.connected(false);
    Ok(())
}

/// Build an authenticated OMAs request. The key is provisioned out of band;
/// neither the tenant nor the principal in the frame grants authority.
pub fn sign_omas_request(
    principal: &str,
    topic: &str,
    nonce: &str,
    call: ToolCall,
    key: &[u8; 32],
) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(*key != [0; 32], "zero HMAC key");
    let canonical = serde_json::to_vec(&(principal, topic, nonce, &call))?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| anyhow::anyhow!("invalid key"))?;
    mac.update(&canonical);
    let digest = mac.finalize().into_bytes();
    let mac_hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    Ok(serde_json::to_vec(&SignedOmatRequest {
        principal: principal.to_owned(),
        topic: topic.to_owned(),
        nonce: nonce.to_owned(),
        call,
        mac_hex,
    })?)
}

/// OMAs sends a signed request to the configured OMAt queryable.
/// A rejected tool call is returned as an error, never as successful execution.
pub async fn request_omat_zenoh(
    session: &zenoh::Session,
    query_key: &str,
    payload: Vec<u8>,
) -> anyhow::Result<()> {
    let replies = session
        .get(query_key)
        .payload(payload)
        .timeout(std::time::Duration::from_secs(5))
        .await
        .map_err(|e| anyhow::anyhow!("OMAt Zenoh get: {e}"))?;
    let reply = tokio::time::timeout(std::time::Duration::from_secs(6), replies.recv_async())
        .await
        .map_err(|_| anyhow::anyhow!("OMAt response timeout"))?
        .map_err(|e| anyhow::anyhow!("OMAt response unavailable: {e}"))?;
    let sample = reply
        .result()
        .map_err(|_| anyhow::anyhow!("OMAt rejected request"))?;
    anyhow::ensure!(
        sample.payload().to_bytes().as_ref() == b"accepted",
        "unexpected OMAt response"
    );
    Ok(())
}

/// Retry only transport-unavailable responses. The same signed frame and nonce
/// are reused: if the server committed the nonce before a lost reply, the
/// retry is denied rather than executing a second time.
pub async fn request_omat_with_retry(
    session: &zenoh::Session,
    query_key: &str,
    payload: Vec<u8>,
    max_attempts: u8,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        (1..=5).contains(&max_attempts),
        "retry attempts must be 1..=5"
    );
    for attempt in 0..max_attempts {
        match request_omat_zenoh(session, query_key, payload.clone()).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                let message = error.to_string();
                let transient = message.contains("timeout")
                    || message.contains("unavailable")
                    || message.contains("OMAt Zenoh get");
                if !transient || attempt + 1 == max_attempts {
                    return Err(error);
                }
                let delay_ms = 100_u64 * (1_u64 << attempt);
                tracing::warn!(attempt = attempt + 1, delay_ms, "omat_transport_retry");
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
    unreachable!("max_attempts validated nonzero")
}
