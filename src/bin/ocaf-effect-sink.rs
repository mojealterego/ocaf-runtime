//! Standalone effect receiver process, deliberately separate from leader
//! and worker. It does not execute arbitrary code or send external HTTP.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::sink::{EffectSink, SinkKeys};
use ocaf_runtime::transport::sink::ZenohSinkServer;
use ocaf_runtime::transport_policy::{read_zenoh_config, validate_zenoh_config};

fn required(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)
        .map_err(|_| anyhow::anyhow!("missing required configuration: {name}"))?;
    if value.is_empty() {
        anyhow::bail!("empty configuration: {name}");
    }
    Ok(value)
}
fn absolute(name: &str) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(required(name)?);
    if !path.is_absolute() {
        anyhow::bail!("{name} must be an absolute path");
    }
    Ok(path)
}
fn key(name: &str) -> anyhow::Result<[u8; 32]> {
    let raw = B64.decode(required(name)?)
        .map_err(|_| anyhow::anyhow!("invalid base64 secret: {name}"))?;
    let key: [u8; 32] = raw
        .try_into()
        .map_err(|_| anyhow::anyhow!("{name} must decode to 32 bytes"))?;
    if key == [0; 32] {
        anyhow::bail!("empty sink secret is prohibited");
    }
    Ok(key)
}
fn assert_private_parent(path: &Path) -> anyhow::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("missing sink database directory"))?;
    let meta = fs::symlink_metadata(dir)?;
    if !meta.file_type().is_dir() {
        anyhow::bail!("sink DB parent must be a real directory");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o022 != 0 {
            anyhow::bail!("sink DB parent may not be group/world writable");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let id = required("OCAF_SINK_ID")?;
    let db = absolute("OCAF_SINK_DB_PATH")?;
    assert_private_parent(&db)?;
    let keys = SinkKeys::new(
        &required("OCAF_SINK_CONTROLLER_ID")?,
        key("OCAF_SINK_CONTROLLER_KEY_B64")?,
        &required("OCAF_SINK_WORKER_ID")?,
        key("OCAF_SINK_WORKER_KEY_B64")?,
    )?;
    let config_path = absolute("OCAF_ZENOH_CONFIG")?;
    let source = read_zenoh_config(&config_path)?;
    let allow_loopback = env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1");
    validate_zenoh_config(&source, allow_loopback)?;
    let config = zenoh::Config::from_json5(&source)
        .map_err(|e| anyhow::anyhow!("invalid vetted sink Zenoh config: {e}"))?;
    let sink = Arc::new(EffectSink::open(&db, &id, keys)?);
    let session = Arc::new(
        zenoh::open(config)
            .await
            .map_err(|e| anyhow::anyhow!("open sink Zenoh session: {e}"))?,
    );
    tracing::info!(sink_id=%id, "independent effect receiver serving");
    ZenohSinkServer::new(session, sink).serve().await
}
