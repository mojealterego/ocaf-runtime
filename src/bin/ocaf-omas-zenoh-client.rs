//! Independent OMAs process for authenticated Zenoh OMAt E2E probes.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::omat_server::ToolCall;
use ocaf_runtime::stage043_omat_zenoh::{request_omat_zenoh, sign_omas_request};
use ocaf_runtime::transport_policy::{read_zenoh_config, validate_zenoh_config};
use std::env;
use std::path::Path;

fn require(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)?;
    anyhow::ensure!(!value.is_empty(), "empty variable: {name}");
    Ok(value)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let path = require("OCAF_ZENOH_CONFIG")?;
    anyhow::ensure!(Path::new(&path).is_absolute(), "config path must be absolute");
    let source = read_zenoh_config(Path::new(&path))?;
    validate_zenoh_config(&source, false)?;
    let config = zenoh::Config::from_json5(&source)
        .map_err(|e| anyhow::anyhow!("invalid Zenoh config: {e}"))?;
    let principal = require("OCAF_OMAT_PEER")?;
    let tenant = require("OCAF_OMAT_TENANT")?;
    let key: [u8; 32] = B64.decode(require("OCAF_OMAT_SHARED_KEY_B64")?)?
        .try_into().map_err(|_| anyhow::anyhow!("key must be 32 bytes"))?;
    let nonce = require("OCAF_OMAT_NONCE")?;
    let request_id = require("OCAF_OMAT_REQUEST_ID")?;
    let topic = format!("ocaf/omat/{tenant}/requests/{request_id}");
    let call = ToolCall {
        request_id,
        tenant_id: tenant,
        agent_id: "agent_1".into(),
        tool_id: "tool_1".into(),
        arguments: serde_json::json!({}),
        idempotency_key: nonce.clone(),
        ostc_enabled: false,
        source_access: "disabled".into(),
    };
    let signed = sign_omas_request(&principal, &topic, &nonce, call, &key)?;
    let session = zenoh::open(config).await.map_err(|e| anyhow::anyhow!("Zenoh open: {e}"))?;
    request_omat_zenoh(&session, &topic, signed).await
}
