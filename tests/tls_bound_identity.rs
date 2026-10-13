#![cfg(all(feature = "tls-bound", unix))]
//! Real TLS 1.3 connections with distinct client certificates and HMAC keys.
//! An authentic signed RPC request cannot claim a different TLS peer identity.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use ocaf_runtime::rpc::{PeerPolicy, RpcAction, RpcClient, RpcGateway, RpcStatus};
use ocaf_runtime::jobs::remote::RemoteJobWorker;
use ocaf_runtime::transport::{pricing_tool_manifest, EchoTool};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::tls_bound::{
    client_config, request, server_config, TlsBoundRpcServer,
};
use sha2::{Digest, Sha256};

const HMAC_KEY: [u8; 32] = [0x37; 32];

struct TestPki {
    _dir: tempfile::TempDir,
    root: PathBuf,
    alien_root: PathBuf,
    server: PathBuf,
    server_key: PathBuf,
    client: PathBuf,
    client_key: PathBuf,
    other: PathBuf,
    other_key: PathBuf,
    alien: PathBuf,
    alien_key: PathBuf,
}

fn openssl(args: &[&str]) {
    let output = Command::new("openssl")
        .args(args)
        .output()
        .expect("OpenSSL available");
    assert!(
        output.status.success(),
        "PKI generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn path(path: &Path) -> &str {
    path.to_str().expect("temp path UTF-8")
}

fn ca(dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    let cert = dir.join(format!("{name}.pem"));
    let key = dir.join(format!("{name}.key"));
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "ec",
        "-pkeyopt",
        "ec_paramgen_curve:P-256",
        "-nodes",
        "-keyout",
        path(&key),
        "-out",
        path(&cert),
        "-days",
        "2",
        "-subj",
        &format!("/CN={name}"),
        "-addext",
        "basicConstraints=critical,CA:TRUE",
        "-addext",
        "keyUsage=critical,keyCertSign,cRLSign",
    ]);
    (cert, key)
}

fn sign(
    dir: &Path,
    name: &str,
    issuer: &Path,
    issuer_key: &Path,
    server: bool,
) -> (PathBuf, PathBuf) {
    let cert = dir.join(format!("{name}.pem"));
    let key = dir.join(format!("{name}.key"));
    let csr = dir.join(format!("{name}.csr"));
    let ext = dir.join(format!("{name}.ext"));
    openssl(&[
        "req",
        "-new",
        "-newkey",
        "ec",
        "-pkeyopt",
        "ec_paramgen_curve:P-256",
        "-nodes",
        "-keyout",
        path(&key),
        "-out",
        path(&csr),
        "-subj",
        &format!("/CN={name}"),
    ]);
    let text = if server {
        "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost\n"
    } else {
        match name {
            "agent-a" => "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\nsubjectAltName=DNS:agent-a.test\n",
            "agent-b" => "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\nsubjectAltName=DNS:agent-b.test\n",
            _ => "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\nsubjectAltName=DNS:alien.test\n",
        }
    };
    std::fs::write(&ext, text).unwrap();
    openssl(&[
        "x509",
        "-req",
        "-in",
        path(&csr),
        "-CA",
        path(issuer),
        "-CAkey",
        path(issuer_key),
        "-CAcreateserial",
        "-out",
        path(&cert),
        "-days",
        "2",
        "-sha256",
        "-extfile",
        path(&ext),
    ]);
    (cert, key)
}

fn fixture() -> TestPki {
    let dir = tempfile::tempdir().unwrap();
    let (root, root_key) = ca(dir.path(), "trusted-root");
    let (alien_root, alien_root_key) = ca(dir.path(), "rogue-root");
    let (server, server_key) = sign(dir.path(), "server", &root, &root_key, true);
    let (client, client_key) = sign(dir.path(), "agent-a", &root, &root_key, false);
    let (other, other_key) = sign(dir.path(), "agent-b", &root, &root_key, false);
    let (alien, alien_key) = sign(dir.path(), "alien", &alien_root, &alien_root_key, false);
    TestPki {
        _dir: dir,
        root,
        alien_root,
        server,
        server_key,
        client,
        client_key,
        other,
        other_key,
        alien,
        alien_key,
    }
}

