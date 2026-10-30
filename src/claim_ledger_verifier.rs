//! PDF-1.54: bounded, fail-closed architectural claim preflight.
//! Prevents accidental promotion of adapter plans, hypotheses and untested
//! implementations to IMPLEMENTED / DEPLOYED. External evidence signatures,
//! real CI provenance, and live deployment health require separate gates.
//! EvidenceReceipt is trusted input ONLY after upstream signature validation.
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_CLAIMS: usize = 4096;
const MAX_RECEIPTS: usize = 4096;
const MAX_ARTIFACT_PATH: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Implemented,
    AdapterRoadmap,
    Hypothesis,
}

#[derive(Debug, Clone)]
pub struct ArchitecturalClaim<'a> {
    pub claim_id: &'a str,
    pub component_name: &'a str,
    pub classification: Classification,
    pub source_artifact_path: &'a str,
    pub evidence_id: Option<&'a str>,
    pub claims_cloud_provisioned: bool,
    pub claims_unmeasured_submillisecond: bool,
}

#[derive(Debug, Clone)]
pub struct EvidenceReceipt<'a> {
    pub evidence_id: &'a str,
    pub claim_id: &'a str,
    pub source_artifact_path: &'a str,
    pub verified_commit_sha: &'a str,
    pub tests_passed: bool,
    pub deployment_health_checked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    InvalidClaim,
    DuplicateClaim,
    MissingImplementationEvidence,
    IncompatibleEvidence,
    UnverifiedImplementation,
    UnverifiedDeployment,
    UnmeasuredLatencyAssertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimViolation {
    pub claim_id: String,
    pub kind: ViolationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimAudit {
    pub valid: bool,
    pub checked_claims: usize,
    pub violations: Vec<ClaimViolation>,
    /// This validator does NOT authenticate the supplied evidence receipts.
    pub external_evidence_provenance_checked: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClaimAuditError {
    #[error("empty or malformed expected repository SHA")]
    InvalidRevision,
    #[error("audit input exceeded bounded resource limits")]
    Limit,
    #[error("duplicate or malformed evidence receipt")]
    InvalidEvidence,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_ARTIFACT_PATH
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'.'))
}

