//! Authenticated, tenant-bound discovery backed by durable OMAt SQLite state.
//! Every request reloads the registry, so signed revocation is immediately visible.
use crate::omat_server::{OmatPolicy, OmatRegistry, ToolDescriptor, ToolSchemas};
use crate::stage043_store::Stage043ToolStore;
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::Path;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRequest {
    pub principal: String,
    pub topic: String,
    pub nonce: String,
    pub operation: String,
    pub tool_id: Option<String>,
    pub manifest_sha256: Option<String>,
    pub mac_hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryResponse {
    pub descriptors: Option<Vec<ToolDescriptor>>,
    pub schemas: Option<ToolSchemas>,
}

pub struct DurableDiscovery {
    store: Stage043ToolStore,
    policy: OmatPolicy,
    principal: String,
    key: [u8; 32],
    replay: Connection,
}

impl DurableDiscovery {
    pub fn open(
        store: Stage043ToolStore,
        policy: OmatPolicy,
        principal: String,
        key: [u8; 32],
        replay_path: &Path,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !principal.is_empty() && key != [0; 32],
            "invalid discovery authority"
        );
        anyhow::ensure!(
            replay_path.is_absolute() && replay_path.is_file() && !replay_path.is_symlink(),
            "existing private replay DB required"
        );
        let replay = Connection::open_with_flags(
            replay_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        replay.busy_timeout(std::time::Duration::from_secs(5))?;
        replay.execute_batch(
            "CREATE TABLE IF NOT EXISTS stage043_discovery_nonce (
            principal TEXT NOT NULL, nonce TEXT NOT NULL,
            PRIMARY KEY(principal, nonce));",
        )?;
        Ok(Self {
            store,
            policy,
            principal,
            key,
            replay,
        })
    }

    pub fn handle(&mut self, bytes: &[u8], actual_topic: &str) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(bytes.len() <= 4096, "oversized discovery frame");
        let request: DiscoveryRequest = serde_json::from_slice(bytes)?;
        let expected = format!("ocaf/omat/{}/discovery", self.policy.tenant_id);
        anyhow::ensure!(
            actual_topic == expected
                && request.topic == expected
                && request.principal == self.principal,
            "discovery identity mismatch"
        );
        anyhow::ensure!(
            request.nonce.len() >= 16
                && request.nonce.len() <= 128
                && request.nonce.bytes().all(|b| b.is_ascii_alphanumeric()),
            "invalid nonce"
        );
        let canonical = serde_json::to_vec(&(
            &request.principal,
            &request.topic,
            &request.nonce,
            &request.operation,
            &request.tool_id,
            &request.manifest_sha256,
        ))?;
        let mut mac = HmacSha256::new_from_slice(&self.key)?;
        mac.update(&canonical);
        let signature = decode_hex(&request.mac_hex)?;
        mac.verify_slice(&signature)
            .map_err(|_| anyhow::anyhow!("bad discovery MAC"))?;
        anyhow::ensure!(
            self.replay.execute(
                "INSERT OR IGNORE INTO stage043_discovery_nonce(principal,nonce) VALUES (?1,?2)",
                params![request.principal, request.nonce]
            )? == 1,
            "discovery replay"
        );
        let mut registry = OmatRegistry::default();
        for manifest in self.store.list_active(&self.policy.tenant_id)? {
            registry.register(manifest)?;
        }
        let response = match request.operation.as_str() {
            "discover" if request.tool_id.is_none() && request.manifest_sha256.is_none() => {
                DiscoveryResponse {
                    descriptors: Some(registry.discover_for(&self.policy.tenant_id, &self.policy)?),
                    schemas: None,
                }
            }
            "schemas" => {
                let id = request
                    .tool_id
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("missing tool"))?;
                let digest = request
                    .manifest_sha256
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("missing digest"))?;
                DiscoveryResponse {
                    descriptors: None,
                    schemas: Some(registry.schemas_for(
                        &self.policy.tenant_id,
                        id,
                        &self.policy,
                        digest,
                    )?),
                }
            }
            _ => anyhow::bail!("invalid discovery operation"),
        };
        let payload = serde_json::to_vec(&response)?;
        anyhow::ensure!(payload.len() <= 256 * 1024, "discovery response too large");
        Ok(payload)
    }
}

fn decode_hex(value: &str) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(value.len() == 64, "invalid MAC length");
    let mut bytes = Vec::with_capacity(32);
    for pair in value.as_bytes().chunks_exact(2) {
        let s = std::str::from_utf8(pair)?;
        bytes.push(u8::from_str_radix(s, 16)?);
    }
    Ok(bytes)
}

pub fn sign_discovery(
    principal: &str,
    topic: &str,
    nonce: &str,
    operation: &str,
    tool_id: Option<String>,
    manifest_sha256: Option<String>,
    key: &[u8; 32],
) -> anyhow::Result<Vec<u8>> {
    let canonical = serde_json::to_vec(&(
        principal,
        topic,
        nonce,
        operation,
        &tool_id,
        &manifest_sha256,
    ))?;
    let mut mac = HmacSha256::new_from_slice(key)?;
    mac.update(&canonical);
    let mac_hex = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(serde_json::to_vec(&DiscoveryRequest {
        principal: principal.into(),
        topic: topic.into(),
        nonce: nonce.into(),
        operation: operation.into(),
        tool_id,
        manifest_sha256,
        mac_hex,
    })?)
}

pub async fn serve_discovery_until<F>(
    session: Arc<zenoh::Session>,
    mut state: DurableDiscovery,
    tenant: &str,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = ()> + Send,
{
    let selector = format!("ocaf/omat/{tenant}/discovery");
    let queryable = session
        .declare_queryable(&selector)
        .await
        .map_err(|e| anyhow::anyhow!("declare discovery queryable: {e}"))?;
    tokio::pin!(shutdown);
    loop {
        let query = tokio::select! {
            _=&mut shutdown=>break,
            result=queryable.recv_async()=>match result {Ok(q)=>q,Err(_)=>break},
        };
        let result = match query.payload() {
            Some(payload) => state.handle(payload.to_bytes().as_ref(), query.key_expr().as_str()),
            None => Err(anyhow::anyhow!("missing discovery payload")),
        };
        match result {
            Ok(payload) => {
                let _ = query.reply(query.key_expr().as_str(), payload).await;
            }
            Err(_) => {
                let _ = query.reply_err("rejected").await;
            }
        }
    }
    Ok(())
}

pub async fn request_discovery_zenoh(
    session: &zenoh::Session,
    topic: &str,
    frame: Vec<u8>,
) -> anyhow::Result<DiscoveryResponse> {
    let replies = session
        .get(topic)
        .payload(frame)
        .timeout(std::time::Duration::from_secs(5))
        .await
        .map_err(|e| anyhow::anyhow!("discovery get: {e}"))?;
    let reply = tokio::time::timeout(std::time::Duration::from_secs(6), replies.recv_async())
        .await?
        .map_err(|e| anyhow::anyhow!("discovery reply: {e}"))?;
    let sample = reply
        .result()
        .map_err(|_| anyhow::anyhow!("discovery denied"))?;
    Ok(serde_json::from_slice(
        sample.payload().to_bytes().as_ref(),
    )?)
}
