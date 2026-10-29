//! Dedicated OMAt Zenoh ingress. Requires vetted TLS transport configuration.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::omat_server::{OmatPolicy, OmatRegistry};
use ocaf_runtime::stage043_omat_discovery::{serve_discovery_until, DurableDiscovery};
use ocaf_runtime::stage043_omat_revoke_zenoh::{serve_revoke_zenoh_until, RevokeAuthority};
use ocaf_runtime::stage043_omat_zenoh::{serve_omat_zenoh_until, OmatZenohAdmission};
use ocaf_runtime::stage043_store::Stage043ToolStore;
use ocaf_runtime::transport_policy::{read_zenoh_config, validate_zenoh_config};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;

fn require(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)?;
    anyhow::ensure!(!value.is_empty(), "empty environment variable: {name}");
    Ok(value)
}

struct B64Decoded([u8; 32]);
impl B64Decoded {
    fn new(raw: &str) -> anyhow::Result<Self> {
        let bytes = B64.decode(raw)?;
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Ed25519 public key must be 32 bytes"))?;
        anyhow::ensure!(key != [0; 32], "zero revoke public key");
        Ok(Self(key))
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                tokio::select! {
                    _ = signal.recv() => {},
                    _ = tokio::signal::ctrl_c() => {},
                }
                return;
            }
            Err(error) => {
                tracing::error!(%error, "omat_sigterm_handler_unavailable");
            }
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let config_path = require("OCAF_ZENOH_CONFIG")?;
    let config_path = Path::new(&config_path);
    anyhow::ensure!(
        config_path.is_absolute(),
        "OCAF_ZENOH_CONFIG must be absolute"
    );
    let source = read_zenoh_config(config_path)?;
    validate_zenoh_config(&source, false)?;
    let config = zenoh::Config::from_json5(&source)
        .map_err(|e| anyhow::anyhow!("invalid Zenoh config: {e}"))?;
    let principal = require("OCAF_OMAT_PEER")?;
    let tenant = require("OCAF_OMAT_TENANT")?;
    let raw = B64.decode(require("OCAF_OMAT_SHARED_KEY_B64")?)?;
    let key: [u8; 32] = raw
        .try_into()
        .map_err(|_| anyhow::anyhow!("HMAC key must be 32 bytes"))?;
    let replay_path = require("OCAF_OMAT_REPLAY_DB")?;
    anyhow::ensure!(
        Path::new(&replay_path).is_absolute(),
        "replay DB path must be absolute"
    );
    let tool_db = require("OCAF_OMAT_TOOL_DB")?;
    let tool_db = Path::new(&tool_db);
    anyhow::ensure!(tool_db.is_absolute(), "tool DB path must be absolute");
    let store = Stage043ToolStore::open_existing(tool_db)?;
    let policy_path = require("OCAF_OMAT_POLICY_JSON")?;
    let policy_path = Path::new(&policy_path);
    anyhow::ensure!(
        policy_path.is_absolute() && !policy_path.is_symlink(),
        "private absolute policy path required"
    );
    let policy_metadata = policy_path.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        anyhow::ensure!(
            policy_metadata.uid() == unsafe { libc::geteuid() }
                && policy_metadata.permissions().mode() & 0o077 == 0,
            "policy file must be private and owned by OMAt"
        );
    }
    anyhow::ensure!(
        policy_metadata.is_file() && policy_metadata.len() <= 1024 * 1024,
        "bounded policy file required"
    );
    let policy: OmatPolicy = serde_json::from_slice(&std::fs::read(policy_path)?)?;
    anyhow::ensure!(policy.tenant_id == tenant, "policy tenant mismatch");
    let policies = HashMap::from([(tenant.clone(), policy.clone())]);
    let revoke_sender = require("OCAF_OMAT_REVOKE_SENDER")?;
    let revoke_public: B64Decoded = B64Decoded::new(&require("OCAF_OMAT_REVOKE_PUBLIC_KEY_B64")?)?;
    let authority = RevokeAuthority {
        tenant: tenant.clone(),
        sender: revoke_sender,
        public_key: revoke_public.0,
        store: store.clone(),
    };
    let principal_for_discovery = principal.clone();
    let mut registry = OmatRegistry::default();
    for manifest in store.list_active(&tenant)? {
        registry.register(manifest)?;
    }
    let mut admission = OmatZenohAdmission::new(
        HashMap::from([(principal.clone(), key)]),
        HashMap::from([(principal, tenant.clone())]),
        policies,
        registry,
    )?
    .with_replay_database(Path::new(&replay_path))?
    .with_tool_store(store.clone());

    // Opt-in operator-owned capability key. If absent, the live ingress
    // rejects all capability dispatches; it never falls back to legacy
    // authorization. The private file is read with the existing O_NOFOLLOW
    // and owner-only permissions enforcement.
    if let Ok(key_path) = env::var("OCAF_OMAT_CAPABILITY_KEY_FILE") {
        let path = Path::new(&key_path);
        anyhow::ensure!(path.is_absolute(), "capability key path must be absolute");
        let encoded = ocaf_runtime::ops::secrets::read_private_text(path, 128)?;
        let cap_key: [u8; 32] = B64
            .decode(encoded.trim_end_matches(['\r', '\n']))?
            .try_into()
            .map_err(|_| anyhow::anyhow!("capability authority key must be 32 bytes"))?;
        anyhow::ensure!(
            cap_key != key,
            "capability key must differ from OMAs transport key"
        );
        let authority = ocaf_runtime::sasos_capability::CapabilityAuthority::new(cap_key)?;
        admission = admission.with_capability_authority(authority);
    }
    let session = Arc::new(
        zenoh::open(config)
            .await
            .map_err(|e| anyhow::anyhow!("Zenoh open: {e}"))?,
    );
    let selector = format!("ocaf/omat/{tenant}/requests/*");
    let discovery = DurableDiscovery::open(
        store,
        policy,
        principal_for_discovery,
        key,
        Path::new(&replay_path),
    )?;
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = stop_tx.send(true);
    });
    let mut request_stop = stop_rx.clone();
    let mut discovery_stop = stop_rx.clone();
    let mut revoke_stop = stop_rx;
    tokio::try_join!(
        serve_omat_zenoh_until(Arc::clone(&session), admission, &selector, async move {
            let _ = request_stop.changed().await;
        }),
        serve_discovery_until(Arc::clone(&session), discovery, &tenant, async move {
            let _ = discovery_stop.changed().await;
        }),
        serve_revoke_zenoh_until(session, authority, async move {
            let _ = revoke_stop.changed().await;
        })
    )?;
    Ok(())
}
