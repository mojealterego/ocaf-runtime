//! Source PDF-1.39–1.40: bounded, read-only compliance evidence inspection.
//! This module does not verify regulatory conformity, external attestations,
//! human sign-off, nor authorize remediation/external tool execution.
//! Its integrity check establishes ONLY agreement with a caller-supplied digest;
//! an independent trusted digest anchor is needed to prove original provenance.
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

const MAX_ARTIFACTS: usize = 64;
const MAX_ARTIFACT_BYTES: usize = 256 * 1024;
const MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegulatoryFramework {
    Soc2Type2,
    GdprArticle32,
    NcscCafGovAssure,
    CyberEssentialsPlus,
    NercCip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CafObjective {
    ManagingSecurityRisk,
    ProtectingAgainstCyberAttack,
    DetectingCyberSecurityEvents,
    MinimisingIncidentImpact,
}

impl CafObjective {
    const ALL: [Self; 4] = [
        Self::ManagingSecurityRisk,
        Self::ProtectingAgainstCyberAttack,
        Self::DetectingCyberSecurityEvents,
        Self::MinimisingIncidentImpact,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensitivity {
    Internal,
    Confidential,
    Restricted,
}

#[derive(Debug, Clone)]
pub struct ComplianceEvidenceArtifact<'a> {
    pub tenant_id: &'a str,
    pub evidence_id: &'a str,
    pub source_id: &'a str,
    pub framework: RegulatoryFramework,
    pub objective: Option<CafObjective>,
    pub sensitivity: Sensitivity,
    pub collected_at_unix: u64,
    pub valid_until_unix: u64,
    pub content: &'a [u8],
    pub expected_sha256: [u8; 32],
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvidenceError {
    #[error("invalid identifier, tenant or framework binding")]
    InvalidBinding,
    #[error("empty content or resource limit exceeded")]
    ResourceLimit,
    #[error("duplicate evidence identifier")]
    DuplicateEvidence,
    #[error("invalid evidence validity interval")]
    InvalidInterval,
    #[error("evidence checksum mismatch")]
    ChecksumMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceAssessmentState {
    InsufficientEvidence,
    CafObjectivesPresentUnattested,
    IntegrityCheckedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAssessment {
    pub state: EvidenceAssessmentState,
    pub checked_evidence_ids: Vec<String>,
    pub expired_evidence_ids: Vec<String>,
    pub covered_caf_objectives: BTreeSet<CafObjective>,
    pub missing_caf_objectives: BTreeSet<CafObjective>,
    /// Always false here: an independent verifier is NOT integrated.
    pub independently_attested: bool,
    /// Always false here: no certification/authorization is issued.
    pub certified_compliant: bool,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

/// All artifacts must already be authenticated and tenant-scoped by the caller.
/// Objective labels are supplied claims; checking A/B/C/D coverage does not
/// authenticate the claim's semantic meaning.
pub fn inspect_evidence(
    trusted_tenant: &str,
    framework: RegulatoryFramework,
    artifacts: &[ComplianceEvidenceArtifact<'_>],
    now_unix: u64,
) -> Result<EvidenceAssessment, EvidenceError> {
    if !valid_id(trusted_tenant) {
        return Err(EvidenceError::InvalidBinding);
    }
    if artifacts.len() > MAX_ARTIFACTS {
        return Err(EvidenceError::ResourceLimit);
    }
    let mut ids = BTreeSet::new();
    let mut size = 0usize;
    let mut accepted = Vec::new();
    let mut expired = Vec::new();
    let mut covered = BTreeSet::new();
    for artifact in artifacts {
        if artifact.tenant_id != trusted_tenant
            || !valid_id(artifact.tenant_id)
            || !valid_id(artifact.source_id)
            || !valid_id(artifact.evidence_id)
            || artifact.framework != framework
            || (framework == RegulatoryFramework::NcscCafGovAssure)
                != artifact.objective.is_some()
        {
            return Err(EvidenceError::InvalidBinding);
        }
        if !ids.insert(artifact.evidence_id) {
            return Err(EvidenceError::DuplicateEvidence);
        }
        if artifact.collected_at_unix >= artifact.valid_until_unix
            || artifact.collected_at_unix > now_unix
        {
            return Err(EvidenceError::InvalidInterval);
        }
        if artifact.content.is_empty() || artifact.content.len() > MAX_ARTIFACT_BYTES {
            return Err(EvidenceError::ResourceLimit);
        }
        size = size
            .checked_add(artifact.content.len())
            .ok_or(EvidenceError::ResourceLimit)?;
        if size > MAX_TOTAL_BYTES {
            return Err(EvidenceError::ResourceLimit);
        }
        let digest: [u8; 32] = Sha256::digest(artifact.content).into();
        if digest != artifact.expected_sha256 {
            return Err(EvidenceError::ChecksumMismatch);
        }
        if now_unix >= artifact.valid_until_unix {
            expired.push(artifact.evidence_id.to_owned());
            continue;
        }
        accepted.push(artifact.evidence_id.to_owned());
        if let Some(objective) = artifact.objective {
            covered.insert(objective);
        }
    }
    accepted.sort_unstable();
    expired.sort_unstable();
    let missing = if framework == RegulatoryFramework::NcscCafGovAssure {
        CafObjective::ALL
            .into_iter()
            .filter(|item| !covered.contains(item))
            .collect()
    } else {
        BTreeSet::new()
    };
    let state = if accepted.is_empty() || !missing.is_empty() {
        EvidenceAssessmentState::InsufficientEvidence
    } else if framework == RegulatoryFramework::NcscCafGovAssure {
        EvidenceAssessmentState::CafObjectivesPresentUnattested
    } else {
        EvidenceAssessmentState::IntegrityCheckedOnly
    };
    Ok(EvidenceAssessment {
        state,
        checked_evidence_ids: accepted,
        expired_evidence_ids: expired,
        covered_caf_objectives: covered,
        missing_caf_objectives: missing,
        independently_attested: false,
        certified_compliant: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "tenant_one";
    const CONTENT: &[u8] = b"controlled test evidence - not a real attestation";
    const NOW: u64 = 1_800_000_000;

    fn artifact(id: &'static str, objective: CafObjective) -> ComplianceEvidenceArtifact<'static> {
        ComplianceEvidenceArtifact {
            tenant_id: TENANT,
            evidence_id: id,
            source_id: "fixture_source",
            framework: RegulatoryFramework::NcscCafGovAssure,
            objective: Some(objective),
            sensitivity: Sensitivity::Restricted,
            collected_at_unix: NOW - 86_400,
            valid_until_unix: NOW + 86_400,
            content: CONTENT,
            expected_sha256: Sha256::digest(CONTENT).into(),
        }
    }

    fn complete_pack() -> Vec<ComplianceEvidenceArtifact<'static>> {
        CafObjective::ALL
            .into_iter()
            .enumerate()
            .map(|(i, objective)| {
                artifact(
                    match i {
                        0 => "ev_a",
                        1 => "ev_b",
                        2 => "ev_c",
                        _ => "ev_d",
                    },
                    objective,
                )
            })
            .collect()
    }

    #[test]
    fn all_four_caf_objectives_are_covered_without_self_certification() {
        let result = inspect_evidence(
            TENANT,
            RegulatoryFramework::NcscCafGovAssure,
            &complete_pack(),
            NOW,
        )
        .unwrap();
        assert_eq!(result.state, EvidenceAssessmentState::CafObjectivesPresentUnattested);
        assert_eq!(result.checked_evidence_ids.len(), 4);
        assert!(result.missing_caf_objectives.is_empty());
        assert!(!result.independently_attested);
        assert!(!result.certified_compliant);
    }

    #[test]
    fn missing_objective_does_not_pass() {
        let pack = complete_pack();
        let result = inspect_evidence(
            TENANT,
            RegulatoryFramework::NcscCafGovAssure,
            &pack[..3],
            NOW,
        )
        .unwrap();
        assert_eq!(result.state, EvidenceAssessmentState::InsufficientEvidence);
        assert!(result
            .missing_caf_objectives
            .contains(&CafObjective::MinimisingIncidentImpact));
    }

    #[test]
    fn tampered_bytes_are_rejected_even_if_objective_labels_are_complete() {
        let mut pack = complete_pack();
        pack[0].content = b"altered content";
        assert_eq!(
            inspect_evidence(TENANT, RegulatoryFramework::NcscCafGovAssure, &pack, NOW).unwrap_err(),
            EvidenceError::ChecksumMismatch
        );
    }

    #[test]
    fn expired_evidence_is_not_accepted_as_caf_coverage() {
        let mut pack = complete_pack();
        pack[0].valid_until_unix = NOW;
        let result = inspect_evidence(
            TENANT,
            RegulatoryFramework::NcscCafGovAssure,
            &pack,
            NOW,
        )
        .unwrap();
        assert_eq!(result.state, EvidenceAssessmentState::InsufficientEvidence);
        assert_eq!(result.expired_evidence_ids, vec!["ev_a"]);
        assert!(result
            .missing_caf_objectives
            .contains(&CafObjective::ManagingSecurityRisk));
    }

    #[test]
    fn tenant_mixing_and_framework_confusion_fail_closed() {
        let mut pack = complete_pack();
        pack[1].tenant_id = "tenant_two";
        assert_eq!(
            inspect_evidence(TENANT, RegulatoryFramework::NcscCafGovAssure, &pack, NOW).unwrap_err(),
            EvidenceError::InvalidBinding
        );
        pack[1].tenant_id = TENANT;
        pack[1].framework = RegulatoryFramework::Soc2Type2;
        assert_eq!(
            inspect_evidence(TENANT, RegulatoryFramework::NcscCafGovAssure, &pack, NOW).unwrap_err(),
            EvidenceError::InvalidBinding
        );
    }

    #[test]
    fn duplicate_evidence_and_invalid_intervals_are_rejected() {
        let mut pack = complete_pack();
        pack[1].evidence_id = "ev_a";
        assert_eq!(
            inspect_evidence(TENANT, RegulatoryFramework::NcscCafGovAssure, &pack, NOW).unwrap_err(),
            EvidenceError::DuplicateEvidence
        );
        pack[1].evidence_id = "ev_b";
        pack[1].collected_at_unix = NOW + 10;
        assert_eq!(
            inspect_evidence(TENANT, RegulatoryFramework::NcscCafGovAssure, &pack, NOW).unwrap_err(),
            EvidenceError::InvalidInterval
        );
    }

    #[test]
    fn non_caf_framework_checks_integrity_but_never_claims_certification() {
        let mut evidence = artifact("soc2_control_1", CafObjective::ManagingSecurityRisk);
        evidence.framework = RegulatoryFramework::Soc2Type2;
        evidence.objective = None;
        let result = inspect_evidence(
            TENANT,
            RegulatoryFramework::Soc2Type2,
            &[evidence],
            NOW,
        )
        .unwrap();
        assert_eq!(result.state, EvidenceAssessmentState::IntegrityCheckedOnly);
        assert!(!result.certified_compliant);
    }

    #[test]
    fn no_artifacts_never_passes() {
        let result = inspect_evidence(
            TENANT,
            RegulatoryFramework::NcscCafGovAssure,
            &[],
            NOW,
        )
        .unwrap();
        assert_eq!(result.state, EvidenceAssessmentState::InsufficientEvidence);
        assert_eq!(result.missing_caf_objectives.len(), 4);
    }

    #[test]
    fn oversized_artifact_does_not_get_hashed_or_accepted() {
        let body = vec![0u8; MAX_ARTIFACT_BYTES + 1];
        let mut evidence = artifact("oversized", CafObjective::ManagingSecurityRisk);
        evidence.content = &body;
        let result = inspect_evidence(
            TENANT,
            RegulatoryFramework::NcscCafGovAssure,
            &[evidence],
            NOW,
        );
        assert_eq!(result.unwrap_err(), EvidenceError::ResourceLimit);
    }
}
