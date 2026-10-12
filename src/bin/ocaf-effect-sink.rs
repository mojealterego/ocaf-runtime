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
    let value =
        env::var(name).map_err(|_| anyhow::anyhow!("missing required configuration: {name}"))?;
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
/// Read an owner-only key file via its descriptor, refusing a final
/// symlink on Unix. The payload is bounded and never logged.
fn read_private_secret(path: &Path) -> anyhow::Result<String> {
    use std::io::Read;
    if !path.is_absolute() {
        anyhow::bail!("sink secret file path must be absolute");
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > 256 {
        anyhow::bail!("sink secret file must be a nonempty regular file <=256 bytes");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if meta.permissions().mode() & 0o077 != 0 || meta.uid() != unsafe { libc::geteuid() } {
            anyhow::bail!("sink secret file must be owned by service user and chmod 600");
        }
    }
    let mut data = String::new();
    file.take(257).read_to_string(&mut data)?;
    if data.len() > 256 {
        anyhow::bail!("sink secret file exceeds size limit");
    }
    Ok(data.trim().to_owned())
}
fn key(name: &str) -> anyhow::Result<[u8; 32]> {
    let file_key = format!(
        "{}_FILE",
        name.strip_suffix("_B64")
            .ok_or_else(|| anyhow::anyhow!("invalid secret variable name"))?
    );
    let encoded = match (env::var(name).ok(), env::var(&file_key).ok()) {
        (Some(_), Some(_)) => anyhow::bail!("{name} conflicts with {file_key}"),
        (Some(value), None) => value,
        (None, Some(path)) => read_private_secret(Path::new(&path))?,
        (None, None) => anyhow::bail!("configure either {name} or {file_key}"),
    };
    let raw = B64
        .decode(encoded)
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
    )?
    .with_cluster_id(&required("OCAF_SINK_CLUSTER_ID")?)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn sink_secret_files_must_be_private_and_cannot_be_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("controller.key");
        std::fs::write(&real, B64.encode([47u8; 32])).unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(read_private_secret(&real).unwrap(), B64.encode([47u8; 32]));
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_private_secret(&real).is_err());
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = dir.path().join("alias.key");
        symlink(&real, &link).unwrap();
        assert!(read_private_secret(&link).is_err());
        let relative = Path::new("controller.key");
        assert!(read_private_secret(relative).is_err());
    }
}
