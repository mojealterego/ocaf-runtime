//! PDF Argon 1.23-1.26 and 1.30: bounded governance preflight.
//! This module never executes a tool, mints human approvals or grants authority.
//! A Continue decision still requires mTLS/OAuth, durable audit, idempotency,
//! executor authorization, replay protection and the existing OSTc boundary.
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

type HmacSha256 = Hmac<Sha256>;
const MAX_APPROVAL_LIFETIME_SECS: u64 = 900;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskTier {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    ContractObligations,
    ComplianceEvidence,
    ProcurementScout,
    CoreRetrieval,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApproverRole {
    LegalReviewer,
    FinanceApprover,
    ProcurementLead,
    ComplianceOfficer,
}

impl ApproverRole {
    fn wire(self) -> &'static str {
        match self {
            Self::LegalReviewer => "LEGAL_REVIEWER",
            Self::FinanceApprover => "FINANCE_APPROVER",
            Self::ProcurementLead => "PROCUREMENT_LEAD",
            Self::ComplianceOfficer => "COMPLIANCE_OFFICER",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SocialCommitment {
    pub debtor: String,
    pub creditor: String,
    pub condition: String,
}

#[derive(Clone, Debug)]
pub struct Accountability {
    pub owner: String,
    pub escalation_reviewer: String,
    pub audit_authority: String,
}

#[derive(Clone, Debug)]
pub struct AgentContract {
    pub contract_id: String,
    pub version: String,
    pub tenant: String,
    pub actor: String,
    pub obligations: Vec<SocialCommitment>,
    pub permissions: BTreeSet<String>,
    pub prohibitions: BTreeSet<String>,
    pub accountability: Accountability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyReason {
    InvalidContract,
    InvalidRequest,
    IdentityMismatch,
    ExplicitProhibition,
    NotAllowlisted,
    RiskCapExceeded,
    ProviderNotAuthorized,
    MissingApproval,
    InvalidApproval,
}

/// Continue is NOT an execution authorization. It only means this additional
/// preflight found no denial; downstream independent gates remain mandatory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreflightDecision {
    Continue,
    Deny(DenyReason),
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':')
        })
}

impl AgentContract {
    pub fn validate(&self) -> bool {
        valid_id(&self.contract_id)
            && valid_id(&self.version)
            && valid_id(&self.tenant)
            && valid_id(&self.actor)
            && valid_id(&self.accountability.owner)
            && valid_id(&self.accountability.escalation_reviewer)
            && valid_id(&self.accountability.audit_authority)
            && !self.permissions.is_empty()
            && self.permissions.len() <= 128
            && self.prohibitions.len() <= 128
            && self.permissions.iter().all(|s| valid_id(s))
            && self.prohibitions.iter().all(|s| valid_id(s))
            && self.obligations.len() <= 128
            && self.obligations.iter().all(|c| {
                valid_id(&c.debtor)
                    && valid_id(&c.creditor)
                    && !c.condition.trim().is_empty()
                    && c.condition.len() <= 1024
                    && !c.condition.chars().any(char::is_control)
            })
    }

    /// Call only with identity and time resolved from trusted server context.
    /// Never use request-supplied principal or timestamp as an authority.
    pub fn evaluate(
        &self,
        req: &ActionRequest<'_>,
        authority: Option<&ApprovalAuthority>,
        trusted_now_unix: u64,
    ) -> PreflightDecision {
        use DenyReason as D;
        if !self.validate() {
            return PreflightDecision::Deny(D::InvalidContract);
        }
        if !valid_id(req.request_id)
            || !valid_id(req.action)
            || !valid_id(req.tenant)
            || !valid_id(req.actor)
            || trusted_now_unix == 0
        {
            return PreflightDecision::Deny(D::InvalidRequest);
        }
        if self.tenant != req.tenant || self.actor != req.actor {
            return PreflightDecision::Deny(D::IdentityMismatch);
        }
        if self.prohibitions.contains(req.action) {
            return PreflightDecision::Deny(D::ExplicitProhibition);
        }
        if !self.permissions.contains(req.action) {
            return PreflightDecision::Deny(D::NotAllowlisted);
        }
        if req.domain == Domain::ProcurementScout && req.risk > RiskTier::Medium {
            return PreflightDecision::Deny(D::RiskCapExceeded);
        }
        if req.uses_external_provider {
            match req.provider {
                Some(provider)
                    if provider.authenticated
                        && provider.region_model_authorized
                        && provider.explicit_cost_approved
                        && provider.estimated_cost_micros <= provider.approved_cost_micros => {}
                _ => return PreflightDecision::Deny(D::ProviderNotAuthorized),
            }
        }
        if req.risk >= RiskTier::Medium {
            let Some(approval) = req.approval else {
                return PreflightDecision::Deny(D::MissingApproval);
            };
            let Some(verifier) = authority else {
                return PreflightDecision::Deny(D::MissingApproval);
            };
            if !verifier.verify(approval, req, trusted_now_unix) {
                return PreflightDecision::Deny(D::InvalidApproval);
            }
        }
        PreflightDecision::Continue
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProviderAccess {
    pub authenticated: bool,
    pub region_model_authorized: bool,
    pub explicit_cost_approved: bool,
    /// Integer micro-units of the configured billing currency; never floats.
    pub estimated_cost_micros: u64,
    pub approved_cost_micros: u64,
}

pub struct ActionRequest<'a> {
    pub request_id: &'a str,
    pub tenant: &'a str,
    pub actor: &'a str,
    pub action: &'a str,
    pub domain: Domain,
    pub risk: RiskTier,
    pub payload_sha256: [u8; 32],
    pub uses_external_provider: bool,
    pub provider: Option<&'a ProviderAccess>,
    pub approval: Option<&'a ApprovalArtifact>,
}

pub fn payload_sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Signed by an external trusted human-approval service, never by the agent.
/// A valid signature alone is NOT one-time-use; a durable nonce journal must
/// consume approval_id before any effectful execution.
#[derive(Clone, Debug)]
pub struct ApprovalArtifact {
    pub approval_id: String,
    pub approver_id: String,
    pub role: ApproverRole,
    pub request_id: String,
    pub tenant: String,
    pub actor: String,
    pub action: String,
    pub payload_sha256: [u8; 32],
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    pub mac: [u8; 32],
}

impl ApprovalArtifact {
    fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = b"OCAF-HUMAN-APPROVAL-v1".to_vec();
        for field in [
            self.approval_id.as_bytes(),
            self.approver_id.as_bytes(),
            self.role.wire().as_bytes(),
            self.request_id.as_bytes(),
            self.tenant.as_bytes(),
            self.actor.as_bytes(),
            self.action.as_bytes(),
            &self.payload_sha256,
            &self.issued_at_unix.to_be_bytes(),
            &self.expires_at_unix.to_be_bytes(),
        ] {
            bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
            bytes.extend_from_slice(field);
        }
        bytes
    }
}

