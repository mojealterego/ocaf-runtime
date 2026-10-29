//! Bounded, tenant-bound SASOS capability tickets.
//!
//! This is an object authorization primitive, not a substitute for mTLS,
//! OMAt policy, OSTc attestation, durable replay protection or a real executor.
//! Root keys must be provisioned by the operator from a protected key source.
use crate::stage043_omat_route::AuthenticatedPeer;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;
const MAX_CAVEATS: usize = 16;
const MAX_IDENTIFIER_BYTES: usize = 128;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("invalid capability configuration or identifier")]
    Invalid,
    #[error("capability signature is invalid")]
    Signature,
    #[error("capability attenuation would increase authority")]
    Escalation,
    #[error("capability is not authorized for this operation")]
    Denied,
    #[error("trusted server clock is unavailable")]
    Clock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAction {
    Read,
    Write,
    Execute,
}

impl CapabilityAction {
    fn bit(self) -> u8 {
        match self {
            Self::Read => 1,
            Self::Write => 2,
            Self::Execute => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityVerbs(u8);

impl CapabilityVerbs {
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(2);
    pub const EXECUTE: Self = Self(4);

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn valid(self) -> bool {
        self.0 != 0 && self.0 & !7 == 0
    }

    fn permits(self, action: CapabilityAction) -> bool {
        self.0 & action.bit() != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCaveat {
    pub max_bytes: u64,
    pub expires_at_unix: u64,
    pub verbs: CapabilityVerbs,
}

impl CapabilityCaveat {
    fn valid(self) -> bool {
        self.max_bytes > 0 && self.expires_at_unix > 0 && self.verbs.valid()
    }

    fn attenuates(self, parent: Self) -> bool {
        self.valid()
            && self.max_bytes <= parent.max_bytes
            && self.expires_at_unix <= parent.expires_at_unix
            && self.verbs.0 & !parent.verbs.0 == 0
    }

    fn bytes(self) -> [u8; 17] {
        let mut encoded = [0_u8; 17];
        encoded[..8].copy_from_slice(&self.max_bytes.to_be_bytes());
        encoded[8..16].copy_from_slice(&self.expires_at_unix.to_be_bytes());
        encoded[16] = self.verbs.0;
        encoded
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityTicket {
    id: String,
    tenant_id: String,
    subject_id: String,
    resource_id: String,
    caveats: Vec<CapabilityCaveat>,
    signature: [u8; 32],
}

impl CapabilityTicket {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }
}

/// Authority is deliberately not Clone or Debug: do not duplicate or log root keys.
pub struct CapabilityAuthority {
    root_key: [u8; 32],
}

impl CapabilityAuthority {
    pub fn new(root_key: [u8; 32]) -> Result<Self, CapabilityError> {
        if root_key.iter().all(|byte| *byte == 0) {
            return Err(CapabilityError::Invalid);
        }
        Ok(Self { root_key })
    }

    /// Mint only for an operator-authenticated peer; never derive peer identity
    /// from untrusted JSON arguments, tool descriptions or model output.
    pub fn mint(
        &self,
        peer: &AuthenticatedPeer,
        resource_id: &str,
        caveat: CapabilityCaveat,
    ) -> Result<CapabilityTicket, CapabilityError> {
        if !valid_id(&peer.tenant)
            || !valid_id(&peer.principal)
            || !valid_resource(resource_id)
            || !caveat.valid()
        {
            return Err(CapabilityError::Invalid);
        }
        let mut ticket = CapabilityTicket {
            id: Uuid::new_v4().to_string(),
            tenant_id: peer.tenant.clone(),
            subject_id: peer.principal.clone(),
            resource_id: resource_id.to_owned(),
            caveats: vec![caveat],
            signature: [0; 32],
        };
        let base = self.base_signature(&ticket)?;
        ticket.signature = hmac(&base, &caveat.bytes())?;
        Ok(ticket)
    }

    /// Reject widening before signing. Verify the parent, including every
    /// earlier caveat, so a modified parent can never become authoritative.
    pub fn attenuate(
        &self,
        parent: &CapabilityTicket,
        narrowed: CapabilityCaveat,
    ) -> Result<CapabilityTicket, CapabilityError> {
        self.verify_signature(parent)?;
        let previous = *parent.caveats.last().ok_or(CapabilityError::Invalid)?;
        if !narrowed.attenuates(previous) {
            return Err(CapabilityError::Escalation);
        }
        if parent.caveats.len() >= MAX_CAVEATS {
            return Err(CapabilityError::Invalid);
        }
        let mut child = parent.clone();
        child.signature = hmac(&parent.signature, &narrowed.bytes())?;
        child.caveats.push(narrowed);
        Ok(child)
    }

    /// The trusted server clock is used here, never a timestamp supplied by
    /// the client. This does not provide one-shot/replay semantics.
    pub fn authorize(
        &self,
        ticket: &CapabilityTicket,
        peer: &AuthenticatedPeer,
        resource_id: &str,
        action: CapabilityAction,
        bytes: u64,
    ) -> Result<(), CapabilityError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CapabilityError::Clock)?
            .as_secs();
        self.authorize_at(ticket, peer, resource_id, action, bytes, now)
    }

    fn authorize_at(
        &self,
        ticket: &CapabilityTicket,
        peer: &AuthenticatedPeer,
        resource_id: &str,
        action: CapabilityAction,
        bytes: u64,
        now: u64,
    ) -> Result<(), CapabilityError> {
        self.verify_signature(ticket)?;
        if peer.tenant != ticket.tenant_id
            || peer.principal != ticket.subject_id
            || resource_id != ticket.resource_id
        {
            return Err(CapabilityError::Denied);
        }
        for caveat in &ticket.caveats {
            if now >= caveat.expires_at_unix
                || bytes > caveat.max_bytes
                || !caveat.verbs.permits(action)
            {
                return Err(CapabilityError::Denied);
            }
        }
        Ok(())
    }

    fn base_signature(&self, ticket: &CapabilityTicket) -> Result<[u8; 32], CapabilityError> {
        if !valid_id(&ticket.id)
            || !valid_id(&ticket.tenant_id)
            || !valid_id(&ticket.subject_id)
            || !valid_resource(&ticket.resource_id)
        {
            return Err(CapabilityError::Invalid);
        }
        let mut bytes = b"ocaf.sasos.capability.v1".to_vec();
        for field in [
            &ticket.id,
            &ticket.tenant_id,
            &ticket.subject_id,
            &ticket.resource_id,
        ] {
            bytes.extend_from_slice(&(field.len() as u16).to_be_bytes());
            bytes.extend_from_slice(field.as_bytes());
        }
        hmac(&self.root_key, &bytes)
    }

    fn verify_signature(&self, ticket: &CapabilityTicket) -> Result<(), CapabilityError> {
        if ticket.caveats.is_empty() || ticket.caveats.len() > MAX_CAVEATS {
            return Err(CapabilityError::Invalid);
        }
        let mut signature = self.base_signature(ticket)?;
        let mut previous = None;
        for (index, caveat) in ticket.caveats.iter().enumerate() {
            if !caveat.valid() || previous.is_some_and(|p| !caveat.attenuates(p)) {
                return Err(CapabilityError::Invalid);
            }
            let mut mac =
                HmacSha256::new_from_slice(&signature).map_err(|_| CapabilityError::Invalid)?;
            mac.update(&caveat.bytes());
            if index + 1 == ticket.caveats.len() {
                mac.verify_slice(&ticket.signature)
                    .map_err(|_| CapabilityError::Signature)?;
            } else {
                signature = mac.finalize().into_bytes().into();
            }
            previous = Some(*caveat);
        }
        Ok(())
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

fn valid_resource(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && !value.contains("..")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}

fn hmac(key: &[u8], data: &[u8]) -> Result<[u8; 32], CapabilityError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| CapabilityError::Invalid)?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(tenant: &str, principal: &str) -> AuthenticatedPeer {
        AuthenticatedPeer {
            tenant: tenant.into(),
            principal: principal.into(),
        }
    }

    fn caveat() -> CapabilityCaveat {
        CapabilityCaveat {
            max_bytes: 1024,
            expires_at_unix: 2_000_000_000,
            verbs: CapabilityVerbs::READ.union(CapabilityVerbs::WRITE),
        }
    }

    #[test]
    fn authorized_read_and_monotonic_delegation() {
        let authority = CapabilityAuthority::new([17; 32]).unwrap();
        let identity = peer("tenant_a", "agent_a");
        let root = authority
            .mint(&identity, "sasos://object-a", caveat())
            .unwrap();
        let child = authority
            .attenuate(
                &root,
                CapabilityCaveat {
                    max_bytes: 512,
                    expires_at_unix: 1_900_000_000,
                    verbs: CapabilityVerbs::READ,
                },
            )
            .unwrap();
        assert!(authority
            .authorize_at(
                &child,
                &identity,
                "sasos://object-a",
                CapabilityAction::Read,
                512,
                1_800_000_000
            )
            .is_ok());
        assert_eq!(
            authority.authorize_at(
                &child,
                &identity,
                "sasos://object-a",
                CapabilityAction::Write,
                1,
                1_800_000_000
            ),
            Err(CapabilityError::Denied)
        );
        assert_eq!(
            authority.authorize_at(
                &child,
                &identity,
                "sasos://object-a",
                CapabilityAction::Read,
                513,
                1_800_000_000
            ),
            Err(CapabilityError::Denied)
        );
    }

    #[test]
    fn rejects_privilege_expansion_in_every_dimension() {
        let authority = CapabilityAuthority::new([17; 32]).unwrap();
        let root = authority
            .mint(&peer("t", "a"), "resource", caveat())
            .unwrap();
        for widened in [
            CapabilityCaveat {
                max_bytes: 1025,
                ..caveat()
            },
            CapabilityCaveat {
                expires_at_unix: 2_000_000_001,
                ..caveat()
            },
            CapabilityCaveat {
                verbs: caveat().verbs.union(CapabilityVerbs::EXECUTE),
                ..caveat()
            },
        ] {
            assert!(matches!(
                authority.attenuate(&root, widened),
                Err(CapabilityError::Escalation)
            ));
        }
    }

    #[test]
    fn rejects_cross_tenant_subject_resource_and_expiry() {
        let authority = CapabilityAuthority::new([17; 32]).unwrap();
        let root = authority
            .mint(&peer("t", "a"), "resource", caveat())
            .unwrap();
        for (identity, resource) in [
            (peer("other", "a"), "resource"),
            (peer("t", "other"), "resource"),
            (peer("t", "a"), "other"),
        ] {
            assert_eq!(
                authority.authorize_at(
                    &root,
                    &identity,
                    resource,
                    CapabilityAction::Read,
                    1,
                    1_800_000_000
                ),
                Err(CapabilityError::Denied)
            );
        }
        assert_eq!(
            authority.authorize_at(
                &root,
                &peer("t", "a"),
                "resource",
                CapabilityAction::Read,
                1,
                2_000_000_000
            ),
            Err(CapabilityError::Denied)
        );
    }

    #[test]
    fn rejects_tampering_and_different_signing_key() {
        let authority = CapabilityAuthority::new([17; 32]).unwrap();
        let root = authority
            .mint(&peer("t", "a"), "resource", caveat())
            .unwrap();
        let mut tampered = root.clone();
        tampered.resource_id = "other".into();
        assert_eq!(
            authority.authorize_at(
                &tampered,
                &peer("t", "a"),
                "other",
                CapabilityAction::Read,
                1,
                1_800_000_000
            ),
            Err(CapabilityError::Signature)
        );
        let mut tampered = root.clone();
        tampered.caveats[0].max_bytes = 2048;
        assert_eq!(
            authority.authorize_at(
                &tampered,
                &peer("t", "a"),
                "resource",
                CapabilityAction::Read,
                1,
                1_800_000_000
            ),
            Err(CapabilityError::Signature)
        );
        let other = CapabilityAuthority::new([18; 32]).unwrap();
        assert_eq!(
            other.authorize_at(
                &root,
                &peer("t", "a"),
                "resource",
                CapabilityAction::Read,
                1,
                1_800_000_000
            ),
            Err(CapabilityError::Signature)
        );
    }

    #[test]
    fn rejects_invalid_keys_and_caveats() {
        assert!(matches!(
            CapabilityAuthority::new([0; 32]),
            Err(CapabilityError::Invalid)
        ));
        let authority = CapabilityAuthority::new([17; 32]).unwrap();
        assert!(matches!(
            authority.mint(
                &peer("t", "a"),
                "resource",
                CapabilityCaveat {
                    max_bytes: 0,
                    ..caveat()
                }
            ),
            Err(CapabilityError::Invalid)
        ));
        assert!(matches!(
            authority.mint(&peer("t", "a"), "../resource", caveat()),
            Err(CapabilityError::Invalid)
        ));
    }
}
