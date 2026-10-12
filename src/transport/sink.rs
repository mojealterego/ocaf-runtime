//! Independent effect sink network transport. Zenoh TLS/mTLS config must
//! be validated by the launching binary before opening the session.
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::cluster::leadership::PgLeader;
use crate::rpc::{now_ms, validate_node_id, MAX_FRAME_BYTES, SignedFrame};
use crate::sink::{EffectSink, SinkAction, SinkError, SinkPermit, SinkRequest, SinkResponse};

const WIRE_TIMEOUT: Duration = Duration::from_secs(6);

pub fn sink_key(sink_id: &str) -> String {
    format!("ocaf/v1/sinks/{sink_id}/effects")
}

pub struct ZenohSinkServer {
    session: Arc<zenoh::Session>,
    sink: Arc<EffectSink>,
}

impl ZenohSinkServer {
    pub fn new(session: Arc<zenoh::Session>, sink: Arc<EffectSink>) -> Self {
        Self { session, sink }
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        let key = sink_key(self.sink.id());
        let pending = Arc::new(Semaphore::new(32));
        let queryable = self
            .session
            .declare_queryable(key.as_str())
            .await
            .map_err(|e| anyhow::anyhow!("declare effect sink queryable: {e}"))?;
        while let Ok(query) = queryable.recv_async().await {
            let permit = match Arc::clone(&pending).try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    let _ = query.reply_err("rejected").await;
                    continue;
                }
            };
            let receiver = Arc::clone(&self.sink);
            let reply_key = key.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let result = match query.payload() {
                    Some(payload) if payload.len() <= 100_000 => {
                        receiver.handle(payload.to_bytes().as_ref()).await
                    }
                    _ => Err(SinkError::Invalid),
                };
                match result {
                    Ok(bytes) => {
                        if let Err(error) = query.reply(reply_key, bytes).await {
                            tracing::warn!(error=%error, "independent sink reply failed");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(reason=%error, "independent sink rejected request");
                        let _ = query.reply_err("rejected").await;
                    }
                }
            });
        }
        Ok(())
    }
}

pub struct ZenohSinkClient {
    session: Arc<zenoh::Session>,
    sink_id: String,
    sender: String,
    key: [u8; 32],
}

impl ZenohSinkClient {
    pub fn new(
        session: Arc<zenoh::Session>,
        sink_id: &str,
        sender: &str,
        key: [u8; 32],
    ) -> anyhow::Result<Self> {
        validate_node_id(sink_id)?;
        validate_node_id(sender)?;
        if key == [0; 32] {
            anyhow::bail!("all-zero sink key denied");
        }
        Ok(Self {
            session,
            sink_id: sink_id.into(),
            sender: sender.into(),
            key,
        })
    }

    pub async fn request(&self, action: SinkAction) -> anyhow::Result<SinkResponse> {
        let req = SinkRequest::new(&self.sink_id, &self.sender, now_ms()?, 5_000, action)?;
        let frame = req.seal(&self.key)?;
        let key = sink_key(&self.sink_id);
        let replies = self
            .session
            .get(key.as_str())
            .payload(frame)
            .timeout(WIRE_TIMEOUT)
            .await
            .map_err(|e| anyhow::anyhow!("effect sink request: {e}"))?;
        let reply = tokio::time::timeout(WIRE_TIMEOUT, replies.recv_async())
            .await
            .map_err(|_| anyhow::anyhow!("effect sink reply timed out"))?
            .map_err(|e| anyhow::anyhow!("effect sink reply missing: {e}"))?;
        let sample = reply
            .result()
            .map_err(|_| anyhow::anyhow!("effect sink rejected request"))?;
        if sample.payload().len() > MAX_FRAME_BYTES {
            anyhow::bail!("oversized sink reply");
        }
        Ok(SinkResponse::open(
            sample.payload().to_bytes().as_ref(),
            &self.key,
            &req,
        )?)
    }

    /// The trusted controller must have the current PgLeader. The separate
    /// sink retains its own monotonic epoch and expires this authorization
    /// after a short interval, even if the coordinator is unreachable.
    pub async fn activate_from_leader(
        &self,
        leader: &PgLeader,
        validity_ms: u64,
    ) -> anyhow::Result<SinkResponse> {
        if !(1..=15_000).contains(&validity_ms) {
            anyhow::bail!("sink activation validity must be 1..=15000 ms");
        }
        let epoch = leader.fencing_epoch().await?;
        let expires_at_ms = now_ms()?
            .checked_add(validity_ms)
            .ok_or_else(|| anyhow::anyhow!("activation clock overflow"))?;
        self.request(SinkAction::ActivateEpoch {
            epoch,
            expires_at_ms,
        })
        .await
    }

    /// Trusted controller issues a bounded, payload-bound permit for one
    /// job and one named worker. Distribution is caller-specific and must
    /// only occur after job ACL and leader verification.
    pub async fn issue_permit(
        &self,
        leader: &PgLeader,
        worker: &str,
        job_id: &str,
        expires_at_ms: u64,
        effect: &serde_json::Value,
    ) -> anyhow::Result<Vec<u8>> {
        let epoch = leader.fencing_epoch().await?;
        let token = SinkPermit::new(
            &self.sink_id,
            worker,
            job_id,
            epoch,
            expires_at_ms,
            effect,
        )?;
        Ok(SignedFrame::seal(&token, &self.key)?)
    }
}
