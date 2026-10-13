//! Controlled deployment of exactly one operator-approved WASIp1 tool.
//! This loader is used by the coordinator and the TLS-bound remote worker.
//! No tenant, manifest, module path or publisher key is read from RPC JSON.
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

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
    let mut file = OpenOptions::new()
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
    let namespace = env::var("OCAF_TENANT_NAMESPACE").ok();
    let mut runtime = OcafRuntime::new(0);
    if let Some(value) = &namespace {
        runtime = runtime.with_tenant_namespace(value)?;
    }

    if env::var("OCAF_WASI_MANIFEST_FILE").is_err() {
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
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn descriptor_based_loader_rejects_untrusted_path_and_inode_metadata() {
        let workspace = tempfile::tempdir().expect("temporary operator workspace");
        let sealed = workspace.path().join("manifest.json");
        fs::write(&sealed, b"safe").expect("create manifest");
        fs::set_permissions(&sealed, fs::Permissions::from_mode(0o600))
            .expect("seal manifest");

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
        fs::set_permissions(&sealed, fs::Permissions::from_mode(0o600))
            .expect("reseal file");

        fs::write(&sealed, b"").expect("clear file");
        assert!(read_operator_file(&sealed, "TEST_MANIFEST", 16).is_err());
    }
}
