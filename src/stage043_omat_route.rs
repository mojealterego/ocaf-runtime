//! Stage 0.43 point 2: fail-closed OMAs -> OMAt route authorization.
//! Routing authorization is independent of transport: the caller MUST verify
//! the peer using mutual TLS before creating an AuthenticatedPeer.
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedPeer {
    pub principal: String,
    pub tenant: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteError {
    Disconnected,
    Unauthorized,
    InvalidTopic,
}

pub struct OmatRouteGuard {
    peers: HashMap<String, String>,
    connected: bool,
}

impl OmatRouteGuard {
    pub fn new(peers: HashMap<String, String>) -> Result<Self, RouteError> {
        if peers.is_empty()
            || peers
                .iter()
                .any(|(principal, tenant)| !valid_segment(principal) || !valid_segment(tenant))
        {
            return Err(RouteError::Unauthorized);
        }
        Ok(Self {
            peers,
            connected: false,
        })
    }

    pub fn set_transport_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn authorize(&self, peer: &AuthenticatedPeer, topic: &str) -> Result<(), RouteError> {
        if !self.connected {
            return Err(RouteError::Disconnected);
        }
        if self.peers.get(&peer.principal) != Some(&peer.tenant) {
            return Err(RouteError::Unauthorized);
        }
        let mut parts = topic.split('/');
        if parts.next() != Some("ocaf")
            || parts.next() != Some("omat")
            || parts.next() != Some(peer.tenant.as_str())
            || parts.next() != Some("requests")
        {
            return Err(RouteError::InvalidTopic);
        }
        let request_id = parts.next().ok_or(RouteError::InvalidTopic)?;
        if !valid_segment(request_id) || parts.next().is_some() {
            return Err(RouteError::InvalidTopic);
        }
        Ok(())
    }
}

fn valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.len() <= 128
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