fn valid_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Audits only syntactic/procedural evidence agreement. The caller must
/// independently authenticate each receipt and the expected revision.
/// Consequently, success is NOT permission to mark a canonical stage completed.
pub fn audit_claims(
    claims: &[ArchitecturalClaim<'_>],
    receipts: &[EvidenceReceipt<'_>],
    expected_revision: &str,
) -> Result<ClaimAudit, ClaimAuditError> {
    if !valid_git_sha(expected_revision) {
        return Err(ClaimAuditError::InvalidRevision);
    }
    if claims.len() > MAX_CLAIMS || receipts.len() > MAX_RECEIPTS {
        return Err(ClaimAuditError::Limit);
    }
    let mut evidence = BTreeMap::new();
    for receipt in receipts {
        if !valid_id(receipt.evidence_id)
            || !valid_id(receipt.claim_id)
            || !valid_path(receipt.source_artifact_path)
            || !valid_git_sha(receipt.verified_commit_sha)
            || evidence.insert(receipt.evidence_id, receipt).is_some()
        {
            return Err(ClaimAuditError::InvalidEvidence);
        }
    }
    let mut ids = BTreeSet::new();
    let mut violations = Vec::new();
    for claim in claims {
        let mut add = |kind| {
            violations.push(ClaimViolation {
                claim_id: claim.claim_id.to_owned(),
                kind,
            });
        };
        if !valid_id(claim.claim_id)
            || !valid_id(claim.component_name)
            || !valid_path(claim.source_artifact_path)
            || claim.evidence_id.is_some_and(|s| !valid_id(s))
        {
            add(ViolationKind::InvalidClaim);
            continue;
        }
        if !ids.insert(claim.claim_id) {
            add(ViolationKind::DuplicateClaim);
            continue;
        }
        let receipt = claim.evidence_id.and_then(|id| evidence.get(id).copied());
        let bound = receipt.is_some_and(|r| {
            r.claim_id == claim.claim_id
                && r.source_artifact_path == claim.source_artifact_path
                && r.verified_commit_sha == expected_revision
        });
        let tested = bound && receipt.is_some_and(|r| r.tests_passed);
        if claim.classification == Classification::Implemented {
            if receipt.is_none() {
                add(ViolationKind::MissingImplementationEvidence);
            } else if !bound {
                add(ViolationKind::IncompatibleEvidence);
            } else if !tested {
                add(ViolationKind::UnverifiedImplementation);
            }
        }
        if claim.claims_cloud_provisioned
            && (claim.classification != Classification::Implemented
                || !tested
                || !receipt.is_some_and(|r| r.deployment_health_checked))
        {
            add(ViolationKind::UnverifiedDeployment);
        }
        if claim.claims_unmeasured_submillisecond {
            add(ViolationKind::UnmeasuredLatencyAssertion);
        }
    }
    Ok(ClaimAudit {
        valid: violations.is_empty(),
        checked_claims: claims.len(),
        violations,
        external_evidence_provenance_checked: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "1234567890abcdef1234567890abcdef12345678";
    const OTHER_SHA: &str = "abcd567890abcdef1234567890abcdef12345678";

    fn claim() -> ArchitecturalClaim<'static> {
        ArchitecturalClaim {
            claim_id: "claim_1",
            component_name: "omat_registry",
            classification: Classification::Implemented,
            source_artifact_path: "src/mcp_registry_integrity.rs",
            evidence_id: Some("ci_1"),
            claims_cloud_provisioned: false,
            claims_unmeasured_submillisecond: false,
        }
    }

    fn receipt() -> EvidenceReceipt<'static> {
        EvidenceReceipt {
            evidence_id: "ci_1",
            claim_id: "claim_1",
            source_artifact_path: "src/mcp_registry_integrity.rs",
            verified_commit_sha: SHA,
            tests_passed: true,
            deployment_health_checked: false,
        }
    }

    fn check(claims: &[ArchitecturalClaim<'_>], receipts: &[EvidenceReceipt<'_>]) -> ClaimAudit {
        audit_claims(claims, receipts, SHA).unwrap()
    }

    #[test]
    fn valid_implemented_claim_requires_bound_test_evidence() {
        let result = check(&[claim()], &[receipt()]);
        assert!(result.valid);
        assert_eq!(result.checked_claims, 1);
        assert!(!result.external_evidence_provenance_checked);
    }

    #[test]
    fn implemented_without_test_evidence_is_rejected() {
        let result = check(&[claim()], &[]);
        assert_eq!(
            result.violations[0].kind,
            ViolationKind::MissingImplementationEvidence
        );
        assert!(!result.valid);
    }

    #[test]
    fn incorrect_revision_and_claim_binding_are_rejected() {
        let mut stale = receipt();
        stale.verified_commit_sha = OTHER_SHA;
        assert_eq!(
            check(&[claim()], &[stale]).violations[0].kind,
            ViolationKind::IncompatibleEvidence
        );
        let mut foreign = receipt();
        foreign.claim_id = "claim_2";
        assert_eq!(
            check(&[claim()], &[foreign]).violations[0].kind,
            ViolationKind::IncompatibleEvidence
        );
    }

    #[test]
    fn failed_ci_does_not_qualify_as_implementation() {
        let mut failed = receipt();
        failed.tests_passed = false;
        assert_eq!(
            check(&[claim()], &[failed]).violations[0].kind,
            ViolationKind::UnverifiedImplementation
        );
    }

    #[test]
    fn cloud_deployment_needs_real_health_receipt_and_implemented_classification() {
        let mut c = claim();
        c.claims_cloud_provisioned = true;
        assert!(check(&[c.clone()], &[receipt()])
            .violations
            .iter()
            .any(|v| v.kind == ViolationKind::UnverifiedDeployment));
        let mut r = receipt();
        r.deployment_health_checked = true;
        assert!(check(&[c.clone()], &[r]).valid);
        c.classification = Classification::Hypothesis;
        assert_eq!(
            check(&[c], &[receipt()]).violations[0].kind,
            ViolationKind::UnverifiedDeployment
        );
    }

    #[test]
    fn unmeasured_latency_claim_is_never_accepted() {
        let mut c = claim();
        c.claims_unmeasured_submillisecond = true;
        assert_eq!(
            check(&[c], &[receipt()]).violations[0].kind,
            ViolationKind::UnmeasuredLatencyAssertion
        );
    }

    #[test]
    fn duplicate_claim_ids_and_evidence_ids_are_rejected() {
        let c = claim();
        assert_eq!(
            check(&[c.clone(), c], &[receipt()]).violations[0].kind,
            ViolationKind::DuplicateClaim
        );
        assert_eq!(
            audit_claims(&[], &[receipt(), receipt()], SHA).unwrap_err(),
            ClaimAuditError::InvalidEvidence
        );
    }

    #[test]
    fn absolute_and_traversal_artifact_paths_are_rejected() {
        let mut c = claim();
        c.source_artifact_path = "../src/mcp_registry_integrity.rs";
        assert_eq!(
            check(&[c], &[receipt()]).violations[0].kind,
            ViolationKind::InvalidClaim
        );
    }

    #[test]
    fn hypothesis_without_deployment_claim_does_not_require_test_receipt() {
        let mut c = claim();
        c.classification = Classification::Hypothesis;
        c.evidence_id = None;
        assert!(check(&[c], &[]).valid);
    }

    #[test]
    fn invalid_expected_revision_is_rejected() {
        assert_eq!(
            audit_claims(&[], &[], "HEAD").unwrap_err(),
            ClaimAuditError::InvalidRevision
        );
    }
}
