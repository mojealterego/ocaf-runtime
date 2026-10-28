//! Zenoh transport boundary. No live adapter is shipped in this package.

use thiserror::Error;

use crate::security::{AgentConfig, SecurityError, ServiceConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    Omas,
    Omat,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("transport is disabled")]
    Disabled,
    #[error("payload exceeds 65536 bytes")]
    PayloadTooLarge,
    #[error(transparent)]
    Security(#[from] SecurityError),
}

/// Implementations may only be called after the agent's local policy check.
pub trait ZenohTransport {
    fn request(&self, endpoint: &str, payload: &[u8]) -> Result<Vec<u8>, TransportError>;
}

pub struct DisabledZenohTransport;

impl ZenohTransport for DisabledZenohTransport {
    fn request(&self, _endpoint: &str, _payload: &[u8]) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::Disabled)
    }
}

pub fn request<T: ZenohTransport>(
    config: &AgentConfig,
    service: Service,
    payload: &[u8],
    transport: &T,
) -> Result<Vec<u8>, TransportError> {
    config.validate()?;
    if payload.len() > 65_536 {
        return Err(TransportError::PayloadTooLarge);
    }
    let endpoint: &ServiceConfig = match service {
        Service::Omas => &config.omas,
        Service::Omat => &config.omat,
    };
    if !endpoint.enabled {
        return Err(TransportError::Disabled);
    }
    transport.request(&endpoint.endpoint, payload)
}
