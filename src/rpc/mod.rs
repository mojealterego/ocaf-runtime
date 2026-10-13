//! Authenticated, allowlisted RPC boundary for Zenoh query/reply.
//!
//! HMAC-SHA256 prevents forgery/tampering by peers without the matching key.
//! Frames are NOT encrypted. Use authenticated TLS and private networks for
//! sensitive deployment; this module deliberately exposes only free tools
//! without elevated permissions.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::cluster::directory::PgDirectory;
use crate::cluster::jobs::PgJobQueue;
use crate::cluster::leadership::PgLeader;
use crate::cluster::replay::PgReplay;
use crate::coordinator::{validate_capabilities, validate_tool_name, CoordinatorStore};
use crate::jobs::{JobError, JobSpec, JobStore, LeasedJob};
use crate::runtime::{OcafRuntime, RuntimeError, ToolCallRequest};

mod cert_identity;
mod directory_backend;
mod job_backend;
mod replay;
use cert_identity::PinnedCertificate;
use directory_backend::DirectoryBackend;
use job_backend::JobBackend;
use replay::ReplayBackend;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 65_536;
const MAX_BODY_BYTES: usize = 32_768;
const MAX_REQUEST_TTL_MS: u64 = 30_000;
const FUTURE_CLOCK_SKEW_MS: u64 = 5_000;
const RATE_WINDOW_MS: u64 = 1_000;
const DEFAULT_PEER_RATE_PER_SECOND: u32 = 64;
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RpcError {
    #[error("RPC request could not be authenticated")]
    Unauthorized,
    #[error("request was already processed")]
    Replay,
    #[error("request is expired or outside accepted clock window")]
    Expired,
    #[error("request intended for a different recipient")]
    WrongRecipient,
    #[error("invalid or malformed RPC message")]
    Malformed,
    #[error("RPC frame exceeds allowed size")]
    FrameTooLarge,
    #[error("invalid signed RPC response")]
    InvalidReply,
    #[error("RPC gateway is busy")]
    Busy,
    #[error("RPC configuration or runtime unavailable")]
    Unavailable,
}

pub fn validate_node_id(id: &str) -> Result<(), RpcError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(RpcError::Malformed);
    }
    Ok(())
}

pub fn now_ms() -> Result<u64, RpcError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RpcError::Unavailable)?;
    u64::try_from(elapsed.as_millis()).map_err(|_| RpcError::Unavailable)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RpcAction {
    Discover,
    Execute {
        tool: String,
        args: Value,
    },
    Heartbeat {
        tools: Vec<String>,
    },
    Resolve {
        tool: String,
    },
    SubmitJob {
        id: String,
        tool: String,
        args: Value,
    },
    GetJob {
        id: String,
    },
    /// Owner-private, metadata-only PostgreSQL job history.
    GetJobAudit {
        id: String,
        limit: usize,
    },
    ClaimJob,
    ValidateLease {
        lease: LeasedJob,
    },
    /// Opt-in PostgreSQL-local effect sink; no generic external tool call.
    ApplyFencedEffect {
        lease: LeasedJob,
        effect: Value,
    },
    FinishJob {
        lease: LeasedJob,
        outcome: RemoteJobOutcome,
    },
}

/// A remote worker is trusted to report whether an execution was completed,
/// definitively rejected before execution, or ambiguous.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteJobOutcome {
    Succeeded { output: Value },
    Failed,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcRequest {
    pub version: u16,
    pub request_id: String,
    pub sender: String,
    pub recipient: String,
    pub issued_at_ms: u64,
    pub ttl_ms: u64,
    pub action: RpcAction,
    /// Optional certificate-private-key proof over the exact immutable request
    /// UUID, timestamp, recipient and action. Never trust an unsigned claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_proof_b64: Option<String>,
}

