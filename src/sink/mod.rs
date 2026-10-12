//! Independent, authenticated durable effect receiver for a separate host.
//!
//! A signed controller activates a monotonic epoch with a *short* validity
//! window. Workers cannot invent effect jobs: a second controller-signed,
//! payload-bound permit is required. This sink is a concrete durable SQLite
//! journal, not a general-purpose HTTP/payment/filesystem adapter.
//!
//! The sink independently persists the highest observed generation, so a
//! stale worker fails after a successor activation. An isolated old leader
//! may still act until the short activation expires; there is no shared
//! consensus between this sink and PostgreSQL.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::rpc::{now_ms, validate_node_id, RpcError, SignedFrame, MAX_FRAME_BYTES};

pub const SINK_VERSION: u16 = 1;
const MAX_WIRE: usize = 100_000;
const MAX_EFFECT: usize = 8192;
const MAX_TTL_MS: u64 = 5_000;
const MAX_ACTIVATION_MS: u64 = 15_000;
const CLOCK_SKEW_MS: u64 = 2_000;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SinkError {
    #[error("unauthenticated sink request")]
    Unauthorized,
    #[error("invalid, oversized or expired sink request")]
    Invalid,
    #[error("signed request was already received")]
    Replay,
    #[error("sink journal unavailable")]
    Storage,
}

impl From<RpcError> for SinkError {
    fn from(_: RpcError) -> Self {
        Self::Unauthorized
    }
}

#[derive(Clone)]
pub struct SinkKeys {
    controller_id: String,
    controller_key: [u8; 32],
    worker_id: String,
    worker_key: [u8; 32],
}

impl SinkKeys {
    pub fn new(
        controller_id: &str,
        controller_key: [u8; 32],
        worker_id: &str,
        worker_key: [u8; 32],
    ) -> Result<Self, SinkError> {
        if validate_node_id(controller_id).is_err()
            || validate_node_id(worker_id).is_err()
            || controller_id == worker_id
            || controller_key == worker_key
            || controller_key == [0; 32]
            || worker_key == [0; 32]
        {
            return Err(SinkError::Invalid);
        }
        Ok(Self {
            controller_id: controller_id.to_owned(),
            controller_key,
            worker_id: worker_id.to_owned(),
            worker_key,
        })
    }

