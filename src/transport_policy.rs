//! Fail-closed operator-supplied Zenoh network configuration validation.
//!
//! This preflight blocks plaintext outside explicit loopback development and
//! requires TLS-only protocol whitelisting and mTLS material. It does NOT
//! replace Zenoh's actual certificate chain validation and TLS handshake.

use std::io::Read;
use std::net::IpAddr;
use std::path::Path;

use serde_json::Value;
use thiserror::Error;

const MAX_CONFIG_BYTES: usize = 65_536;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransportPolicyError {
    #[error("malformed or oversized Zenoh security configuration")]
    Malformed,
    #[error("Zenoh configuration violates the required transport security policy")]
    Unsafe,
}

/// Bound network configuration reads before JSON5 parsing. Never attempt to
/// load arbitrarily large or binary operator-supplied config into memory.
pub fn read_zenoh_config(path: &Path) -> Result<String, TransportPolicyError> {
    let file = std::fs::File::open(path).map_err(|_| TransportPolicyError::Malformed)?;
    let mut source = String::new();
    file.take((MAX_CONFIG_BYTES + 1) as u64)
        .read_to_string(&mut source)
        .map_err(|_| TransportPolicyError::Malformed)?;
    if source.len() > MAX_CONFIG_BYTES {
        return Err(TransportPolicyError::Malformed);
    }
    Ok(source)
}

fn valid_loopback_endpoint(endpoint: &str) -> bool {
    let Some(address) = endpoint.strip_prefix("tcp/") else {
        return false;
    };
    let address = address.split(['#', '?']).next().unwrap_or_default();
    let Some((host, port)) = address.rsplit_once(':') else {
        return false;
    };
    if !port.parse::<u16>().is_ok_and(|parsed| parsed > 0) {
        return false;
    }
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host == "localhost" || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn file_exists(config: &Value, key: &str, private: bool) -> bool {
    let Some(filename) = config.get(key).and_then(Value::as_str) else {
        return false;
    };
    let path = Path::new(filename);
    if !path.is_absolute() {
        return false;
    }
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.file_type().is_file() || meta.len() == 0 {
        return false;
    }
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return false;
        }
    }
    #[cfg(not(unix))]
    let _ = private;
    true
}

/// Validate JSON5 text *before* passing the same text to Zenoh's parser.
/// This intentionally rejects implicit/default discovery or transports,
/// including mixed TLS/plaintext protocol whitelists.
pub fn validate_zenoh_config(
    source: &str,
    allow_insecure_loopback: bool,
) -> Result<(), TransportPolicyError> {
    if source.len() > MAX_CONFIG_BYTES {
        return Err(TransportPolicyError::Malformed);
    }
    let root: Value = json5::from_str(source).map_err(|_| TransportPolicyError::Malformed)?;
    let mode = root
        .get("mode")
        .and_then(Value::as_str)
        .ok_or(TransportPolicyError::Unsafe)?;
    if !matches!(mode, "peer" | "router" | "client") {
        return Err(TransportPolicyError::Unsafe);
    }
    let multicast = root.pointer("/scouting/multicast/enabled") == Some(&Value::Bool(false));
    let gossip = root.pointer("/scouting/gossip/enabled") == Some(&Value::Bool(false));
    if !multicast || !gossip {
        return Err(TransportPolicyError::Unsafe);
    }
    let protocols = root
        .pointer("/transport/link/protocols")
        .and_then(Value::as_array)
        .ok_or(TransportPolicyError::Unsafe)?;
    if protocols.len() != 1 {
        return Err(TransportPolicyError::Unsafe);
    }
    let scheme = protocols[0].as_str().ok_or(TransportPolicyError::Unsafe)?;
    if scheme != "tls" && scheme != "tcp" {
        return Err(TransportPolicyError::Unsafe);
    }

    let mut endpoints = Vec::new();
    for field in ["/listen/endpoints", "/connect/endpoints"] {
        if let Some(raw) = root.pointer(field) {
            let items = raw.as_array().ok_or(TransportPolicyError::Unsafe)?;
            for item in items {
                endpoints.push(item.as_str().ok_or(TransportPolicyError::Unsafe)?);
            }
        }
    }
    if endpoints.is_empty() || endpoints.len() > 16 {
        return Err(TransportPolicyError::Unsafe);
    }
    if mode == "client" && root.pointer("/connect/endpoints").is_none() {
        return Err(TransportPolicyError::Unsafe);
    }
    if scheme == "tcp" {
        if !allow_insecure_loopback || !endpoints.iter().all(|ep| valid_loopback_endpoint(ep)) {
            return Err(TransportPolicyError::Unsafe);
        }
        return Ok(());
    }
    if !endpoints.iter().all(|ep| ep.starts_with("tls/")) {
        return Err(TransportPolicyError::Unsafe);
    }

    let tls = root
        .pointer("/transport/link/tls")
        .ok_or(TransportPolicyError::Unsafe)?;
    if tls.get("enable_mtls") != Some(&Value::Bool(true))
        || tls.get("verify_name_on_connect") != Some(&Value::Bool(true))
        || tls.get("close_link_on_expiration") != Some(&Value::Bool(true))
        || !file_exists(tls, "root_ca_certificate", false)
    {
        return Err(TransportPolicyError::Unsafe);
    }
    if root
        .pointer("/listen/endpoints")
        .and_then(Value::as_array)
        .is_some_and(|v| !v.is_empty())
        && (!file_exists(tls, "listen_private_key", true)
            || !file_exists(tls, "listen_certificate", false))
    {
        return Err(TransportPolicyError::Unsafe);
    }
    if root
        .pointer("/connect/endpoints")
        .and_then(Value::as_array)
        .is_some_and(|v| !v.is_empty())
        && (!file_exists(tls, "connect_private_key", true)
            || !file_exists(tls, "connect_certificate", false))
    {
        return Err(TransportPolicyError::Unsafe);
    }
    Ok(())
}