fn material(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

fn pin(cert: &Path) -> String {
    let openssl = openssl::x509::X509::from_pem(&material(cert)).unwrap();
    format!("{:x}", Sha256::digest(openssl.to_der().unwrap()))
}

async fn start(
    pki: &TestPki,
) -> (
    std::net::SocketAddr,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let peer = PeerPolicy::new(HMAC_KEY, vec![])
        .with_pinned_certificate(&material(&pki.client), &pin(&pki.client))
        .unwrap()
        .with_tls_dns_identity("agent-a.test")
        .unwrap();
    let gateway = RpcGateway::new(
        "coordinator",
        OcafRuntime::new(0),
        HashMap::from([("agent-a".into(), peer)]),
        8,
    )
    .unwrap();
    let config = server_config(
        &material(&pki.server),
        &material(&pki.server_key),
        &material(&pki.root),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = TlsBoundRpcServer::new(listener, config, Arc::new(gateway)).unwrap();
    let task = tokio::spawn(async move { server.serve().await });
    (address, task)
}

fn client(
    pki: &TestPki,
    cert: &Path,
    key: &Path,
    server_ca: &Path,
    proof: &Path,
) -> (Arc<rustls::ClientConfig>, RpcClient) {
    let tls = client_config(&material(cert), &material(key), &material(server_ca)).unwrap();
    let rpc = RpcClient::new("agent-a", "coordinator", HMAC_KEY)
        .unwrap()
        .with_certificate_private_key(material(proof))
        .unwrap();
    let _ = pki;
    (Arc::new(tls), rpc)
}

#[tokio::test]
async fn active_tls_leaf_matches_sender_pin_and_signed_rpc() {
    let pki = fixture();
    let (address, task) = start(&pki).await;
    let (tls, rpc) = client(
        &pki,
        &pki.client,
        &pki.client_key,
        &pki.root,
        &pki.client_key,
    );
    let reply = request(address, "localhost", tls, &rpc, RpcAction::Discover)
        .await
        .unwrap();
    assert_eq!(reply.status, RpcStatus::Ok);
    task.abort();
}

#[tokio::test]
async fn valid_hmac_and_pop_with_different_tls_cert_must_be_denied() {
    let pki = fixture();
    let (address, task) = start(&pki).await;
    let (tls, rpc) = client(&pki, &pki.other, &pki.other_key, &pki.root, &pki.client_key);
    assert!(
        request(address, "localhost", tls, &rpc, RpcAction::Discover)
            .await
            .is_err()
    );
    task.abort();
}

#[tokio::test]
async fn untrusted_ca_certificate_rejected_before_signed_rpc_dispatch() {
    let pki = fixture();
    let (address, task) = start(&pki).await;
    let (tls, rpc) = client(&pki, &pki.alien, &pki.alien_key, &pki.root, &pki.client_key);
    assert!(
        request(address, "localhost", tls, &rpc, RpcAction::Discover)
            .await
            .is_err()
    );
    task.abort();
}

#[tokio::test]
async fn wrong_server_hostname_rejected_even_with_valid_client_identity() {
    let pki = fixture();
    let (address, task) = start(&pki).await;
    let (tls, rpc) = client(
        &pki,
        &pki.client,
        &pki.client_key,
        &pki.root,
        &pki.client_key,
    );
    assert!(
        request(address, "evil.example", tls, &rpc, RpcAction::Discover)
            .await
            .is_err()
    );
    task.abort();
}

#[tokio::test]
async fn no_unpinned_legacy_hmac_peer_can_enable_tls_bound_listener() {
    let pki = fixture();
    let gateway = RpcGateway::new(
        "coordinator",
        OcafRuntime::new(0),
        HashMap::from([("agent-a".into(), PeerPolicy::new(HMAC_KEY, vec![]))]),
        8,
    )
    .unwrap();
    let config = server_config(
        &material(&pki.server),
        &material(&pki.server_key),
        &material(&pki.root),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    assert!(TlsBoundRpcServer::new(listener, config, Arc::new(gateway)).is_err());
}

#[test]
fn wrong_dns_san_rejected_before_server_start() {
    let pki = fixture();
    let peer = PeerPolicy::new(HMAC_KEY, vec![])
        .with_pinned_certificate(&material(&pki.client), &pin(&pki.client))
        .unwrap();
    assert!(peer.with_tls_dns_identity("agent-b.test").is_err());
    assert!(client_config(
        &material(&pki.client),
        &material(&pki.client_key),
        &material(&pki.alien_root)
    )
    .is_ok());
}

#[tokio::test]
async fn tls_bound_worker_heartbeats_and_polls_through_actual_mtls_session() {
    let pki = fixture();
    let peer = PeerPolicy::new(HMAC_KEY, vec!["demo_echo".into()])
        .allow_job_worker()
        .with_pinned_certificate(&material(&pki.client), &pin(&pki.client))
        .expect("pinned worker certificate")
        .with_tls_dns_identity("agent-a.test")
        .expect("worker SAN");

    let directory = pki._dir.path().join("directory.db");
    let jobs = pki._dir.path().join("jobs.db");
    let gateway = RpcGateway::new(
        "coordinator",
        OcafRuntime::new(0),
        HashMap::from([("agent-a".to_owned(), peer)]),
        8,
    ).expect("gateway")
    .with_coordinator_db(&directory).expect("directory")
    .with_job_db(&jobs).expect("jobs");

    let server_tls = server_config(
        &material(&pki.server),
        &material(&pki.server_key),
        &material(&pki.root),
    ).expect("server certificate");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listen");
    let address = listener.local_addr().expect("address");
    let server = TlsBoundRpcServer::new(listener, server_tls, Arc::new(gateway))
        .expect("mTLS ingress");
    let task = tokio::spawn(async move { server.serve().await });

    let (client_tls, protocol) = client(
        &pki, &pki.client, &pki.client_key, &pki.root, &pki.client_key,
    );
    let runtime = OcafRuntime::new(0);
    runtime.register(pricing_tool_manifest(), EchoTool).await.expect("tool");
    let worker = RemoteJobWorker::new_tls_bound(
        protocol,
        address,
        "localhost".into(),
        client_tls,
        runtime,
        HashSet::from(["demo_echo".into()]),
    ).expect("trusted worker");
    worker.heartbeat().await.expect("mTLS heartbeat and PoP");
    assert_eq!(worker.poll_once().await.expect("empty queue"), None);
    task.abort();
}