impl RpcRequest {
    pub fn new(
        sender: &str,
        recipient: &str,
        issued_at_ms: u64,
        ttl_ms: u64,
        action: RpcAction,
    ) -> Result<Self, RpcError> {
        validate_node_id(sender)?;
        validate_node_id(recipient)?;
        if ttl_ms == 0 || ttl_ms > MAX_REQUEST_TTL_MS {
            return Err(RpcError::Malformed);
        }
        match &action {
            RpcAction::Execute { tool, .. }
            | RpcAction::Resolve { tool }
            | RpcAction::SubmitJob { tool, .. } => {
                if !validate_tool_name(tool) {
                    return Err(RpcError::Malformed);
                }
            }
            RpcAction::Heartbeat { tools } => {
                validate_capabilities(tools).map_err(|_| RpcError::Malformed)?;
            }
            RpcAction::GetJob { .. }
            | RpcAction::GetJobAudit { .. }
            | RpcAction::ClaimJob
            | RpcAction::ValidateLease { .. }
            | RpcAction::ApplyFencedEffect { .. }
            | RpcAction::FinishJob { .. }
            | RpcAction::Discover => {}
        }
        if let RpcAction::SubmitJob { id, .. }
        | RpcAction::GetJob { id }
        | RpcAction::GetJobAudit { id, .. } = &action
        {
            if Uuid::parse_str(id).is_err() {
                return Err(RpcError::Malformed);
            }
        }
        if let RpcAction::GetJobAudit { limit, .. } = &action {
            if !(1..=100).contains(limit) {
                return Err(RpcError::Malformed);
            }
        }
        Ok(Self {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4().to_string(),
            sender: sender.to_owned(),
            recipient: recipient.to_owned(),
            issued_at_ms,
            ttl_ms,
            action,
            certificate_proof_b64: None,
        })
    }

