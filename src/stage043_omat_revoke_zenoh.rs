//! Dedicated OMAt control queryable for signed, durable tool revocation.
//! The operator-provisioned Ed25519 key and sender mapping are the authority.
use crate::omas_signed::SignedOperation;
use crate::stage043_store::{Stage043ToolStore, ToolRevocationRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_CONTROL_FRAME: usize = 16 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedRevokeFrame {
    pub request: ToolRevocationRequest,
    pub operation: SignedOperation,
}

#[derive(Clone)]
pub struct RevokeAuthority {
    pub tenant: String,
    pub sender: String,
    pub public_key: [u8; 32],
    pub store: Stage043ToolStore,
}

impl RevokeAuthority {
    pub fn selector(&self) -> String {
        format!("ocaf/omat/{}/control/revoke", self.tenant)
    }
    pub fn handle(&self, selector: &str, bytes: &[u8], now: i64) -> anyhow::Result<()> {
        anyhow::ensure!(selector == self.selector(), "wrong revoke selector");
        anyhow::ensure!(bytes.len() <= MAX_CONTROL_FRAME, "oversized revoke frame");
        let frame: SignedRevokeFrame = serde_json::from_slice(bytes)?;
        self.store.revoke_signed(
            &frame.request,
            &frame.operation,
            &self.sender,
            &self.tenant,
            &self.public_key,
            now,
        )?;
        Ok(())
    }
}

pub async fn serve_revoke_zenoh_until<F>(
    session: Arc<zenoh::Session>,
    authority: RevokeAuthority,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = ()> + Send,
{
    let selector = authority.selector();
    let queryable = session
        .declare_queryable(&selector)
        .await
        .map_err(|e| anyhow::anyhow!("declare revoke queryable: {e}"))?;
    tokio::pin!(shutdown);
    loop {
        let query = tokio::select! {
            _ = &mut shutdown => break,
            result=queryable.recv_async() => match result { Ok(q)=>q, Err(_)=>break },
        };
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let outcome = query
            .payload()
            .ok_or_else(|| anyhow::anyhow!("missing revoke payload"))
            .and_then(|payload| {
                authority.handle(query.key_expr().as_str(), payload.to_bytes().as_ref(), now)
            });
        if outcome.is_ok() {
            let _ = query
                .reply(query.key_expr().as_str(), b"revoked".as_slice())
                .await;
        } else {
            tracing::warn!("omat_signed_revoke_rejected");
            let _ = query.reply_err("rejected").await;
        }
    }
    Ok(())
}

pub async fn request_revoke_zenoh(
    session: &zenoh::Session,
    selector: &str,
    frame: &SignedRevokeFrame,
) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(frame)?;
    anyhow::ensure!(payload.len() <= MAX_CONTROL_FRAME, "oversized revoke frame");
    let replies = session
        .get(selector)
        .payload(payload)
        .timeout(std::time::Duration::from_secs(5))
        .await
        .map_err(|e| anyhow::anyhow!("revoke Zenoh get: {e}"))?;
    let reply = tokio::time::timeout(std::time::Duration::from_secs(6), replies.recv_async())
        .await?
        .map_err(|e| anyhow::anyhow!("revoke response unavailable: {e}"))?;
    let sample = reply
        .result()
        .map_err(|_| anyhow::anyhow!("revoke rejected"))?;
    anyhow::ensure!(
        sample.payload().to_bytes().as_ref() == b"revoked",
        "unexpected revoke reply"
    );
    Ok(())
}
