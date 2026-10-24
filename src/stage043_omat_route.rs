//! Stage 0.43 point 2: fail-closed OMAs -> OMAt route authorization.
//! Routing authorization is independent of transport: the caller MUST verify
//! the peer using mutual TLS before creating an AuthenticatedPeer.
use crate::omat_server::{OmatPolicy, OmatRegistry, ToolCall, ToolReceipt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

    pub fn tenant_for(&self, principal: &str) -> Option<&str> {
        self.peers.get(principal).map(String::as_str)
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

/// Server-side entry point: transport-authenticated identity and request topic
/// must pass the route guard before OMAt evaluates its own tool policy.
/// Tool execution remains disabled by the OMAt executor boundary.
pub fn dispatch_tool_call(
    guard: &OmatRouteGuard,
    peer: &AuthenticatedPeer,
    topic: &str,
    call: &ToolCall,
    policy: &OmatPolicy,
    registry: &OmatRegistry,
) -> Result<ToolReceipt, DispatchError> {
    guard.authorize(peer, topic).map_err(DispatchError::Route)?;
    if call.tenant_id != peer.tenant || policy.tenant_id != peer.tenant {
        return Err(DispatchError::Route(RouteError::Unauthorized));
    }
    let expected_topic = format!("ocaf/omat/{}/requests/{}", peer.tenant, call.request_id);
    if topic != expected_topic {
        return Err(DispatchError::Route(RouteError::InvalidTopic));
    }
    registry.execute(call, policy).map_err(DispatchError::Tool)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    Route(RouteError),
    Tool(crate::omat_server::OmatError),
}

/// A transport session owns the connection lease. Dropping it fences all
/// subsequent dispatches, even when an application keeps the guard alive.
pub struct OmatSessionLease {
    guard: Arc<Mutex<OmatRouteGuard>>,
}

impl OmatSessionLease {
    pub fn new(guard: Arc<Mutex<OmatRouteGuard>>) -> Self {
        Self { guard }
    }

    pub fn connect(&self) -> Result<(), RouteError> {
        let mut guard = self.guard.lock().map_err(|_| RouteError::Disconnected)?;
        guard.set_transport_connected(true);
        Ok(())
    }

    pub fn disconnect(&self) -> Result<(), RouteError> {
        let mut guard = self.guard.lock().map_err(|_| RouteError::Disconnected)?;
        guard.set_transport_connected(false);
        Ok(())
    }

    pub fn dispatch(
        &self,
        peer: &AuthenticatedPeer,
        topic: &str,
        call: &ToolCall,
        policy: &OmatPolicy,
        registry: &OmatRegistry,
    ) -> Result<ToolReceipt, DispatchError> {
        let guard = self
            .guard
            .lock()
            .map_err(|_| DispatchError::Route(RouteError::Disconnected))?;
        dispatch_tool_call(&guard, peer, topic, call, policy, registry)
    }
}

impl Drop for OmatSessionLease {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.guard.lock() {
            guard.set_transport_connected(false);
        }
    }
}