    fn for_sender(&self, sender: &str) -> Result<&[u8; 32], SinkError> {
        if sender == self.controller_id {
            Ok(&self.controller_key)
        } else if sender == self.worker_id {
            Ok(&self.worker_key)
        } else {
            Err(SinkError::Unauthorized)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SinkAction {
    /// Lease the receiver's epoch for at most 15 seconds. The caller must
    /// have checked its leadership with the authoritative coordinator.
    ActivateEpoch { epoch: i64, expires_at_ms: u64 },
    /// Commit a JSON event only if a controller-signed permit and current
    /// local activation both agree with this signed worker identity.
    Apply { permit: Vec<u8>, effect: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SinkRequest {
    pub version: u16,
    pub sink_id: String,
    pub sender: String,
    pub request_id: String,
    pub issued_at_ms: u64,
    pub ttl_ms: u64,
    pub action: SinkAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SinkWire {
    sender: String,
    frame_b64: String,
}

impl SinkRequest {
    pub fn new(
        sink_id: &str,
        sender: &str,
        issued_at_ms: u64,
        ttl_ms: u64,
        action: SinkAction,
    ) -> Result<Self, SinkError> {
        if validate_node_id(sink_id).is_err()
            || validate_node_id(sender).is_err()
            || !(1..=MAX_TTL_MS).contains(&ttl_ms)
        {
            return Err(SinkError::Invalid);
        }
        Ok(Self {
            version: SINK_VERSION,
            sink_id: sink_id.to_owned(),
            sender: sender.to_owned(),
            request_id: Uuid::new_v4().to_string(),
            issued_at_ms,
            ttl_ms,
            action,
        })
    }

    pub fn seal(&self, key: &[u8; 32]) -> Result<Vec<u8>, SinkError> {
        let frame = SignedFrame::seal(self, key).map_err(|_| SinkError::Invalid)?;
        serde_json::to_vec(&SinkWire {
            sender: self.sender.clone(),
            frame_b64: B64.encode(frame),
        })
        .map_err(|_| SinkError::Invalid)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SinkPermit {
    pub version: u16,
    pub sink_id: String,
    pub worker: String,
    pub job_id: String,
    pub epoch: i64,
    pub expires_at_ms: u64,
    pub payload_hash: String,
}

fn payload_hash(value: &Value) -> Result<String, SinkError> {
    let data = serde_json::to_vec(value).map_err(|_| SinkError::Invalid)?;
    if data.len() > MAX_EFFECT {
        return Err(SinkError::Invalid);
    }
    Ok(B64.encode(Sha256::digest(data)))
}

impl SinkPermit {
    pub fn new(
        sink_id: &str,
        worker: &str,
        job_id: &str,
        epoch: i64,
        expires_at_ms: u64,
        effect: &Value,
    ) -> Result<Self, SinkError> {
        if validate_node_id(sink_id).is_err()
            || validate_node_id(worker).is_err()
            || Uuid::parse_str(job_id).is_err()
            || epoch <= 0
            || expires_at_ms == 0
        {
            return Err(SinkError::Invalid);
        }
        Ok(Self {
            version: SINK_VERSION,
            sink_id: sink_id.to_owned(),
            worker: worker.to_owned(),
            job_id: job_id.to_owned(),
            epoch,
            expires_at_ms,
            payload_hash: payload_hash(effect)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkStatus {
    Ok,
    Denied,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SinkResponse {
    pub version: u16,
    pub sink_id: String,
    pub recipient: String,
    pub request_id: String,
    pub status: SinkStatus,
    pub output: Value,
}

impl SinkResponse {
    pub fn open(
        bytes: &[u8],
        key: &[u8; 32],
        request: &SinkRequest,
    ) -> Result<Self, SinkError> {
        let reply: Self = SignedFrame::open(bytes, key)?;
        if reply.version != SINK_VERSION
            || reply.sink_id != request.sink_id
            || reply.recipient != request.sender
            || reply.request_id != request.request_id
        {
            return Err(SinkError::Unauthorized);
        }
        Ok(reply)
    }
}

#[derive(Clone)]
pub struct EffectSink {
    sink_id: String,
    keys: SinkKeys,
    database: Arc<Mutex<Connection>>,
}

impl EffectSink {
    pub fn open(path: &Path, sink_id: &str, keys: SinkKeys) -> Result<Self, SinkError> {
        if validate_node_id(sink_id).is_err() || path.as_os_str().is_empty() {
            return Err(SinkError::Invalid);
        }
        #[cfg(unix)]
        {
            use std::fs::{OpenOptions, Permissions};
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if !meta.is_file() {
                    return Err(SinkError::Storage);
                }
            }
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o600)
                .open(path)
                .map_err(|_| SinkError::Storage)?;
            std::fs::set_permissions(path, Permissions::from_mode(0o600))
                .map_err(|_| SinkError::Storage)?;
        }
        let mut db = Connection::open(path).map_err(|_| SinkError::Storage)?;
        db.busy_timeout(Duration::from_secs(5))
            .map_err(|_| SinkError::Storage)?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS sink_identity_v1 (
                 one INTEGER PRIMARY KEY CHECK(one=1),
                 sink_id TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sink_epochs_v1 (
                 one INTEGER PRIMARY KEY CHECK(one=1),
                 epoch INTEGER NOT NULL CHECK(epoch>0),
                 expires_at_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sink_effects_v1 (
                 job_id TEXT PRIMARY KEY,
                 epoch INTEGER NOT NULL CHECK(epoch>0),
                 worker TEXT NOT NULL,
                 payload_hash TEXT NOT NULL,
                 effect TEXT NOT NULL,
                 committed_at_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sink_replay_v1 (
                 sender TEXT NOT NULL,
                 request_id TEXT NOT NULL,
                 expires_at_ms INTEGER NOT NULL,
                 PRIMARY KEY(sender,request_id)
             );
             CREATE INDEX IF NOT EXISTS sink_replay_expiry_v1
                 ON sink_replay_v1(expires_at_ms);",
        )
        .map_err(|_| SinkError::Storage)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SinkError::Storage)?;
        tx.execute(
            "INSERT OR IGNORE INTO sink_identity_v1(one,sink_id) VALUES(1,?1)",
            params![sink_id],
        )
        .map_err(|_| SinkError::Storage)?;
        let existing: String = tx
            .query_row("SELECT sink_id FROM sink_identity_v1 WHERE one=1", [], |row| row.get(0))
            .map_err(|_| SinkError::Storage)?;
        if existing != sink_id {
            return Err(SinkError::Invalid);
        }
        tx.commit().map_err(|_| SinkError::Storage)?;
        Ok(Self {
            sink_id: sink_id.to_owned(),
            keys,
            database: Arc::new(Mutex::new(db)),
        })
    }

    pub fn id(&self) -> &str {
        &self.sink_id
    }

    pub async fn handle(&self, wire: &[u8]) -> Result<Vec<u8>, SinkError> {
        self.handle_at(wire, now_ms().map_err(|_| SinkError::Storage)?)
            .await
    }

    pub async fn handle_at(&self, wire: &[u8], now: u64) -> Result<Vec<u8>, SinkError> {
        if wire.len() > MAX_WIRE {
            return Err(SinkError::Invalid);
        }
        let outer: SinkWire = serde_json::from_slice(wire).map_err(|_| SinkError::Invalid)?;
        let key = *self.keys.for_sender(&outer.sender)?;
        let frame = B64.decode(outer.frame_b64).map_err(|_| SinkError::Invalid)?;
        if frame.len() > MAX_FRAME_BYTES {
            return Err(SinkError::Invalid);
        }
        let req: SinkRequest = SignedFrame::open(&frame, &key)?;
        if req.version != SINK_VERSION
            || req.sink_id != self.sink_id
            || req.sender != outer.sender
            || Uuid::parse_str(&req.request_id).is_err()
            || !(1..=MAX_TTL_MS).contains(&req.ttl_ms)
            || req.issued_at_ms > now.saturating_add(CLOCK_SKEW_MS)
            || req
                .issued_at_ms
                .checked_add(req.ttl_ms)
                .is_none_or(|end| end < now)
        {
            return Err(SinkError::Invalid);
        }

        let sink = self.clone();
        let request_id = req.request_id.clone();
        let recipient = req.sender.clone();
        let (status, output) =
            tokio::task::spawn_blocking(move || sink.run_action(req, now))
                .await
                .map_err(|_| SinkError::Storage)??;

        let reply = SinkResponse {
            version: SINK_VERSION,
            sink_id: self.sink_id.clone(),
            recipient,
            request_id,
            status,
            output,
        };
        SignedFrame::seal(&reply, &key).map_err(|_| SinkError::Storage)
    }

    fn run_action(&self, req: SinkRequest, now: u64) -> Result<(SinkStatus, Value), SinkError> {
        let now = i64::try_from(now).map_err(|_| SinkError::Invalid)?;
        let expiry = req
            .issued_at_ms
            .checked_add(req.ttl_ms)
            .and_then(|ms| i64::try_from(ms).ok())
            .ok_or(SinkError::Invalid)?;
        let mut db = self.database.lock().map_err(|_| SinkError::Storage)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SinkError::Storage)?;
        tx.execute("DELETE FROM sink_replay_v1 WHERE expires_at_ms < ?1", params![now])
            .map_err(|_| SinkError::Storage)?;
        let inserted = tx
            .execute(
                "INSERT OR IGNORE INTO sink_replay_v1(sender,request_id,expires_at_ms)
                 VALUES(?1,?2,?3)",
                params![req.sender, req.request_id, expiry],
            )
            .map_err(|_| SinkError::Storage)?;
        if inserted != 1 {
            return Err(SinkError::Replay);
        }

        let result = match req.action {
            SinkAction::ActivateEpoch {
                epoch,
                expires_at_ms,
            } => {
                if req.sender != self.keys.controller_id
                    || epoch <= 0
                    || !(u64::try_from(now).map_err(|_| SinkError::Invalid)?
                        ..=u64::try_from(now).map_err(|_| SinkError::Invalid)?
                            .saturating_add(MAX_ACTIVATION_MS))
                        .contains(&expires_at_ms)
                    || expires_at_ms == u64::try_from(now).map_err(|_| SinkError::Invalid)?
                {
                    (SinkStatus::Denied, Value::Null)
                } else {
                    let previous: Option<i64> = tx
                        .query_row(
                            "SELECT epoch FROM sink_epochs_v1 WHERE one=1",
                            [],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(|_| SinkError::Storage)?;
                    if previous.is_some_and(|current| epoch < current) {
                        (SinkStatus::Denied, Value::Null)
                    } else {
                        tx.execute(
                            "INSERT INTO sink_epochs_v1(one,epoch,expires_at_ms)
                             VALUES(1,?1,?2)
                             ON CONFLICT(one) DO UPDATE SET
                               epoch=excluded.epoch,
                               expires_at_ms=excluded.expires_at_ms",
                            params![
                                epoch,
                                i64::try_from(expires_at_ms).map_err(|_| SinkError::Invalid)?
                            ],
                        )
                        .map_err(|_| SinkError::Storage)?;
                        (SinkStatus::Ok, json!({"epoch":epoch}))
                    }
                }
            }
            SinkAction::Apply { permit, effect } => {
                if req.sender != self.keys.worker_id || permit.len() > MAX_FRAME_BYTES {
                    (SinkStatus::Denied, Value::Null)
                } else {
                    self.apply(&tx, &req.sender, &permit, &effect, now)?
                }
            }
        };
        tx.commit().map_err(|_| SinkError::Storage)?;
        Ok(result)
    }

    fn apply(
        &self,
        tx: &rusqlite::Transaction<'_>,
        worker: &str,
        signed: &[u8],
        effect: &Value,
        now: i64,
    ) -> Result<(SinkStatus, Value), SinkError> {
        let permit: SinkPermit = match SignedFrame::open(signed, &self.keys.controller_key) {
            Ok(value) => value,
            Err(_) => return Ok((SinkStatus::Denied, Value::Null)),
        };
        let data = serde_json::to_string(effect).map_err(|_| SinkError::Invalid)?;
        if data.len() > MAX_EFFECT
            || permit.version != SINK_VERSION
            || permit.sink_id != self.sink_id
            || permit.worker != worker
            || Uuid::parse_str(&permit.job_id).is_err()
            || permit.epoch <= 0
            || permit.expires_at_ms <= u64::try_from(now).map_err(|_| SinkError::Invalid)?
            || permit.payload_hash != payload_hash(effect)?
        {
            return Ok((SinkStatus::Denied, Value::Null));
        }
        let active = tx
            .query_row(
                "SELECT epoch,expires_at_ms FROM sink_epochs_v1 WHERE one=1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(|_| SinkError::Storage)?;
        if active.is_none_or(|(epoch, expiry)| epoch != permit.epoch || expiry <= now) {
            return Ok((SinkStatus::Denied, Value::Null));
        }

        let wrote = tx
            .execute(
                "INSERT OR IGNORE INTO sink_effects_v1
                   (job_id,epoch,worker,payload_hash,effect,committed_at_ms)
                 VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    permit.job_id,
                    permit.epoch,
                    worker,
                    permit.payload_hash,
                    data,
                    now
                ],
            )
            .map_err(|_| SinkError::Storage)?;
        if wrote == 1 {
            return Ok((
                SinkStatus::Ok,
                json!({"job_id":permit.job_id, "applied":true,
                    "epoch":permit.epoch, "committed_at_ms":now}),
            ));
        }
        let existing: (String, String, i64, String, i64) = tx
            .query_row(
                "SELECT worker,payload_hash,epoch,effect,committed_at_ms
                 FROM sink_effects_v1 WHERE job_id=?1",
                params![permit.job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .map_err(|_| SinkError::Storage)?;
        if existing.0 != worker
            || existing.1 != permit.payload_hash
            || serde_json::from_str::<Value>(&existing.3)
                .map_err(|_| SinkError::Storage)?
                != *effect
        {
            return Ok((SinkStatus::Conflict, Value::Null));
        }
        Ok((
            SinkStatus::Ok,
            json!({"job_id":permit.job_id,"applied":false,
                "epoch":existing.2,"committed_at_ms":existing.4}),
        ))
    }
}
