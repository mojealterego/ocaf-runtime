//! Controlled deployment of exactly one operator-approved WASIp1 tool.
//! This loader is used by the coordinator and the TLS-bound remote worker.
//! No tenant, manifest, module path or publisher key is read from RPC JSON.
use std::collections::HashSet;
use std::env;
use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Result};
use openssl::pkey::{Id, PKey};

use crate::executor::wasi::{PinnedWasmtime, WasmToolExecutor};
use crate::runtime::OcafRuntime;

fn required(name: &str) -> Result<String> {
    let v = env::var(name)?;
    if v.is_empty() {
        bail!("{name} must be nonempty");
    }
    Ok(v)
}

fn path(name: &str) -> Result<PathBuf> {
    let p = PathBuf::from(required(name)?);
    if !p.is_absolute() {
        bail!("{name} must be an absolute path");
    }
    Ok(p)
}

/// Open a sealed operator file by descriptor, rather than checking a path
/// and reopening it later. O_NOFOLLOW rejects final-component symlink swaps;
/// fstat checks the inode actually read and enforces effective-UID custody.
fn read_operator_file(p: &Path, name: &str, max_bytes: u64) -> Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(p)?;
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.permissions().mode() & 0o077 != 0
        || meta.len() == 0
        || meta.len() > max_bytes
    {
        bail!("{name} must be a bounded, single-linked, effective-UID-owned, owner-only file");
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() as u64 > max_bytes {
        bail!("{name} changed during reading");
    }
    Ok(bytes)
}

fn owner_only_file(name: &str, max_bytes: u64) -> Result<Vec<u8>> {
    let p = path(name)?;
    read_operator_file(&p, name, max_bytes)
}

fn owner_only_replay_database(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
        bail!("OMAS replay database path must be absolute and normalized");
    }
    let file = path.symlink_metadata()?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("OMAS replay path has no parent"))?;
    let directory = parent.symlink_metadata()?;
    if parent.canonicalize()? != parent {
        bail!("OMAS replay database parent must not contain a symlink");
    }
    if !file.file_type().is_file()
        || file.nlink() != 1
        || file.uid() != unsafe { libc::geteuid() }
        || file.permissions().mode() & 0o077 != 0
        || !directory.file_type().is_dir()
        || directory.uid() != unsafe { libc::geteuid() }
        || directory.permissions().mode() & 0o077 != 0
    {
        bail!("OMAS replay database and parent must be private, owned, regular files");
    }
    Ok(())
}

#[cfg(test)]
mod omas_replay_path_tests {
    use super::owner_only_replay_database;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn accepts_private_database_and_rejects_symlink_or_public_parent() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("private");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let db = dir.join("nonce.sqlite");
        std::fs::write(&db, b"").unwrap();
        std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(owner_only_replay_database(&db).is_ok());
        let link = dir.join("link.sqlite");
        symlink(&db, &link).unwrap();
        assert!(owner_only_replay_database(&link).is_err());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(owner_only_replay_database(&db).is_err());
        assert!(owner_only_replay_database(&dir.join("..").join("private/nonce.sqlite")).is_err());
    }
}

const SIGNED_WASI_REQUIRED_ENV: [&str; 6] = [
    "OCAF_WASI_MANIFEST_FILE",
    "OCAF_WASI_SIGNATURE_FILE",
    "OCAF_WASI_PUBLISHER_KEY_FILE",
    "OCAF_WASI_MODULE_FILE",
    "OCAF_WASMTIME_EXECUTABLE",
    "OCAF_WASMTIME_SHA256",
];

/// Returns whether the complete signed WASI configuration was requested.
/// Keep this pure so partial or misspelled bundles can be tested independently.
fn signed_wasi_bundle_enabled(names: &HashSet<String>) -> Result<bool> {
    let is_wasi_key =
        |name: &str| name.starts_with("OCAF_WASI_") || name.starts_with("OCAF_WASMTIME_");
    let requested = names.iter().any(|name| is_wasi_key(name));
    if !requested {
        return Ok(false);
    }

    // Reject unknown names as well as incomplete bundles. Otherwise a typo
    // silently disables or weakens a security-sensitive deployment.
    for name in names {
        if is_wasi_key(name) && !SIGNED_WASI_REQUIRED_ENV.contains(&name.as_str()) {
            bail!("unknown signed WASI configuration variable: {name}");
        }
    }
    for required in SIGNED_WASI_REQUIRED_ENV {
        if !names.contains(required) {
            bail!("incomplete signed WASI configuration, missing {required}");
        }
    }
    Ok(true)
}

fn lowercase_sha256(raw: &str) -> Result<[u8; 32]> {
    if raw.len() != 64
        || !raw.bytes().all(|b| b.is_ascii_hexdigit())
        || raw.bytes().any(|b| b.is_ascii_uppercase())
    {
        bail!("SHA-256 must be exactly 64 lowercase hexadecimal characters");
    }
    let mut out = [0u8; 32];
    for (index, bytes) in raw.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        out[index] = u8::from_str_radix(std::str::from_utf8(bytes)?, 16)?;
    }
    Ok(out)
}

