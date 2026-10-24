//! Authenticated Zenoh ingress for OMAt. Zenoh transport encryption is an
//! operator configuration responsibility; message authentication is mandatory.
use crate::omat_server::{OmatPolicy, OmatRegistry, ToolCall};
use crate::stage043_omat_route::{dispatch_tool_call, AuthenticatedPeer, OmatRouteGuard};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;
const MAX_FRAME: usize = 256 * 1024;
const MAX_REPLAY: usize = 100_000;

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
    keys: HashMap<String, [u8; 32]>,
    guard: OmatRouteGuard,
    used: HashSet<(String, String)>,
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
        anyhow::ensure!(!keys.is_empty() && keys.len() == tenants.len(), "invalid peer keys");
        anyhow::ensure!(keys.keys().all(|k| tenants.contains_key(k)), "missing tenant binding");
        anyhow::ensure!(keys.values().all(|k| *k != [0; 32]), "zero HMAC key");
        let guard = OmatRouteGuard::new(tenants).map_err(|_| anyhow::anyhow!("invalid tenant mapping"))?;
        Ok(Self { keys, guard, used: HashSet::new(), registry, policies })
    }

    /// A session must be established before accepting requests. Never use a
    /// caller-provided tenant as the source of authorization authority.
    pub fn connected(&mut self, connected: bool) {
        self.guard.set_transport_connected(connected);
    }

    pub fn handle(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        anyhow::ensure!(bytes.len() <= MAX_FRAME, "frame too large");
        let request: SignedOmatRequest = serde_json::from_slice(bytes)?;
        anyhow::ensure!(request.nonce.len() >= 16 && request.nonce.len() <= 128 && request.nonce.bytes().all(|b| b.is_ascii_alphanumeric()), "invalid nonce");
        let key = self.keys.get(&request.principal).ok_or_else(|| anyhow::anyhow!("unknown principal"))?;
        let mut mac = HmacSha256::new_from_slice(key).map_err(|_| anyhow::anyhow!("invalid key"))?;
        let canonical = serde_json::to_vec(&(&request.principal, &request.topic, &request.nonce, &request.call))?;
        mac.update(&canonical);
        let signature = decode_hex(&request.mac_hex)?;
        mac.verify_slice(&signature).map_err(|_| anyhow::anyhow!("bad signature"))?;
        anyhow::ensure!(self.guard.tenant_for(&request.principal).is_some(), "unknown tenant");
        let identity = (request.principal.clone(), request.nonce);
        anyhow::ensure!(!self.used.contains(&identity), "replay denied");
        anyhow::ensure!(self.used.len() < MAX_REPLAY, "replay store full");
        self.used.insert(identity);
        let tenant = self.guard.tenant_for(&request.principal).ok_or_else(|| anyhow::anyhow!("unknown tenant"))?.to_owned();
        let peer = AuthenticatedPeer { principal: request.principal, tenant: tenant.clone() };
        let policy = self.policies.get(&tenant).ok_or_else(|| anyhow::anyhow!("no tenant policy"))?;
        dispatch_tool_call(&self.guard, &peer, &request.topic, &request.call, policy, &self.registry)
            .map(|_| ())
            .map_err(|_| anyhow::anyhow!("OMAt request denied"))
    }
}

fn decode_hex(value: &str) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(value.len() == 64, "invalid MAC length");
    (0..32).map(|i| u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(Into::into)).collect()
}

/// Zenoh queryable: authenticated ingress; all requests remain fail-closed
/// until the separately attested OSTc executor is configured.
pub async fn serve_omat_zenoh(
    session: Arc<zenoh::Session>,
    admission: OmatZenohAdmission,
    key: &str,
) -> anyhow::Result<()> {
    let state = tokio::sync::Mutex::new(admission);
    let queryable = session.declare_queryable(key).await.map_err(|e| anyhow::anyhow!("declare OMAt queryable: {e}"))?;
    state.lock().await.connected(true);
    while let Ok(query) = queryable.recv_async().await {
        let result = match query.payload() {
            Some(payload) => state.lock().await.handle(payload.to_bytes().as_ref()),
            None => Err(anyhow::anyhow!("missing payload")),
        };
        if result.is_err() {
            let _ = query.reply_err("rejected").await;
        } else {
            let _ = query.reply(key, b"accepted".as_slice()).await;
        }
    }
    state.lock().await.connected(false);
    Ok(())
}
