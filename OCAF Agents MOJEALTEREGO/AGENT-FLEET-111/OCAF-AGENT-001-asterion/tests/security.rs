use std::sync::atomic::{AtomicUsize, Ordering};

use ocaf_agent_001_asterion::lifecycle::{request_transition, Lifecycle};
use ocaf_agent_001_asterion::security::{AgentConfig, ReadinessEvidence, SecurityError};
use ocaf_agent_001_asterion::zenoh::{request, Service, TransportError, ZenohTransport};

fn config() -> AgentConfig {
    AgentConfig::parse(include_str!("../config/agent.toml")).unwrap()
}

#[test]
fn forged_readiness_cannot_enable_active() {
    let config = config();
    let forged = ReadinessEvidence {
        tests_passed: true,
        signature_verified: true,
        omas_ready: true,
        omat_ready: true,
        ostc_ready: true,
        source_access_granted: true,
    };
    assert!(matches!(
        config.admit_active(&forged),
        Err(SecurityError::ActiveDenied)
    ));
    assert!(matches!(
        request_transition(&config, Lifecycle::Active, &forged),
        Err(SecurityError::ActiveDenied)
    ));
    assert!(request_transition(&config, Lifecycle::Staged, &forged).is_err());
}

#[test]
fn source_access_and_service_forgery_are_rejected() {
    let mut config = config();
    config.source_access = "granted".into();
    assert!(config.validate().is_err());
    config = self::config();
    config.ostc_enabled = true;
    assert!(config.validate().is_err());
    config = self::config();
    config.active_admission = "ALLOW".into();
    assert!(config.validate().is_err());
    config = self::config();
    config.omas.endpoint = config.omat.endpoint.clone();
    assert!(config.validate().is_err());
    config = self::config();
    config.omat.enabled = true;
    assert!(config.validate().is_err());
    config = self::config();
    config.model_backend = "claimed-ready".into();
    assert!(config.validate().is_err());
}

#[test]
fn unknown_configuration_fields_are_rejected() {
    let source = format!(
        "{}\nunsafe_override = true\n",
        include_str!("../config/agent.toml")
    );
    assert!(AgentConfig::parse(&source).is_err());
}

struct CountingTransport(AtomicUsize);

impl ZenohTransport for CountingTransport {
    fn request(&self, _endpoint: &str, _payload: &[u8]) -> Result<Vec<u8>, TransportError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

#[test]
fn remote_transport_is_closed_before_adapter_is_called() {
    let transport = CountingTransport(AtomicUsize::new(0));
    for service in [Service::Omas, Service::Omat] {
        assert!(matches!(
            request(&config(), service, b"probe", &transport),
            Err(TransportError::Disabled)
        ));
    }
    assert_eq!(transport.0.load(Ordering::SeqCst), 0);
}