/// Registers one signed tool if OCAF_WASI_MANIFEST_FILE is set. A typo or
/// incomplete bundle aborts service initialization rather than falling back.
/// Agent identity must be derived from an authenticated TLS-bound RPC peer.
/// The caller supplies a deployment-owned namespace via environment only.
pub async fn runtime_from_env() -> Result<OcafRuntime> {
    let namespace = match env::var("OCAF_TENANT_NAMESPACE") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => bail!("invalid tenant namespace environment"),
    };
    let mut runtime = OcafRuntime::new(0);
    if let Some(value) = &namespace {
        runtime = runtime.with_tenant_namespace(value)?;
    }

    let omas_key_requested = env::var_os("OCAF_OMAS_PUBLIC_KEY_FILE").is_some();
    let omas_replay_requested = env::var_os("OCAF_OMAS_REPLAY_DB_PATH").is_some();
    if namespace.is_some() {
        if !omas_key_requested || !omas_replay_requested {
            bail!("tenant-bound runtime requires complete OMAS key and replay configuration");
        }
        let key = owner_only_file("OCAF_OMAS_PUBLIC_KEY_FILE", 2048)?;
        let public = PKey::public_key_from_pem(&key)?;
        if public.id() != Id::ED25519 {
            bail!("OMAS public key must be Ed25519");
        }
        let bytes: [u8; 32] = public
            .raw_public_key()?
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid OMAS Ed25519 public key"))?;
        let replay = path("OCAF_OMAS_REPLAY_DB_PATH")?;
        owner_only_replay_database(&replay)?;
        runtime = runtime
            .with_omas_trusted_key(bytes)
            .with_omas_replay_database(replay);
    } else if omas_key_requested || omas_replay_requested {
        bail!("OMAS authority cannot be configured without a tenant namespace");
    }

    let config_names: HashSet<String> = env::vars_os()
        .map(|(key, _)| key.to_string_lossy().into_owned())
        .collect();
    if !signed_wasi_bundle_enabled(&config_names)? {
        return Ok(runtime);
    }
    if namespace.is_none()
        || env::var("OCAF_RPC_INGRESS").as_deref() != Ok("tls-bound")
        || env::var("OCAF_REQUIRE_PEER_CERT_POP").as_deref() != Ok("1")
    {
        bail!("signed WASI runtime requires tenant namespace, TLS-bound RPC and certificate PoP");
    }
    let engine = PinnedWasmtime::new(
        path("OCAF_WASMTIME_EXECUTABLE")?,
        lowercase_sha256(&required("OCAF_WASMTIME_SHA256")?)?,
    )?;
    let module = owner_only_file("OCAF_WASI_MODULE_FILE", 1024 * 1024)?;
    let manifest = owner_only_file("OCAF_WASI_MANIFEST_FILE", 24 * 1024)?;
    let signature = owner_only_file("OCAF_WASI_SIGNATURE_FILE", 64)?;
    let key = owner_only_file("OCAF_WASI_PUBLISHER_KEY_FILE", 2048)?;
    let approved =
        WasmToolExecutor::from_signed_manifest(engine, module, &manifest, &signature, &key)?;
    runtime.register_signed_wasi(approved).await?;
    Ok(runtime)
}

#[cfg(test)]
mod operator_file_tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn descriptor_based_loader_rejects_untrusted_path_and_inode_metadata() {
        let workspace = tempfile::tempdir().expect("temporary operator workspace");
        let sealed = workspace.path().join("manifest.json");
        fs::write(&sealed, b"safe").expect("create manifest");
        fs::set_permissions(&sealed, fs::Permissions::from_mode(0o600)).expect("seal manifest");

        assert_eq!(
            read_operator_file(&sealed, "TEST_MANIFEST", 4).expect("approved file"),
            b"safe"
        );
        assert!(read_operator_file(&sealed, "TEST_MANIFEST", 3).is_err());

        let link = workspace.path().join("swapped.json");
        symlink(&sealed, &link).expect("make symlink");
        assert!(read_operator_file(&link, "TEST_MANIFEST", 16).is_err());

        let second_link = workspace.path().join("hardlink.json");
        fs::hard_link(&sealed, &second_link).expect("make hardlink");
        assert!(read_operator_file(&sealed, "TEST_MANIFEST", 16).is_err());
        fs::remove_file(&second_link).expect("remove hardlink");

        fs::set_permissions(&sealed, fs::Permissions::from_mode(0o644))
            .expect("make world-readable");
        assert!(read_operator_file(&sealed, "TEST_MANIFEST", 16).is_err());
        fs::set_permissions(&sealed, fs::Permissions::from_mode(0o600)).expect("reseal file");

        fs::write(&sealed, b"").expect("clear file");
        assert!(read_operator_file(&sealed, "TEST_MANIFEST", 16).is_err());
    }
    #[test]
    fn signed_wasi_bundle_requires_every_setting_and_rejects_misspelled_keys() {
        let expected: HashSet<String> = SIGNED_WASI_REQUIRED_ENV
            .iter()
            .map(|key| (*key).to_owned())
            .collect();
        assert!(!signed_wasi_bundle_enabled(&HashSet::new()).expect("legacy unconfigured mode"));
        assert!(signed_wasi_bundle_enabled(&expected).expect("complete operator bundle"));

        for name in SIGNED_WASI_REQUIRED_ENV {
            let mut incomplete = expected.clone();
            incomplete.remove(name);
            assert!(
                signed_wasi_bundle_enabled(&incomplete).is_err(),
                "missing {name} must abort deployment"
            );
        }

        for wrong in ["OCAF_WASI_MANIFEST_FIEL", "OCAF_WASMTIME_SHA25"] {
            let mut misspelled = expected.clone();
            misspelled.insert(wrong.to_owned());
            assert!(
                signed_wasi_bundle_enabled(&misspelled).is_err(),
                "unknown operator variable {wrong} must abort startup"
            );
        }

        let mut unrelated = expected.clone();
        unrelated.insert("OCAF_TEST_PG_DSN".to_owned());
        assert!(signed_wasi_bundle_enabled(&unrelated).expect("unrelated environment is allowed"));
    }
}
