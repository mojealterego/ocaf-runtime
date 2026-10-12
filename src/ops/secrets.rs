//! Read secrets via private file descriptors, with no unauthenticated remote
//! listener and no secret values in error messages or Debug output.
use std::env;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

/// Absolute, regular, user-owned, owner-only file. The descriptor is opened
/// O_NOFOLLOW on Unix to prevent final-component symlink swaps.
pub fn read_private_text(path: &Path, max_bytes: usize) -> anyhow::Result<String> {
    if !path.is_absolute() || max_bytes == 0 || max_bytes > 65_536 {
        anyhow::bail!("invalid secure operator-file path or size limit");
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(path)
        .map_err(|_| anyhow::anyhow!("private operator file cannot be opened"))?;
    let meta = file
        .metadata()
        .map_err(|_| anyhow::anyhow!("operator file metadata unavailable"))?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > max_bytes as u64 {
        anyhow::bail!("operator file must be a nonempty regular file within size limit");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if meta.permissions().mode() & 0o077 != 0 || meta.uid() != unsafe { libc::geteuid() } {
            anyhow::bail!("operator file must be owned by service user and chmod 600");
        }
    }
    let mut data = Vec::new();
    file.take((max_bytes + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|_| anyhow::anyhow!("private operator file read failed"))?;
    if data.is_empty() || data.len() > max_bytes {
        anyhow::bail!("operator file has invalid size");
    }
    String::from_utf8(data).map_err(|_| anyhow::anyhow!("operator file is not UTF-8"))
}

/// Select exactly one configuration source. Deliberately refuse ambiguity:
/// accidentally retaining env secrets defeats the benefit of a file store.
pub fn resolve_sources(
    env_value: Option<&str>,
    file_path: Option<&Path>,
    max_bytes: usize,
) -> anyhow::Result<String> {
    let raw = match (env_value, file_path) {
        (Some(_), Some(_)) => anyhow::bail!("conflicting environment and secret file sources"),
        (None, None) => anyhow::bail!("required operator secret not configured"),
        (Some(value), None) => {
            if value.len() > max_bytes {
                anyhow::bail!("operator secret exceeds size limit");
            }
            value.to_owned()
        }
        (None, Some(path)) => read_private_text(path, max_bytes)?,
    };
    let value = raw.trim_end_matches(['\r', '\n']);
    if value.is_empty() || value.contains('\0') || value.contains('\n') || value.contains('\r') {
        anyhow::bail!("operator secret contains invalid characters");
    }
    Ok(value.to_owned())
}

/// Load NAME or NAME_FILE (absolute private file path); never both.
pub fn load_env_secret(name: &str, max_bytes: usize) -> anyhow::Result<String> {
    let value = match env::var(name) {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(_) => anyhow::bail!("operator environment secret is not valid UTF-8"),
    };
    let file_name = format!("{name}_FILE");
    let file = match env::var(&file_name) {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(_) => anyhow::bail!("operator secret file path is not valid UTF-8"),
    };
    resolve_sources(value.as_deref(), file.as_deref().map(Path::new), max_bytes)
}
