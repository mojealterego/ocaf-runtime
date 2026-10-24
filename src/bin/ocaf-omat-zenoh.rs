//! Dedicated OMAt Zenoh ingress. Requires vetted TLS transport configuration.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::omat_server::{OmatPolicy, OmatRegistry};
use ocaf_runtime::stage043_omat_zenoh::{serve_omat_zenoh, OmatZenohAdmission};
use ocaf_runtime::transport_policy::{read_zenoh_config, validate_zenoh_config};
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::path::Path;
use std::sync::Arc;

fn require(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)?;
    anyhow::ensure!(!value.is_empty(), "empty environment variable: {name}");
    Ok(value)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = require("OCAF_ZENOH_CONFIG")?;
    let config_path = Path::new(&config_path);
    anyhow::ensure!(config_path.is_absolute(), "OCAF_ZENOH_CONFIG must be absolute");
    let source = read_zenoh_config(config_path)?;
    validate_zenoh_config(&source, false)?;
    let config = zenoh::Config::from_json5(&source)
        .map_err(|e| anyhow::anyhow!("invalid Zenoh config: {e}"))?;
    let principal = require("OCAF_OMAT_PEER")?;
    let tenant = require("OCAF_OMAT_TENANT")?;
    let raw = B64.decode(require("OCAF_OMAT_SHARED_KEY_B64")?)?;
    let key: [u8; 32] = raw.try_into().map_err(|_| anyhow::anyhow!("HMAC key must be 32 bytes"))?;
    let replay_path = require("OCAF_OMAT_REPLAY_DB")?;
    anyhow::ensure!(Path::new(&replay_path).is_absolute(), "replay DB path must be absolute");
    let policies = HashMap::from([(tenant.clone(), OmatPolicy {
        tenant_id: tenant.clone(),
        allowed_tools: BTreeSet::new(),
        allowed_capabilities: BTreeSet::new(),
        max_timeout_ms: 1000,
        max_output_bytes: 1024,
        allow_external_side_effects: false,
    })]);
    let admission = OmatZenohAdmission::new(
        HashMap::from([(principal.clone(), key)]),
        HashMap::from([(principal, tenant.clone())]),
        policies,
        OmatRegistry::default(),
    )?.with_replay_database(Path::new(&replay_path))?;
    let session = Arc::new(zenoh::open(config).await.map_err(|e| anyhow::anyhow!("Zenoh open: {e}"))?);
    let selector = format!("ocaf/omat/{tenant}/requests/*");
    serve_omat_zenoh(session, admission, &selector).await
}
