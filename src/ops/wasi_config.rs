//! Controlled deployment of exactly one operator-approved WASIp1 tool.
//! This loader is used by the coordinator and the TLS-bound remote worker.
//! No tenant, manifest, module path or publisher key is read from RPC JSON.
use std::env;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

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

fn owner_only_file(name: &str, max_bytes: u64) -> Result<Vec<u8>> {
    let p = path(name)?;
    let meta = fs::symlink_metadata(&p)?;
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.nlink() != 1
        || meta.permissions().mode() & 0o077 != 0
        || meta.len() == 0
        || meta.len() > max_bytes
    {
        bail!("{name} must be a bounded single-linked owner-only file");
    }
    let bytes = fs::read(p)?;
    if bytes.is_empty() || bytes.len() as u64 > max_bytes {
        bail!("{name} changed during reading");
    }
    Ok(bytes)
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
    let approved = WasmToolExecutor::from_signed_manifest(
        engine, module, &manifest, &signature, &key
    )?;
    runtime.register_signed_wasi(approved).await?;
    Ok(runtime)
}