/// Operator-provisioned verification key. No production approval-minting API.
pub struct ApprovalAuthority {
    key: [u8; 32],
}

impl ApprovalAuthority {
    pub fn new(key: [u8; 32]) -> Option<Self> {
        (!key.iter().all(|byte| *byte == 0)).then_some(Self { key })
    }

    pub fn verify(
        &self,
        approval: &ApprovalArtifact,
        req: &ActionRequest<'_>,
        trusted_now_unix: u64,
    ) -> bool {
        if !valid_id(&approval.approval_id)
            || !valid_id(&approval.approver_id)
            || !valid_id(&approval.request_id)
            || !valid_id(&approval.tenant)
            || !valid_id(&approval.actor)
            || !valid_id(&approval.action)
            || approval.request_id != req.request_id
            || approval.tenant != req.tenant
            || approval.actor != req.actor
            || approval.action != req.action
            || approval.payload_sha256 != req.payload_sha256
            || approval.issued_at_unix > trusted_now_unix
            || trusted_now_unix >= approval.expires_at_unix
            || !approval
                .expires_at_unix
                .checked_sub(approval.issued_at_unix)
                .is_some_and(|ttl| ttl > 0 && ttl <= MAX_APPROVAL_LIFETIME_SECS)
        {
            return false;
        }
        let authorized_role = match req.domain {
            Domain::ContractObligations => approval.role == ApproverRole::LegalReviewer,
            Domain::ComplianceEvidence => approval.role == ApproverRole::ComplianceOfficer,
            Domain::ProcurementScout => matches!(
                approval.role,
                ApproverRole::FinanceApprover | ApproverRole::ProcurementLead
            ),
            Domain::CoreRetrieval => approval.role == ApproverRole::ComplianceOfficer,
        };
        if !authorized_role {
            return false;
        }
        let Ok(mut mac) = HmacSha256::new_from_slice(&self.key) else {
            return false;
        };
        mac.update(&approval.signing_bytes());
        mac.verify_slice(&approval.mac).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> AgentContract {
        AgentContract {
            contract_id: "contract_1".into(),
            version: "v1".into(),
            tenant: "tenant_a".into(),
            actor: "agent_a".into(),
            obligations: vec![SocialCommitment {
                debtor: "agent_a".into(),
                creditor: "operator".into(),
                condition: "Provide verified evidence".into(),
            }],
            permissions: BTreeSet::from(["read".into(), "buy".into()]),
            prohibitions: BTreeSet::new(),
            accountability: Accountability {
                owner: "operator".into(),
                escalation_reviewer: "reviewer".into(),
                audit_authority: "auditor".into(),
            },
        }
    }

    fn request<'a>() -> ActionRequest<'a> {
        ActionRequest {
            request_id: "request_1",
            tenant: "tenant_a",
            actor: "agent_a",
            action: "buy",
            domain: Domain::ProcurementScout,
            risk: RiskTier::Medium,
            payload_sha256: payload_sha256(b"exact request"),
            uses_external_provider: false,
            provider: None,
            approval: None,
        }
    }

    fn signed_approval(req: &ActionRequest<'_>, role: ApproverRole) -> ApprovalArtifact {
        let mut approval = ApprovalArtifact {
            approval_id: "approval_1".into(),
            approver_id: "human_1".into(),
            role,
            request_id: req.request_id.into(),
            tenant: req.tenant.into(),
            actor: req.actor.into(),
            action: req.action.into(),
            payload_sha256: req.payload_sha256,
            issued_at_unix: 100,
            expires_at_unix: 200,
            mac: [0; 32],
        };
        let authority = ApprovalAuthority::new([9; 32]).unwrap();
        let mut mac = HmacSha256::new_from_slice(&authority.key).unwrap();
        mac.update(&approval.signing_bytes());
        approval.mac.copy_from_slice(&mac.finalize().into_bytes());
        approval
    }

    #[test]
    fn exact_allowlist_and_explicit_prohibition_fail_closed() {
        let mut c = contract();
        let mut r = request();
        r.risk = RiskTier::Low;
        r.action = "read";
        assert_eq!(c.evaluate(&r, None, 150), PreflightDecision::Continue);
        r.action = "read.more";
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::NotAllowlisted)
        );
        r.action = "read";
        c.prohibitions.insert("read".into());
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::ExplicitProhibition)
        );
    }

    #[test]
    fn tenant_and_actor_mismatch_are_rejected() {
        let c = contract();
        let mut r = request();
        r.risk = RiskTier::Low;
        r.tenant = "tenant_b";
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::IdentityMismatch)
        );
        r.tenant = "tenant_a";
        r.actor = "agent_b";
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::IdentityMismatch)
        );
    }

    #[test]
    fn procurement_high_risk_denied_before_approval() {
        let c = contract();
        let mut r = request();
        r.risk = RiskTier::High;
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::RiskCapExceeded)
        );
    }

    #[test]
    fn medium_risk_requires_valid_human_approval_and_role() {
        let c = contract();
        let authority = ApprovalAuthority::new([9; 32]).unwrap();
        let mut r = request();
        assert_eq!(
            c.evaluate(&r, Some(&authority), 150),
            PreflightDecision::Deny(DenyReason::MissingApproval)
        );
        let wrong_role = signed_approval(&r, ApproverRole::LegalReviewer);
        r.approval = Some(&wrong_role);
        assert_eq!(
            c.evaluate(&r, Some(&authority), 150),
            PreflightDecision::Deny(DenyReason::InvalidApproval)
        );
        let correct = signed_approval(&r, ApproverRole::ProcurementLead);
        r.approval = Some(&correct);
        assert_eq!(
            c.evaluate(&r, Some(&authority), 150),
            PreflightDecision::Continue
        );
    }

    #[test]
    fn signature_binds_payload_request_and_expiry() {
        let c = contract();
        let authority = ApprovalAuthority::new([9; 32]).unwrap();
        let mut r = request();
        let approval = signed_approval(&r, ApproverRole::FinanceApprover);
        r.approval = Some(&approval);
        assert_eq!(c.evaluate(&r, Some(&authority), 150), PreflightDecision::Continue);
        r.payload_sha256 = payload_sha256(b"tampered");
        assert_eq!(
            c.evaluate(&r, Some(&authority), 150),
            PreflightDecision::Deny(DenyReason::InvalidApproval)
        );
        r.payload_sha256 = approval.payload_sha256;
        r.request_id = "replayed_other_request";
        assert_eq!(
            c.evaluate(&r, Some(&authority), 150),
            PreflightDecision::Deny(DenyReason::InvalidApproval)
        );
        r.request_id = "request_1";
        assert_eq!(
            c.evaluate(&r, Some(&authority), 200),
            PreflightDecision::Deny(DenyReason::InvalidApproval)
        );
    }

    #[test]
    fn external_provider_requires_all_permissions_and_integer_cost_ceiling() {
        let c = contract();
        let mut r = request();
        r.risk = RiskTier::Low;
        r.uses_external_provider = true;
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::ProviderNotAuthorized)
        );
        let mut provider = ProviderAccess {
            authenticated: true,
            region_model_authorized: true,
            explicit_cost_approved: true,
            estimated_cost_micros: 101,
            approved_cost_micros: 100,
        };
        r.provider = Some(&provider);
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::ProviderNotAuthorized)
        );
        provider.estimated_cost_micros = 100;
        r.provider = Some(&provider);
        assert_eq!(c.evaluate(&r, None, 150), PreflightDecision::Continue);
    }

    #[test]
    fn invalid_contract_and_untrusted_time_fail_closed() {
        let mut c = contract();
        let r = request();
        c.permissions.insert("read/*".into());
        assert_eq!(
            c.evaluate(&r, None, 150),
            PreflightDecision::Deny(DenyReason::InvalidContract)
        );
        c.permissions.remove("read/*");
        assert_eq!(
            c.evaluate(&r, None, 0),
            PreflightDecision::Deny(DenyReason::InvalidRequest)
        );
        assert!(ApprovalAuthority::new([0; 32]).is_none());
    }
}
