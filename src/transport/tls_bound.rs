//! Opt-in transport-bound RPC on a **dedicated** TLS 1.3 TCP listener.
//!
//! Zenoh's query API does not expose verified TLS peer certificates. This
//! adapter deliberately uses a separate TLS ingress, with no Zenoh queryable
//! fallback, so the presented certificate comes from the EXACT TLS session
//! that carries the signed RPC request. Protocol is one request/connection.
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::rpc::{RpcAction, RpcClient, RpcGateway, RpcReply, MAX_FRAME_BYTES};

const ALPN: &[u8] = b"ocaf-rpc/1";
const LIMIT: Duration = Duration::from_secs(8);
const MAX_TRUST_PEM_BYTES: usize = 128 * 1024;
const MAX_LEAF_PEM_BYTES: usize = 32 * 1024;
const MAX_KEY_PEM_BYTES: usize = 16 * 1024;

fn read_chain(pem: &[u8], limit: usize) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    anyhow::ensure!(
        !pem.is_empty() && pem.len() <= limit,
        "TLS certificate material out of bounds"
    );
    let certificates = CertificateDer::pem_slice_iter(pem)
        .collect::<Result<Vec<_>, _>>()?;
    anyhow::ensure!(
        !certificates.is_empty() && certificates.len() <= 16,
        "TLS certificate chain invalid"
    );
    Ok(certificates)
}

fn private_key(pem: &[u8]) -> anyhow::Result<PrivateKeyDer<'static>> {
    anyhow::ensure!(
        !pem.is_empty() && pem.len() <= MAX_KEY_PEM_BYTES,
        "TLS private key out of bounds"
    );
    Ok(PrivateKeyDer::from_pem_slice(pem)?)
}

fn roots(pem: &[u8]) -> anyhow::Result<RootCertStore> {
    let mut roots = RootCertStore::empty();
    for certificate in read_chain(pem, MAX_TRUST_PEM_BYTES)? {
        roots.add(certificate)?;
    }
    Ok(roots)
}

/// Trust and EKU are verified by rustls WebPki before any RPC bytes are
/// consumed. Client certificates are mandatory, never optional. TLS1.3 only.
pub fn server_config(
    server_cert_pem: &[u8],
    server_key_pem: &[u8],
    trusted_client_ca_pem: &[u8],
) -> anyhow::Result<ServerConfig> {
    let verifier =
        rustls::server::WebPkiClientVerifier::builder(Arc::new(roots(trusted_client_ca_pem)?))
            .build()?;
    let mut config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            read_chain(server_cert_pem, MAX_LEAF_PEM_BYTES)?,
            private_key(server_key_pem)?,
        )?;
    config.alpn_protocols = vec![ALPN.to_vec()];
    Ok(config)
}

/// Client must authenticate the server name and CA and present its own
/// clientAuth certificate. This is required for the alternate TCP ingress.
pub fn client_config(
    client_cert_pem: &[u8],
    client_key_pem: &[u8],
    trusted_server_ca_pem: &[u8],
) -> anyhow::Result<ClientConfig> {
    let mut config = ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_root_certificates(roots(trusted_server_ca_pem)?)
        .with_client_auth_cert(
            read_chain(client_cert_pem, MAX_LEAF_PEM_BYTES)?,
            private_key(client_key_pem)?,
        )?;
    config.alpn_protocols = vec![ALPN.to_vec()];
    Ok(config)
}

pub struct TlsBoundRpcServer {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    gateway: Arc<RpcGateway>,
}

impl TlsBoundRpcServer {
    pub fn new(
        listener: TcpListener,
        config: ServerConfig,
        gateway: Arc<RpcGateway>,
    ) -> anyhow::Result<Self> {
        gateway.validate_tls_bound_peers()?;
        Ok(Self {
            listener,
            acceptor: TlsAcceptor::from(Arc::new(config)),
            gateway,
        })
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        let ingress = Arc::new(Semaphore::new(64));
        loop {
            let (tcp, _) = self.listener.accept().await?;
            let permit = match Arc::clone(&ingress).try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => continue,
            };
            let acceptor = self.acceptor.clone();
            let gateway = Arc::clone(&self.gateway);
            tokio::spawn(async move {
                let _permit = permit;
                if let Err(error) =
                    tokio::time::timeout(LIMIT, serve_connection(tcp, acceptor, gateway)).await
                {
                    tracing::warn!(error = %error, "bounded TLS-bound RPC connection timed out");
                }
            });
        }
    }
}

async fn serve_connection(tcp: TcpStream, acceptor: TlsAcceptor, gateway: Arc<RpcGateway>) {
    let Ok(mut connection) = acceptor.accept(tcp).await else {
        return;
    };
    let (_tcp, tls) = connection.get_ref();
    if tls.alpn_protocol() != Some(ALPN) {
        return;
    }
    let Some(leaf) = tls.peer_certificates().and_then(|certs| certs.first()) else {
        return;
    };
    // DER was extracted from the *verified* rustls handshake connection,
    // never from the HMAC payload, HTTP header, or client-provided text.
    let verified_leaf = leaf.as_ref().to_vec();
    let Ok(length) = connection.read_u32().await else {
        return;
    };
    if length == 0 || length as usize > MAX_FRAME_BYTES {
        return;
    }
    let mut bytes = vec![0u8; length as usize];
    if connection.read_exact(&mut bytes).await.is_err() {
        return;
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let response = gateway
        .handle_verified_tls_at(&bytes, &verified_leaf, now_ms)
        .await;
    let (status, reply) = match response {
        Ok(reply) => (0u8, reply),
        Err(_) => (1u8, b"rejected".to_vec()),
    };
    if connection.write_u8(status).await.is_ok()
        && connection.write_u32(reply.len() as u32).await.is_ok()
    {
        let _ = connection.write_all(&reply).await;
        let _ = connection.flush().await;
    }
}

/// Single request over a fully authenticated and peer-name-verified mTLS
/// connection. Retries of stateful actions are deliberately not automatic.
pub async fn request(
    address: SocketAddr,
    remote_server_dns: &str,
    config: Arc<ClientConfig>,
    client: &RpcClient,
    action: RpcAction,
) -> anyhow::Result<RpcReply> {
    tokio::time::timeout(LIMIT, async {
        let socket = TcpStream::connect(address).await?;
        let server_name = ServerName::try_from(remote_server_dns.to_string())?;
        let connector = TlsConnector::from(config);
        let mut tls = connector.connect(server_name, socket).await?;
        anyhow::ensure!(
            tls.get_ref().1.alpn_protocol() == Some(ALPN),
            "unrecognized TLS application protocol"
        );
        let (req, bytes) = client.prepare(action, 5_000)?;
        anyhow::ensure!(bytes.len() <= MAX_FRAME_BYTES, "RPC frame oversized");
        tls.write_u32(bytes.len() as u32).await?;
        tls.write_all(&bytes).await?;
        tls.flush().await?;
        let status = tls.read_u8().await?;
        let len = tls.read_u32().await? as usize;
        anyhow::ensure!(
            len <= MAX_FRAME_BYTES && len > 0,
            "RPC reply length invalid"
        );
        let mut response = vec![0; len];
        tls.read_exact(&mut response).await?;
        anyhow::ensure!(status == 0, "remote RPC rejected");
        client.read_reply(&response, &req).map_err(Into::into)
    })
    .await?
}
