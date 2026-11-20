//! Real Zenoh query/reply for non-sensitive staging probes only.
//! The trusted OM Engine owns production sessions, authentication and execution.
use crate::{valid_segment, AgentProfile};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct Probe<'a> {
    protocol: &'static str,
    agent_id: &'a str,
    nonce: &'a str,
}

pub fn probe_key(profile: &AgentProfile, nonce: &str) -> Result<String, String> {
    if !valid_segment(profile.id) || !valid_segment(nonce) {
        return Err("invalid probe identity".into());
    }
    Ok(format!("ocaf/staging/agents/{}/probes/{}", profile.id, nonce))
}

pub async fn staging_probe(session: &zenoh::Session, profile: &AgentProfile, nonce: &str) -> Result<(), String> {
    let key = probe_key(profile, nonce)?;
    let expected = serde_json::to_vec(&Probe { protocol: "ocaf-agent-probe-v1", agent_id: profile.id, nonce })
        .map_err(|error| error.to_string())?;
    let replies = tokio::time::timeout(Duration::from_secs(5),
        async { session.get(key.as_str()).payload(expected.clone()).await })
        .await.map_err(|_| "probe send timeout".to_owned())?.map_err(|error| error.to_string())?;
    let reply = tokio::time::timeout(Duration::from_secs(5), replies.recv_async())
        .await.map_err(|_| "probe reply timeout".to_owned())?.map_err(|error| error.to_string())?;
    let sample = reply.result().map_err(|_| "probe rejected".to_owned())?;
    if sample.key_expr().as_str() != key || sample.payload().to_bytes().as_ref() != expected.as_slice() {
        return Err("probe correlation mismatch".into());
    }
    Ok(())
}
