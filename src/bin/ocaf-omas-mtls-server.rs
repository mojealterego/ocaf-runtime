//! Standalone, fail-closed OMAs mTLS registration listener.
//! Operator must provision CA, server certificate/key, SQLite files and a pinned
//! client certificate fingerprint mapped to an Ed25519 registration signing key.
use ocaf_runtime::stage043_agent_store::Stage043AgentStore;
use ocaf_runtime::stage043_mtls_admission::{
    admit_from_mtls_session, PeerRegistry, SignedAgentRegistrationEnvelope, TrustedPeer,
};
use ocaf_runtime::omas_server::OmasServerConfig;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslVerifyMode};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("missing operator configuration: {name}"))
}

fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 {
        return Err("invalid hex length".into());
    }
    let mut bytes = [0u8; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "invalid hex")?;
    }
    Ok(bytes)
}

fn serve() -> Result<(), String> {
    let bind = required("OCAF_OMAS_MTLS_BIND")?;
    let ca = required("OCAF_OMAS_MTLS_CA")?;
    let cert = required("OCAF_OMAS_MTLS_CERT")?;
    let key = required("OCAF_OMAS_MTLS_KEY")?;
    let db = PathBuf::from(required("OCAF_OMAS_AGENT_DB")?);
    let nonce_db = PathBuf::from(required("OCAF_OMAS_NONCE_DB")?);
    let fingerprint = decode_hex::<32>(&required("OCAF_OMAS_PEER_CERT_SHA256")?)?;
    let signing_key = decode_hex::<32>(&required("OCAF_OMAS_PEER_ED25519_PUBKEY")?)?;
    let sender = required("OCAF_OMAS_PEER_SENDER")?;
    let mut entries = HashMap::new();
    entries.insert(sender.clone(), signing_key);
    let mut pinned = HashMap::new();
    pinned.insert(fingerprint, TrustedPeer { sender, signing_key });
    let peers = PeerRegistry::new(pinned)?;
    let store = Stage043AgentStore::open(&db, OmasServerConfig::default())?;
    if !nonce_db.is_file() {
        return Err("operator-provisioned nonce SQLite file required".into());
    }
    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())
        .map_err(|e| e.to_string())?;
    builder.set_certificate_chain_file(cert).map_err(|e| e.to_string())?;
    builder.set_private_key_file(key, SslFiletype::PEM).map_err(|e| e.to_string())?;
    builder.check_private_key().map_err(|e| e.to_string())?;
    builder.set_ca_file(ca).map_err(|e| e.to_string())?;
    builder.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
    let acceptor = builder.build();
    let listener = TcpListener::bind(&bind).map_err(|e| e.to_string())?;
    eprintln!("OMAs mTLS admission listening on {bind}");
    for connection in listener.incoming() {
        let socket = match connection {
            Ok(socket) => socket,
            Err(error) => {
                eprintln!("accept failed: {error}");
                continue;
            }
        };
        let _ = socket.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = socket.set_write_timeout(Some(Duration::from_secs(5)));
        let mut tls = match acceptor.accept(socket) {
            Ok(tls) => tls,
            Err(error) => {
                eprintln!("TLS handshake rejected: {error}");
                continue;
            }
        };
        let mut line = Vec::new();
        let read = BufReader::new(&mut tls).take(65537).read_until(b'\n', &mut line);
        let response = match read {
            Ok(_) if line.len() <= 65536 && line.ends_with(b"\n") => {
                match serde_json::from_slice::<SignedAgentRegistrationEnvelope>(&line) {
                    Ok(envelope) => {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        match admit_from_mtls_session(
                            &peers, &tls, &store, envelope, &nonce_db, now,
                        ) {
                            Ok(decision) => json!({"ok": true, "decision": decision}),
                            Err(error) => json!({"ok": false, "error": error.to_string()}),
                        }
                    }
                    Err(_) => json!({"ok": false, "error": "invalid envelope"}),
                }
            }
            _ => json!({"ok": false, "error": "invalid or oversized request"}),
        };
        let _ = writeln!(tls, "{response}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = serve() {
        eprintln!("OMAs mTLS admission unavailable: {error}");
        std::process::exit(2);
    }
}
