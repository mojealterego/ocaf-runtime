//! Stage 0.43: transport-authenticated OMAs admission boundary.
//! A peer's identity comes exclusively from its verified TLS client certificate,
//! never from a JSON request or a caller-provided sender string.
use crate::omas_server::{AdmissionDecision, AgentRegistrationRequest};
use crate::omas_signed::SignedOperation;
use crate::stage043_agent_store::Stage043AgentStore;
use crate::stage043_signed_agent::{
    admit_signed_registration_persistent_for_sender, SignedAgentError,
};
use openssl::sha::sha256;
use openssl::ssl::{SslStream, SslVerifyMode};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::TcpStream;
use std::path::Path;

#[derive(Clone)]
pub struct TrustedPeer {
    pub sender: String,
    pub signing_key: [u8; 32],
}

/// Operator-provisioned exact SHA-256 DER certificate fingerprint -> identity.
/// The TLS acceptor MUST require PEER | FAIL_IF_NO_PEER_CERT and verify a trusted
/// certificate chain; fingerprint pinning is an additional identity binding.
pub struct PeerRegistry {
    peers: HashMap<[u8; 32], TrustedPeer>,
}

impl PeerRegistry {
    pub fn new(peers: HashMap<[u8; 32], TrustedPeer>) -> Result<Self, String> {
        if peers.is_empty()
            || peers.values().any(|p| {
                p.sender.is_empty()
                    || p.sender.len() > 128
                    || !p
                        .sender
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            })
        {
            return Err("invalid or empty operator-provisioned peer registry".into());
        }
        Ok(Self { peers })
    }

    pub fn resolve(&self, stream: &SslStream<TcpStream>) -> Result<&TrustedPeer, SignedAgentError> {
        let ssl = stream.ssl();
        if ssl.verify_result() != openssl::x509::X509VerifyResult::OK
            || !ssl.verify_mode().contains(SslVerifyMode::PEER)
            || !ssl
                .verify_mode()
                .contains(SslVerifyMode::FAIL_IF_NO_PEER_CERT)
        {
            return Err(SignedAgentError::Binding);
        }
        let certificate = ssl.peer_certificate().ok_or(SignedAgentError::Binding)?;
        let der = certificate
            .to_der()
            .map_err(|_| SignedAgentError::Binding)?;
        self.peers
            .get(&sha256(&der))
            .ok_or(SignedAgentError::Binding)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedAgentRegistrationEnvelope {
    pub request: AgentRegistrationRequest,
    pub operation: SignedOperation,
}

/// Invoke only after OpenSSL has successfully completed a mutually authenticated
/// TLS handshake. The peer certificate and its verification result are read from
/// the live session. An untrusted payload cannot select its signing key or sender.
pub fn admit_from_mtls_session(
    peers: &PeerRegistry,
    session: &SslStream<TcpStream>,
    store: &Stage043AgentStore,
    envelope: SignedAgentRegistrationEnvelope,
    replay_db: &Path,
    now: i64,
) -> Result<AdmissionDecision, SignedAgentError> {
    let peer = peers.resolve(session)?;
    admit_signed_registration_persistent_for_sender(
        store,
        envelope.request,
        &envelope.operation,
        &peer.sender,
        &peer.signing_key,
        replay_db,
        now,
    )
}