    fn check(&self, recipient: &str, now_ms: u64) -> Result<(), RpcError> {
        if self.version != PROTOCOL_VERSION
            || validate_node_id(&self.sender).is_err()
            || validate_node_id(&self.recipient).is_err()
            || Uuid::parse_str(&self.request_id).is_err()
            || self.ttl_ms == 0
            || self.ttl_ms > MAX_REQUEST_TTL_MS
        {
            return Err(RpcError::Malformed);
        }
        if let RpcAction::SubmitJob { id, tool, .. } = &self.action {
            if Uuid::parse_str(id).is_err() || !validate_tool_name(tool) {
                return Err(RpcError::Malformed);
            }
        }
        if let RpcAction::GetJob { id } | RpcAction::GetJobAudit { id, .. } = &self.action {
            if Uuid::parse_str(id).is_err() {
                return Err(RpcError::Malformed);
            }
        }
        if let RpcAction::GetJobAudit { limit, .. } = &self.action {
            if !(1..=100).contains(limit) {
                return Err(RpcError::Malformed);
            }
        }
        if self.recipient != recipient {
            return Err(RpcError::WrongRecipient);
        }
        if self.issued_at_ms > now_ms.saturating_add(FUTURE_CLOCK_SKEW_MS)
            || self
                .issued_at_ms
                .checked_add(self.ttl_ms)
                .is_none_or(|expiry| expiry < now_ms)
        {
            return Err(RpcError::Expired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcStatus {
    Ok,
    InvalidInput,
    Timeout,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcReply {
    pub version: u16,
    pub request_id: String,
    pub sender: String,
    pub recipient: String,
    pub status: RpcStatus,
    pub output: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireFrame {
    body: String,
    tag: String,
}

pub struct SignedFrame;

impl SignedFrame {
    pub fn seal<T: Serialize>(message: &T, key: &[u8; 32]) -> Result<Vec<u8>, RpcError> {
        let body = serde_json::to_vec(message).map_err(|_| RpcError::Malformed)?;
        if body.len() > MAX_BODY_BYTES {
            return Err(RpcError::FrameTooLarge);
        }
        let mut mac =
            <HmacSha256 as Mac>::new_from_slice(key).map_err(|_| RpcError::Unavailable)?;
        mac.update(&body);
        let frame = WireFrame {
            body: B64.encode(body),
            tag: B64.encode(mac.finalize().into_bytes()),
        };
        let bytes = serde_json::to_vec(&frame).map_err(|_| RpcError::Malformed)?;
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(RpcError::FrameTooLarge);
        }
        Ok(bytes)
    }

    fn unpack(bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>), RpcError> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(RpcError::FrameTooLarge);
        }
        let frame: WireFrame = serde_json::from_slice(bytes).map_err(|_| RpcError::Malformed)?;
        if frame.body.len() > (MAX_BODY_BYTES * 4 / 3 + 8) || frame.tag.len() != 44 {
            return Err(RpcError::FrameTooLarge);
        }
        let body = B64.decode(frame.body).map_err(|_| RpcError::Malformed)?;
        if body.len() > MAX_BODY_BYTES {
            return Err(RpcError::FrameTooLarge);
        }
        let tag = B64.decode(frame.tag).map_err(|_| RpcError::Malformed)?;
        if tag.len() != 32 {
            return Err(RpcError::Malformed);
        }
        Ok((body, tag))
    }

    pub fn open<T: DeserializeOwned>(bytes: &[u8], key: &[u8; 32]) -> Result<T, RpcError> {
        let (body, tag) = Self::unpack(bytes)?;
        let mut mac =
            <HmacSha256 as Mac>::new_from_slice(key).map_err(|_| RpcError::Unavailable)?;
        mac.update(&body);
        mac.verify_slice(&tag).map_err(|_| RpcError::Unauthorized)?;
        serde_json::from_slice(&body).map_err(|_| RpcError::Malformed)
    }
}

#[derive(Clone)]
struct KeyRotation {
    next_key: [u8; 32],
    activate_at_ms: u64,
    retire_old_at_ms: u64,
}

#[derive(Clone)]
pub struct PeerPolicy {
    key: [u8; 32],
    rotation: Option<KeyRotation>,
    certificate_identity: Option<PinnedCertificate>,
    allowed_tools: HashSet<String>,
    requests_per_second: u32,
    job_submitter: bool,
    job_worker: bool,
    effect_writer: bool,
}

impl PeerPolicy {
    pub fn new(key: [u8; 32], allowed_tools: Vec<String>) -> Self {
        Self {
            key,
            rotation: None,
            certificate_identity: None,
            allowed_tools: allowed_tools.into_iter().collect(),
            requests_per_second: DEFAULT_PEER_RATE_PER_SECOND,
            job_submitter: false,
            job_worker: false,
            effect_writer: false,
        }
    }

    /// Require a signature with a private key corresponding to a pre-pinned
    /// X.509 leaf certificate, in addition to HMAC. The caller MUST
    /// independently verify the trust chain, EKU/SAN and revocation of this
    /// leaf using the PKI gate. This is NOT a TLS exporter/channel binding.
    pub fn with_pinned_certificate(
        mut self,
        cert_pem: &[u8],
        expected_sha256: &str,
    ) -> Result<Self, RpcError> {
        self.certificate_identity = Some(PinnedCertificate::from_pem(cert_pem, expected_sha256)?);
        Ok(self)
    }

    /// Stage a strictly bounded HMAC secret rollover for this exact RPC
    /// sender. Old requests remain valid until retire_old_at_ms, while new
    /// requests become valid at activate_at_ms. Replies are signed with the
    /// *same* key that authenticated the request. This does not by itself
    /// bind HMAC identity to the certificate used by Zenoh TLS.
    pub fn with_key_rotation(
        mut self,
        next_key: [u8; 32],
        activate_at_ms: u64,
        retire_old_at_ms: u64,
    ) -> Result<Self, RpcError> {
        const MAX_OVERLAP_MS: u64 = 30 * 60 * 1000;
        if self.key == [0; 32]
            || next_key == [0; 32]
            || next_key == self.key
            || activate_at_ms == 0
            || retire_old_at_ms <= activate_at_ms
            || retire_old_at_ms - activate_at_ms > MAX_OVERLAP_MS
        {
            return Err(RpcError::Malformed);
        }
        self.rotation = Some(KeyRotation {
            next_key,
            activate_at_ms,
            retire_old_at_ms,
        });
        Ok(self)
    }

    /// Verify the request using precisely the keys allowed by the cutover
    /// schedule. No retired key may authenticate or sign a response.
    fn authenticate<'a>(
        &'a self,
        bytes: &[u8],
        now: u64,
    ) -> Result<(RpcRequest, &'a [u8; 32]), RpcError> {
        let Some(rotation) = &self.rotation else {
            return SignedFrame::open(bytes, &self.key).map(|request| (request, &self.key));
        };
        if now < rotation.activate_at_ms {
            return SignedFrame::open(bytes, &self.key).map(|request| (request, &self.key));
        }
        if now >= rotation.retire_old_at_ms {
            return SignedFrame::open(bytes, &rotation.next_key)
                .map(|request| (request, &rotation.next_key));
        }
        // In overlap, both keys are valid; no fallback outside this window.
        if let Ok(request) = SignedFrame::open(bytes, &rotation.next_key) {
            return Ok((request, &rotation.next_key));
        }
        SignedFrame::open(bytes, &self.key).map(|request| (request, &self.key))
    }

    /// Limit the number of authenticated RPC messages in each one-second
    /// fixed window. Limits are in-memory per gateway instance (not global
    /// across multiple hosts).
    pub fn with_rate_limit(mut self, requests_per_second: u32) -> Result<Self, RpcError> {
        if requests_per_second == 0 || requests_per_second > 10_000 {
            return Err(RpcError::Malformed);
        }
        self.requests_per_second = requests_per_second;
        Ok(self)
    }

    /// Explicitly authorize this peer to submit free, single-attempt work.
    pub fn allow_job_submission(mut self) -> Self {
        self.job_submitter = true;
        self
    }

    /// Explicitly authorize this peer to claim and report queued work.
    pub fn allow_job_worker(mut self) -> Self {
        self.job_worker = true;
        self
    }

    /// Grants the explicitly reviewed right to commit a transactional
    /// PostgreSQL-local effect. Job-worker capability alone is not enough.
    /// This must be combined with allow_job_worker() and a tool ACL.
    pub fn allow_fenced_effect_writes(mut self) -> Self {
        self.effect_writer = true;
        self
    }

    fn permits(&self, tool: &str) -> bool {
        self.allowed_tools.contains(tool)
    }
}

pub struct RpcGateway {
    node_id: String,
    runtime: OcafRuntime,
    peers: HashMap<String, PeerPolicy>,
    seen: ReplayBackend,
    rates: Mutex<HashMap<String, (u64, u32)>>,
    concurrency: Semaphore,
    coordinator: Option<DirectoryBackend>,
    jobs: Option<JobBackend>,
    leader: Option<Arc<PgLeader>>,
}

impl RpcGateway {
    pub fn new(
        node_id: &str,
        runtime: OcafRuntime,
        peers: HashMap<String, PeerPolicy>,
        max_concurrent_calls: usize,
    ) -> Result<Self, RpcError> {
        validate_node_id(node_id)?;
        if max_concurrent_calls == 0 || max_concurrent_calls > 10_000 {
            return Err(RpcError::Malformed);
        }
        for (peer_id, peer) in &peers {
            validate_node_id(peer_id)?;
            if peer.key == [0; 32]
                || peer.rotation.as_ref().is_some_and(|rotation| {
                    rotation.next_key == [0; 32]
                        || rotation.next_key == peer.key
                        || rotation.activate_at_ms == 0
                        || rotation.retire_old_at_ms <= rotation.activate_at_ms
                        || rotation.retire_old_at_ms - rotation.activate_at_ms > 1_800_000
                })
                || peer.allowed_tools.iter().any(|tool| tool.is_empty())
                || (peer.effect_writer && !peer.job_worker)
            {
                return Err(RpcError::Malformed);
            }
        }
        Ok(Self {
            node_id: node_id.into(),
            runtime,
            peers,
            seen: ReplayBackend::volatile(),
            rates: Mutex::new(HashMap::new()),
            concurrency: Semaphore::new(max_concurrent_calls),
            coordinator: None,
            jobs: None,
            leader: None,
        })
    }

    /// Persist signed request admission to an on-disk SQLite ledger before
    /// invoking any tool. A storage failure rejects requests rather than
    /// silently falling back to an in-memory replay cache.
    ///
    /// This enforces at-most-once admission within request validity windows,
    /// not exactly-once completion of arbitrary external side effects.
    pub fn with_sqlite_replay(
        node_id: &str,
        runtime: OcafRuntime,
        peers: HashMap<String, PeerPolicy>,
        max_concurrent_calls: usize,
        db_path: &Path,
    ) -> Result<Self, RpcError> {
        let mut gateway = Self::new(node_id, runtime, peers, max_concurrent_calls)?;
        gateway.seen = ReplayBackend::sqlite(db_path)?;
        Ok(gateway)
    }

    /// Attach a persistent membership directory. This is a single-host
    /// coordination component, not a replicated distributed scheduler.
    pub fn with_coordinator_db(mut self, db_path: &Path) -> Result<Self, RpcError> {
        self.coordinator = Some(DirectoryBackend::Sqlite(
            CoordinatorStore::open(db_path).map_err(|_| RpcError::Unavailable)?,
        ));
        Ok(self)
    }

    /// Use a shared PostgreSQL replay journal instead of local cache.
    pub async fn with_postgres_replay(
        mut self,
        dsn: &str,
        allow_insecure_loopback: bool,
    ) -> Result<Self, RpcError> {
        self.seen = ReplayBackend::postgres(PgReplay::connect(dsn, allow_insecure_loopback).await?);
        Ok(self)
    }

    /// Use a shared PostgreSQL capability directory.
    pub async fn with_postgres_coordinator(
        mut self,
        dsn: &str,
        allow_insecure_loopback: bool,
    ) -> Result<Self, RpcError> {
        self.coordinator = Some(DirectoryBackend::Postgres(
            PgDirectory::connect(dsn, allow_insecure_loopback)
                .await
                .map_err(|_| RpcError::Unavailable)?,
        ));
        Ok(self)
    }

    /// Opt in to a durable, single-coordinator SQLite queue.
    pub fn with_job_db(mut self, path: &Path) -> Result<Self, RpcError> {
        if self.leader.is_some() {
            return Err(RpcError::Malformed);
        }
        self.jobs = Some(JobBackend::Sqlite(
            JobStore::open(path).map_err(|_| RpcError::Unavailable)?,
        ));
        Ok(self)
    }

    /// Shared PostgreSQL queue. Transactions are serialized by the database
    /// across independent coordinator hosts. This alone does not replicate
    /// the replay ledger or discovery directory; full cluster failover
    /// requires those components and distributed leadership to be wired too.
    pub async fn with_postgres_job_db(
        mut self,
        dsn: &str,
        allow_insecure_loopback: bool,
    ) -> Result<Self, RpcError> {
        let mut queue = PgJobQueue::connect(dsn, allow_insecure_loopback)
            .await
            .map_err(|_| RpcError::Unavailable)?;
        if let Some(leader) = &self.leader {
            queue = queue.with_leader(Arc::clone(leader));
        }
        self.jobs = Some(JobBackend::Postgres(queue));
        Ok(self)
    }

    /// Only the current PostgreSQL advisory leader is allowed to process
    /// signed RPC. Called by cluster coordinator with a dedicated leader
    /// session, not by the legacy single-host SQLite service.
    pub fn with_pg_leader(mut self, leader: Arc<PgLeader>) -> Self {
        if let Some(jobs) = self.jobs.take() {
            self.jobs = Some(jobs.with_leader(Arc::clone(&leader)));
        }
        if let Some(directory) = self.coordinator.take() {
            self.coordinator = Some(directory.with_leader(Arc::clone(&leader)));
        }
        self.seen.bind_leader(Arc::clone(&leader));
        self.leader = Some(leader);
        self
    }

    pub fn runtime(&self) -> &OcafRuntime {
        &self.runtime
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub async fn handle(&self, message: &[u8]) -> Result<Vec<u8>, RpcError> {
        self.handle_at(message, now_ms()?).await
    }

    /// A late-stage leadership check is also required immediately before
    /// emitting a signed reply. This cannot prevent an already started
    /// external side effect; it only avoids confirming work from a stale
    /// coordinator after detectable leader loss.
    pub async fn verify_current_leader(&self) -> Result<(), RpcError> {
        if let Some(leader) = &self.leader {
            if self
                .jobs
                .as_ref()
                .is_some_and(|backend| !backend.is_epoch_fenced())
            {
                return Err(RpcError::Unavailable);
            }
            leader.verify().await.map_err(|_| RpcError::Unavailable)?;
        }
        Ok(())
    }

    pub async fn handle_at(&self, message: &[u8], now: u64) -> Result<Vec<u8>, RpcError> {
        self.verify_current_leader().await?;
        // Parse only the caller identity from untrusted data before HMAC verification.
        // Every field remains untrusted until the peer-specific MAC is verified.
        let (raw, _) = SignedFrame::unpack(message)?;
        let unverified: RpcRequest =
            serde_json::from_slice(&raw).map_err(|_| RpcError::Malformed)?;
        let peer = self
            .peers
            .get(&unverified.sender)
            .ok_or(RpcError::Unauthorized)?;
        let (request, verified_key) = peer.authenticate(message, now)?;
        request.check(&self.node_id, now)?;
        // Certificate PoP is enforced before any replay admission, rate
        // allocation, tool lookup, or external side effect.
        if let Some(bound_certificate) = &peer.certificate_identity {
            bound_certificate.verify(&request)?;
        } else if request.certificate_proof_b64.is_some() {
            // A proof presented to an unbound peer policy is not trusted
            // evidence of identity. Reject to prevent policy confusion.
            return Err(RpcError::Unauthorized);
        }

        // Count authenticated messages before manifest lookup or discovery,
        // so a compromised but authenticated peer cannot enumerate resources
        // without an independent per-peer quota.
        {
            let mut rates = self.rates.lock().map_err(|_| RpcError::Unavailable)?;
            let window = rates.entry(request.sender.clone()).or_insert((now, 0));
            if now < window.0 || now.saturating_sub(window.0) >= RATE_WINDOW_MS {
                *window = (now, 0);
            }
            if window.1 >= peer.requests_per_second {
                return Err(RpcError::Busy);
            }
            window.1 += 1;
        }

        match &request.action {
            RpcAction::Execute { tool, .. } => {
                // A free, manifest-permissionless tool is not necessarily
                // side-effect-free. In cluster mode the coordinator must
                // never start an unleased, unfenced runtime execution.
                if self.leader.is_some() {
                    return Err(RpcError::Unauthorized);
                }
                if !peer.permits(tool) {
                    return Err(RpcError::Unauthorized);
                }
                let manifest = self
                    .runtime
                    .registry
                    .resolve(tool)
                    .await
                    .ok_or(RpcError::Unauthorized)?;
                if !manifest.manifest.enabled
                    || manifest.manifest.cost_sats != 0
                    || !manifest.manifest.permissions.is_empty()
                {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::Heartbeat { tools } => {
                if self.coordinator.is_none() {
                    return Err(RpcError::Unavailable);
                }
                validate_capabilities(tools).map_err(|_| RpcError::Malformed)?;
                if tools.iter().any(|tool| !peer.permits(tool)) {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::Resolve { tool } => {
                if self.coordinator.is_none() {
                    return Err(RpcError::Unavailable);
                }
                if !validate_tool_name(tool) {
                    return Err(RpcError::Malformed);
                }
                if !peer.permits(tool) {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::SubmitJob { id, tool, .. } => {
                if self.jobs.is_none() {
                    return Err(RpcError::Unavailable);
                }
                if !peer.job_submitter || !peer.permits(tool) || Uuid::parse_str(id).is_err() {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::GetJob { .. } | RpcAction::GetJobAudit { .. } => {
                if self.jobs.is_none() {
                    return Err(RpcError::Unavailable);
                }
                if !peer.job_submitter {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::ClaimJob => {
                if self.jobs.is_none() {
                    return Err(RpcError::Unavailable);
                }
                if !peer.job_worker || peer.allowed_tools.is_empty() {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::ValidateLease { lease } | RpcAction::FinishJob { lease, .. } => {
                if self.jobs.is_none() {
                    return Err(RpcError::Unavailable);
                }
                if !peer.job_worker
                    || lease.worker_id != request.sender
                    || !peer.permits(&lease.tool)
                {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::ApplyFencedEffect { lease, .. } => {
                if self.jobs.is_none() {
                    return Err(RpcError::Unavailable);
                }
                if !peer.job_worker
                    || !peer.effect_writer
                    || lease.worker_id != request.sender
                    || !peer.permits(&lease.tool)
                {
                    return Err(RpcError::Unauthorized);
                }
            }
            RpcAction::Discover => {}
        }

        let _permit = self.concurrency.try_acquire().map_err(|_| RpcError::Busy)?;

        let expires = request
            .issued_at_ms
            .saturating_add(request.ttl_ms)
            .saturating_add(FUTURE_CLOCK_SKEW_MS);
        self.seen
            .reserve(
                &self.node_id,
                &request.sender,
                &request.request_id,
                expires,
                now,
            )
            .await?;

        let (status, output) = match request.action {
            RpcAction::Discover => {
                let all = self.runtime.registry.list().await;
                let public: Vec<_> = all
                    .into_iter()
                    .filter(|tool| {
                        tool.enabled
                            && tool.cost_sats == 0
                            && tool.permissions.is_empty()
                            && peer.permits(&tool.name)
                    })
                    .collect();
                (RpcStatus::Ok, json!(public))
            }
            RpcAction::Heartbeat { tools } => {
                let result = self
                    .coordinator
                    .as_ref()
                    .ok_or(RpcError::Unavailable)?
                    .heartbeat(&request.sender, &tools, now)
                    .await
                    .map_err(|_| RpcError::Unavailable)?;
                (RpcStatus::Ok, json!(result))
            }
            RpcAction::Resolve { tool } => {
                let result = self
                    .coordinator
                    .as_ref()
                    .ok_or(RpcError::Unavailable)?
                    .resolve(&tool, now)
                    .await
                    .map_err(|_| RpcError::Unavailable)?;
                (RpcStatus::Ok, json!(result))
            }
            RpcAction::SubmitJob { id, tool, args } => {
                let spec = JobSpec {
                    id,
                    tool,
                    args,
                    retry_safe: false,
                    max_attempts: 1,
                };
                let result = self
                    .jobs
                    .as_ref()
                    .ok_or(RpcError::Unavailable)?
                    .submit_owned(&request.sender, spec)
                    .await;
                match result {
                    Ok(job) => (RpcStatus::Ok, json!({"job_id":job.id})),
                    Err(JobError::Storage) => return Err(RpcError::Unavailable),
                    Err(_) => (RpcStatus::Failed, Value::Null),
                }
            }
            RpcAction::GetJob { id } => {
                let record = self
                    .jobs
                    .as_ref()
                    .ok_or(RpcError::Unavailable)?
                    .get_owned(&request.sender, &id)
                    .await
                    .map_err(|_| RpcError::Unavailable)?;
                (RpcStatus::Ok, json!(record))
            }
            RpcAction::GetJobAudit { id, limit } => {
                let history = self
                    .jobs
                    .as_ref()
                    .ok_or(RpcError::Unavailable)?
                    .audit_owned(&request.sender, &id, limit)
                    .await
                    .map_err(|_| RpcError::Unavailable)?;
                (RpcStatus::Ok, json!(history))
            }
            RpcAction::ClaimJob => {
                let store = self.jobs.as_ref().ok_or(RpcError::Unavailable)?;
                store
                    .recover_expired(now)
                    .await
                    .map_err(|_| RpcError::Unavailable)?;
                let tools: Vec<String> = peer.allowed_tools.iter().cloned().collect();
                let claim = store
                    .claim_permitted(&request.sender, now, 30_000, &tools)
                    .await
                    .map_err(|_| RpcError::Unavailable)?;
                (RpcStatus::Ok, json!(claim))
            }
            RpcAction::ValidateLease { lease } => {
                let store = self.jobs.as_ref().ok_or(RpcError::Unavailable)?;
                let valid = store
                    .validate_current_lease(&lease)
                    .await
                    .map_err(|_| RpcError::Unavailable)?;
                (
                    if valid {
                        RpcStatus::Ok
                    } else {
                        RpcStatus::Failed
                    },
                    json!({"valid": valid}),
                )
            }
            RpcAction::ApplyFencedEffect { lease, effect } => {
                if self.leader.is_none() {
                    return Err(RpcError::Unavailable);
                }
                let store = self.jobs.as_ref().ok_or(RpcError::Unavailable)?;
                match store.apply_fenced_effect(&lease, effect).await {
                    Ok(receipt) => (RpcStatus::Ok, json!(receipt)),
                    Err(JobError::Storage) => return Err(RpcError::Unavailable),
                    Err(_) => (RpcStatus::Failed, Value::Null),
                }
            }
            RpcAction::FinishJob { lease, outcome } => {
                let store = self.jobs.as_ref().ok_or(RpcError::Unavailable)?;
                let result = match outcome {
                    RemoteJobOutcome::Succeeded { output } => {
                        store.succeed(&lease, output, now).await
                    }
                    RemoteJobOutcome::Failed => {
                        store.fail(&lease, "worker rejected or failed", now).await
                    }
                    RemoteJobOutcome::Uncertain => store.mark_uncertain(&lease, now).await,
                };
                match result {
                    Ok(()) => (RpcStatus::Ok, Value::Null),
                    Err(JobError::Storage) => return Err(RpcError::Unavailable),
                    Err(_) => (RpcStatus::Failed, Value::Null),
                }
            }
            RpcAction::Execute { tool, args } => {
                // No AuthContext is accepted over RPC. Remote privileged/paid tools
                // are always rejected above; local policy may be stronger.
                let call = ToolCallRequest {
                    call_id: request.request_id.clone(),
                    tool,
                    args,
                    auth: None,
                };
                match self.runtime.execute(call).await {
                    Ok(result) => (RpcStatus::Ok, result.output),
                    Err(RuntimeError::InvalidInput(_)) => (RpcStatus::InvalidInput, Value::Null),
                    Err(RuntimeError::Timeout) => (RpcStatus::Timeout, Value::Null),
                    Err(_) => (RpcStatus::Failed, Value::Null),
                }
            }
        };
        let reply = RpcReply {
            version: PROTOCOL_VERSION,
            request_id: request.request_id,
            sender: self.node_id.clone(),
            recipient: request.sender,
            status,
            output,
        };
        SignedFrame::seal(&reply, verified_key)
    }
}

#[derive(Clone)]
pub struct RpcClient {
    node_id: String,
    remote_id: String,
    key: [u8; 32],
    certificate_private_key_pem: Option<Vec<u8>>,
}

impl RpcClient {
    pub fn new(node_id: &str, remote_id: &str, key: [u8; 32]) -> Result<Self, RpcError> {
        validate_node_id(node_id)?;
        validate_node_id(remote_id)?;
        if key == [0; 32] {
            return Err(RpcError::Malformed);
        }
        Ok(Self {
            node_id: node_id.into(),
            remote_id: remote_id.into(),
            key,
            certificate_private_key_pem: None,
        })
    }

    /// Opt-in certificate proof for every newly prepared RPC request. The
    /// key must be loaded from an owner-only, protected file. Client clones
    /// hold independent copies of sensitive bytes; keep their lifetime short.
    pub fn with_certificate_private_key(mut self, key_pem: Vec<u8>) -> Result<Self, RpcError> {
        if key_pem.is_empty() || key_pem.len() > 16 * 1024 {
            return Err(RpcError::Malformed);
        }
        self.certificate_private_key_pem = Some(key_pem);
        Ok(self)
    }

    pub fn prepare(
        &self,
        action: RpcAction,
        ttl_ms: u64,
    ) -> Result<(RpcRequest, Vec<u8>), RpcError> {
        let mut req = RpcRequest::new(&self.node_id, &self.remote_id, now_ms()?, ttl_ms, action)?;
        if let Some(private_key) = &self.certificate_private_key_pem {
            req = req.with_certificate_proof(private_key)?;
        }
        let encoded = SignedFrame::seal(&req, &self.key)?;
        Ok((req, encoded))
    }

    pub fn read_reply(&self, bytes: &[u8], request: &RpcRequest) -> Result<RpcReply, RpcError> {
        let reply: RpcReply = SignedFrame::open(bytes, &self.key)?;
        if reply.version != PROTOCOL_VERSION
            || reply.request_id != request.request_id
            || reply.sender != self.remote_id
            || reply.recipient != self.node_id
        {
            return Err(RpcError::InvalidReply);
        }
        Ok(reply)
    }

    pub fn local_id(&self) -> &str {
        &self.node_id
    }

    pub fn remote_id(&self) -> &str {
        &self.remote_id
    }
}

/// Safe shared gateway for use by an async queryable handler.
pub type SharedRpcGateway = Arc<RpcGateway>;
