#![cfg(all(feature = "zenoh", unix))]
//! Real mutual-TLS handshake integration tests for Zenoh 1.10.
//! Certificates are generated ephemerally using OpenSSL; no signing private
//! keys or test credentials are checked into the repository.
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::{ZenohRpcClient, ZenohRpcServer};
use ocaf_runtime::transport_policy::validate_zenoh_config;
use serde_json::json;

const HMAC_KEY: [u8; 32] = [0x39; 32];

fn openssl(args: &[&str]) -> anyhow::Result<()> {
    let output = Command::new("openssl").args(args).output()?;
    anyhow::ensure!(
        output.status.success(),
        "openssl command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn file(path: &Path) -> &str {
    path.to_str().expect("ASCII temp paths")
}

struct TestPki {
    _dir: tempfile::TempDir,
    ca: PathBuf,
    other_ca: PathBuf,
    server_key: PathBuf,
    server_cert: PathBuf,
    client_key: PathBuf,
    client_cert: PathBuf,
    rogue_key: PathBuf,
    rogue_cert: PathBuf,
}

fn make_ca(dir: &Path, name: &str) -> anyhow::Result<(PathBuf, PathBuf)> {
    let ca = dir.join(format!("{name}.pem"));
    let key = dir.join(format!("{name}-key.pem"));
    openssl(&[
        "req",
        "-new",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-sha256",
        "-days",
        "2",
        "-keyout",
        file(&key),
        "-out",
        file(&ca),
        "-subj",
        &format!("/CN=ocaf-{name}"),
        "-addext",
        "basicConstraints=critical,CA:TRUE",
    ])?;
    Ok((ca, key))
}

fn sign(
    dir: &Path,
    name: &str,
    ca: &Path,
    ca_key: &Path,
    server: bool,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    let key = dir.join(format!("{name}-key.pem"));
    let csr = dir.join(format!("{name}.csr"));
    let cert = dir.join(format!("{name}.pem"));
    let extensions = dir.join(format!("{name}-extensions.cnf"));
    openssl(&[
        "req",
        "-new",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        file(&key),
        "-out",
        file(&csr),
        "-subj",
        &format!("/CN=ocaf-{name}"),
    ])?;
    let extensions_text = if server {
        "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=IP:127.0.0.1\n"
    } else {
        "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\nsubjectAltName=DNS:ocaf-test-client\n"
    };
    std::fs::write(&extensions, extensions_text)?;
    openssl(&[
        "x509",
        "-req",
        "-in",
        file(&csr),
        "-CA",
        file(ca),
        "-CAkey",
        file(ca_key),
        "-CAcreateserial",
        "-out",
        file(&cert),
        "-days",
        "2",
        "-sha256",
        "-extfile",
        file(&extensions),
    ])?;
    Ok((key, cert))
}

fn pki() -> anyhow::Result<TestPki> {
    let dir = tempfile::tempdir()?;
    let (ca, ca_key) = make_ca(dir.path(), "trusted-ca")?;
    let (other_ca, other_key) = make_ca(dir.path(), "untrusted-ca")?;
    let (server_key, server_cert) = sign(dir.path(), "coordinator", &ca, &ca_key, true)?;
    let (client_key, client_cert) = sign(dir.path(), "trusted-client", &ca, &ca_key, false)?;
    let (rogue_key, rogue_cert) = sign(dir.path(), "rogue-client", &other_ca, &other_key, false)?;
    Ok(TestPki {
        _dir: dir,
        ca,
        other_ca,
        server_key,
        server_cert,
        client_key,
        client_cert,
        rogue_key,
        rogue_cert,
    })
}

fn config(
    port: u16,
    pki: &TestPki,
    server: bool,
    rogue_client: bool,
    wrong_ca: bool,
) -> anyhow::Result<zenoh::Config> {
    let endpoint = format!("tls/127.0.0.1:{port}");
    let ca = if wrong_ca { &pki.other_ca } else { &pki.ca };
    let (key, cert) = if rogue_client {
        (&pki.rogue_key, &pki.rogue_cert)
    } else {
        (&pki.client_key, &pki.client_cert)
    };
    let mut transport = json!({
        "root_ca_certificate": file(ca),
        "enable_mtls": true,
        "verify_name_on_connect": true,
        "close_link_on_expiration": true,
    });
    if server {
        transport["listen_private_key"] = json!(file(&pki.server_key));
        transport["listen_certificate"] = json!(file(&pki.server_cert));
    } else {
        transport["connect_private_key"] = json!(file(key));
        transport["connect_certificate"] = json!(file(cert));
    }
    let source = serde_json::to_string(&json!({
        "mode": if server { "peer" } else { "client" },
        "scouting": {
            "multicast": { "enabled": false },
            "gossip": { "enabled": false },
        },
        "transport": { "link": { "protocols": ["tls"], "tls": transport } },
        "listen": { "endpoints": if server {vec![endpoint.clone()]} else {vec![]} },
        "connect": { "endpoints": if server {vec![]} else {vec![endpoint]} },
    }))?;
    validate_zenoh_config(&source, false)?;
    zenoh::Config::from_json5(&source)
        .map_err(|error| anyhow::anyhow!("Zenoh mTLS config: {error}"))
}

fn available_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trusted_mutual_tls_and_signed_rpc_round_trip() -> anyhow::Result<()> {
    let certs = pki()?;
    let port = available_port()?;
    let server = Arc::new(
        zenoh::open(config(port, &certs, true, false, false)?)
            .await
            .map_err(|e| anyhow::anyhow!("open TLS coordinator: {e}"))?,
    );
    let gateway = Arc::new(RpcGateway::new(
        "coord",
        OcafRuntime::new(0),
        HashMap::from([("client".to_string(), PeerPolicy::new(HMAC_KEY, vec![]))]),
        8,
    )?);
    let service = ZenohRpcServer::new(server, gateway);
    let handle = tokio::spawn(async move { service.serve().await });
    tokio::time::sleep(Duration::from_millis(200)).await;

    let client = Arc::new(
        zenoh::open(config(port, &certs, false, false, false)?)
            .await
            .map_err(|e| anyhow::anyhow!("trusted client failed mTLS: {e}"))?,
    );
    let protocol = RpcClient::new("client", "coord", HMAC_KEY)?;
    let reply = ZenohRpcClient::new(client, protocol)
        .request(RpcAction::Discover)
        .await?;
    assert_eq!(reply.status, RpcStatus::Ok);
    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn untrusted_ca_and_rogue_client_certificate_are_rejected() -> anyhow::Result<()> {
    let certs = pki()?;
    let port = available_port()?;
    let server = zenoh::open(config(port, &certs, true, false, false)?)
        .await
        .map_err(|e| anyhow::anyhow!("TLS server failed: {e}"))?;

    // A real handshake must reject a valid-looking client with an
    // unrelated signing CA even though the configuration passes preflight.
    let rogue = tokio::time::timeout(
        Duration::from_secs(6),
        zenoh::open(config(port, &certs, false, true, false)?),
    )
    .await?;
    assert!(
        rogue.is_err(),
        "server must reject a rogue client certificate"
    );

    // Likewise an otherwise valid client must reject a server chain that
    // does not terminate in its configured trusted CA.
    let wrong_trust = tokio::time::timeout(
        Duration::from_secs(6),
        zenoh::open(config(port, &certs, false, false, true)?),
    )
    .await?;
    assert!(wrong_trust.is_err(), "client must reject an untrusted CA");
    drop(server);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trusted_ca_with_wrong_server_ip_san_is_rejected() -> anyhow::Result<()> {
    let mut certs = pki()?;
    let dir = certs._dir.path();
    let wrong_extensions = dir.join("wrong-ip-extensions.cnf");
    let wrong_cert = dir.join("wrong-ip-cert.pem");
    std::fs::write(
        &wrong_extensions,
        "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=IP:127.0.0.2\n",
    )?;
    let ca_key = dir.join("trusted-ca-key.pem");
    let server_csr = dir.join("coordinator.csr");
    openssl(&[
        "x509",
        "-req",
        "-in",
        file(&server_csr),
        "-CA",
        file(&certs.ca),
        "-CAkey",
        file(&ca_key),
        "-CAcreateserial",
        "-out",
        file(&wrong_cert),
        "-days",
        "2",
        "-sha256",
        "-extfile",
        file(&wrong_extensions),
    ])?;
    certs.server_cert = wrong_cert;
    let port = available_port()?;
    let listener = zenoh::open(config(port, &certs, true, false, false)?)
        .await
        .map_err(|e| anyhow::anyhow!("bad SAN test server failed to start: {e}"))?;
    let mismatched = tokio::time::timeout(
        Duration::from_secs(6),
        zenoh::open(config(port, &certs, false, false, false)?),
    )
    .await?;
    assert!(
        mismatched.is_err(),
        "a trusted CA must not bypass server IP subject-alternative-name verification"
    );
    drop(listener);
    Ok(())
}
