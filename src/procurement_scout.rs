//! Argon PDF-1.43/1.44/1.46 — bounded read-only Procurement Scout preflight.
//!
//! This does not authenticate sanctions publishers, authorize purchases or clear
//! suppliers. Caller MUST provide tenant, evaluation time and digest pin from
//! a trusted channel independent of the supplier/agent request.
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

const MAX_ENTRIES: usize = 65_536;
const MAX_OWNERS: usize = 64;
const MAX_NAME_BYTES: usize = 160;
const MAX_FEED_LIFETIME: u64 = 7 * 86_400;

#[derive(Clone, Copy, Debug)]
pub struct ProcurementPolicy {
    /// Trusted policy limit in integer USD cents, not supplier-controlled.
    pub medium_risk_cap_cents: u64,
    pub min_financial_score: u8,
    pub max_feed_age_secs: u64,
    pub require_iso27001_claim: bool,
}

#[derive(Clone, Debug)]
pub struct SupplierDossier<'a> {
    pub tenant: &'a str,
    pub supplier_id: &'a str,
    pub legal_name: &'a str,
    pub beneficial_owners: &'a [&'a str],
    pub tax_evidence_expires: Option<u64>,
    pub insurance_evidence_expires: Option<u64>,
    /// Untrusted declaration until independently evidenced and attested.
    pub iso27001_claimed: bool,
    pub financial_score: u8,
    pub proposed_commitment_cents: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct WatchlistRecord<'a> {
    pub listing_id: &'a str,
    pub entity_name: &'a str,
}

#[derive(Clone, Debug)]
pub struct WatchlistSnapshot<'a> {
    pub tenant: &'a str,
    pub issuer: &'a str,
    pub issued_at: u64,
    pub expires_at: u64,
    pub records: &'a [WatchlistRecord<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recommendation {
    PassCandidate,
    ReviewNeeded,
    NotQualified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finding {
    PossibleWatchlistMatch,
    AboveMediumRiskCap,
    MissingTaxEvidence,
    MissingInsuranceEvidence,
    InsufficientFinancialScore,
    MissingIso27001Claim,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcurementAssessment {
    pub recommendation: Recommendation,
    pub findings: Vec<Finding>,
    pub possible_listing_ids: Vec<String>,
    pub human_attestation_required: bool,
    pub purchase_authorized: bool,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ProcurementError {
    #[error("invalid or cross-tenant procurement input")]
    InvalidInput,
    #[error("name cannot be screened by exact ASCII matching")]
    UnsupportedName,
    #[error("watchlist or owner resource bound exceeded")]
    ResourceLimit,
    #[error("duplicate listing identifier")]
    DuplicateListing,
    #[error("watchlist digest does not match independent trusted pin")]
    BadSnapshotDigest,
    #[error("watchlist is expired, stale or from the future")]
    StaleSnapshot,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !s.starts_with('.')
        && !s.ends_with('.')
        && !s.contains("..")
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

fn canonical_name(name: &str) -> Result<String, ProcurementError> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(ProcurementError::UnsupportedName);
    }
    let words: Vec<_> = name.split_ascii_whitespace().collect();
    if words.is_empty()
        || words.iter().any(|word| {
            !word.bytes().all(|b| {
                b.is_ascii_alphanumeric() || matches!(b, b'-' | b'&' | b'.' | b'\'')
            })
        })
    {
        return Err(ProcurementError::UnsupportedName);
    }
    Ok(words.join(" ").to_ascii_uppercase())
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// Canonical, order-invariant and domain-separated SHA-256 over the full feed.
/// This hash is NOT a signature and is useless as authority without a trusted pin.
pub fn digest_snapshot(feed: &WatchlistSnapshot<'_>) -> Result<[u8; 32], ProcurementError> {
    if !valid_id(feed.tenant)
        || !valid_id(feed.issuer)
        || feed.issued_at == 0
        || feed.expires_at <= feed.issued_at
    {
        return Err(ProcurementError::InvalidInput);
    }
    if feed.records.is_empty() || feed.records.len() > MAX_ENTRIES {
        return Err(ProcurementError::ResourceLimit);
    }
    let mut records = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for record in feed.records {
        if !valid_id(record.listing_id) {
            return Err(ProcurementError::InvalidInput);
        }
        if !ids.insert(record.listing_id) {
            return Err(ProcurementError::DuplicateListing);
        }
        records.insert((record.listing_id, canonical_name(record.entity_name)?));
    }
    let mut h = Sha256::new();
    h.update(b"OCAF-PROCUREMENT-FEED-v1");
    for bytes in [
        feed.tenant.as_bytes(),
        feed.issuer.as_bytes(),
        &feed.issued_at.to_be_bytes(),
        &feed.expires_at.to_be_bytes(),
        &(records.len() as u64).to_be_bytes(),
    ] {
        hash_field(&mut h, bytes);
    }
    for (id, name) in records {
        hash_field(&mut h, id.as_bytes());
        hash_field(&mut h, name.as_bytes());
    }
    Ok(h.finalize().into())
}

/// Read-only assessment; never treats a non-match as sanctions clearance.
pub fn assess_supplier(
    trusted_tenant: &str,
    trusted_now: u64,
    policy: ProcurementPolicy,
    supplier: &SupplierDossier<'_>,
    feed: &WatchlistSnapshot<'_>,
    independent_digest_pin: [u8; 32],
) -> Result<ProcurementAssessment, ProcurementError> {
    if !valid_id(trusted_tenant)
        || !valid_id(supplier.tenant)
        || !valid_id(supplier.supplier_id)
        || supplier.tenant != trusted_tenant
        || feed.tenant != trusted_tenant
        || supplier.beneficial_owners.len() > MAX_OWNERS
        || supplier.financial_score > 100
        || policy.min_financial_score > 100
        || policy.medium_risk_cap_cents == 0
        || policy.max_feed_age_secs == 0
        || policy.max_feed_age_secs > MAX_FEED_LIFETIME
    {
        return Err(ProcurementError::InvalidInput);
    }
    let supplier_name = canonical_name(supplier.legal_name)?;
    let owners = supplier
        .beneficial_owners
        .iter()
        .map(|name| canonical_name(name))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if independent_digest_pin == [0; 32] || digest_snapshot(feed)? != independent_digest_pin {
        return Err(ProcurementError::BadSnapshotDigest);
    }
    if feed.issued_at > trusted_now
        || feed.expires_at <= trusted_now
        || feed.expires_at - feed.issued_at > MAX_FEED_LIFETIME
        || trusted_now - feed.issued_at > policy.max_feed_age_secs
    {
        return Err(ProcurementError::StaleSnapshot);
    }
    let possible_listing_ids = feed
        .records
        .iter()
        .filter_map(|record| {
            let name = canonical_name(record.entity_name).ok()?;
            (supplier_name == name || owners.contains(&name))
                .then(|| record.listing_id.to_owned())
        })
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    if !possible_listing_ids.is_empty() {
        findings.push(Finding::PossibleWatchlistMatch);
    }
    if supplier.proposed_commitment_cents > policy.medium_risk_cap_cents {
        findings.push(Finding::AboveMediumRiskCap);
    }
    if supplier.tax_evidence_expires.is_none_or(|t| t <= trusted_now) {
        findings.push(Finding::MissingTaxEvidence);
    }
    if supplier.insurance_evidence_expires.is_none_or(|t| t <= trusted_now) {
        findings.push(Finding::MissingInsuranceEvidence);
    }
    if supplier.financial_score < policy.min_financial_score {
        findings.push(Finding::InsufficientFinancialScore);
    }
    if policy.require_iso27001_claim && !supplier.iso27001_claimed {
        findings.push(Finding::MissingIso27001Claim);
    }
    let recommendation = if findings.iter().any(|finding| {
        matches!(
            finding,
            Finding::PossibleWatchlistMatch | Finding::AboveMediumRiskCap
        )
    }) {
        Recommendation::NotQualified
    } else if findings.is_empty() {
        Recommendation::PassCandidate
    } else {
        Recommendation::ReviewNeeded
    };
    Ok(ProcurementAssessment {
        recommendation,
        findings,
        possible_listing_ids,
        human_attestation_required: true,
        purchase_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: u64 = 1_800_000_000;
    const RECORD: WatchlistRecord<'static> =
        WatchlistRecord { listing_id: "w1", entity_name: "OTHER LTD" };

    fn policy() -> ProcurementPolicy {
        ProcurementPolicy {
            medium_risk_cap_cents: 10_000,
            min_financial_score: 60,
            max_feed_age_secs: 86_400,
            require_iso27001_claim: true,
        }
    }
    fn supplier<'a>(owners: &'a [&'a str]) -> SupplierDossier<'a> {
        SupplierDossier {
            tenant: "tenant-A",
            supplier_id: "supplier-1",
            legal_name: "ACME INC",
            beneficial_owners: owners,
            tax_evidence_expires: Some(NOW + 1),
            insurance_evidence_expires: Some(NOW + 1),
            iso27001_claimed: true,
            financial_score: 80,
            proposed_commitment_cents: 9_000,
        }
    }
    fn feed<'a>(records: &'a [WatchlistRecord<'a>]) -> WatchlistSnapshot<'a> {
        WatchlistSnapshot {
            tenant: "tenant-A",
            issuer: "trusted-upstream",
            issued_at: NOW - 60,
            expires_at: NOW + 3600,
            records,
        }
    }
    fn assess(
        dossier: &SupplierDossier<'_>,
        snapshot: &WatchlistSnapshot<'_>,
    ) -> Result<ProcurementAssessment, ProcurementError> {
        assess_supplier("tenant-A", NOW, policy(), dossier, snapshot, digest_snapshot(snapshot)?)
    }

    #[test]
    fn candidate_never_authorizes_purchase() {
        let r = assess(&supplier(&["KNOWN OWNER"]), &feed(&[RECORD])).unwrap();
        assert_eq!(r.recommendation, Recommendation::PassCandidate);
        assert!(r.human_attestation_required);
        assert!(!r.purchase_authorized);
    }
    #[test]
    fn missing_beneficial_owner_identity_requires_manual_review() {
        let result = assess(&supplier(&[]), &feed(&[RECORD])).unwrap();
        assert_eq!(result.recommendation, Recommendation::ReviewNeeded);
        assert!(result.findings.contains(&Finding::MissingBeneficialOwnerIdentity));
        assert!(result.human_attestation_required);
        assert!(!result.purchase_authorized);
    }
    #[test]
    fn supplier_exact_match_is_blocked() {
        let records = [WatchlistRecord { listing_id: "w2", entity_name: "  acme  inc " }];
        let r = assess(&supplier(&[]), &feed(&records)).unwrap();
        assert_eq!(r.recommendation, Recommendation::NotQualified);
        assert_eq!(r.possible_listing_ids, vec!["w2"]);
    }
    #[test]
    fn owner_exact_match_is_blocked() {
        let records = [WatchlistRecord { listing_id: "w2", entity_name: "OWNER LLC" }];
        let r = assess(&supplier(&["Owner LLC"]), &feed(&records)).unwrap();
        assert_eq!(r.recommendation, Recommendation::NotQualified);
    }
    #[test]
    fn cross_tenant_input_fails_closed() {
        let mut dossier = supplier(&[]);
        dossier.tenant = "tenant-B";
        assert_eq!(assess(&dossier, &feed(&[RECORD])).unwrap_err(), ProcurementError::InvalidInput);
    }
    #[test]
    fn feed_tampering_is_rejected_by_independent_pin() {
        let a = [RECORD];
        let b = [WatchlistRecord { listing_id: "w1", entity_name: "ACME INC" }];
        let pinned = digest_snapshot(&feed(&a)).unwrap();
        assert_eq!(
            assess_supplier("tenant-A", NOW, policy(), &supplier(&[]), &feed(&b), pinned)
                .unwrap_err(),
            ProcurementError::BadSnapshotDigest
        );
    }
    #[test]
    fn expired_feed_fails_closed() {
        let records = [RECORD];
        let mut snapshot = feed(&records);
        snapshot.expires_at = NOW;
        assert_eq!(assess(&supplier(&[]), &snapshot).unwrap_err(), ProcurementError::StaleSnapshot);
    }
    #[test]
    fn over_cap_commitment_cannot_be_approved() {
        let mut dossier = supplier(&[]);
        dossier.proposed_commitment_cents = 10_001;
        let r = assess(&dossier, &feed(&[RECORD])).unwrap();
        assert_eq!(r.recommendation, Recommendation::NotQualified);
        assert!(!r.purchase_authorized);
    }
    #[test]
    fn missing_documents_require_review() {
        let mut dossier = supplier(&[]);
        dossier.tax_evidence_expires = None;
        let r = assess(&dossier, &feed(&[RECORD])).unwrap();
        assert_eq!(r.recommendation, Recommendation::ReviewNeeded);
    }
    #[test]
    fn feed_reordering_is_hash_stable() {
        let a = [RECORD, WatchlistRecord { listing_id: "w2", entity_name: "ABCD LLC" }];
        let b = [a[1], a[0]];
        assert_eq!(digest_snapshot(&feed(&a)).unwrap(), digest_snapshot(&feed(&b)).unwrap());
    }
    #[test]
    fn malformed_or_duplicate_feed_cannot_clear_a_supplier() {
        let d = [RECORD, WatchlistRecord { listing_id: "w1", entity_name: "NEW INC" }];
        assert_eq!(digest_snapshot(&feed(&d)).unwrap_err(), ProcurementError::DuplicateListing);
        assert_eq!(digest_snapshot(&feed(&[])).unwrap_err(), ProcurementError::ResourceLimit);
    }
    #[test]
    fn non_ascii_names_are_not_marked_clear() {
        let mut dossier = supplier(&[]);
        dossier.legal_name = "ŻURAW SA";
        assert_eq!(assess(&dossier, &feed(&[RECORD])).unwrap_err(), ProcurementError::UnsupportedName);
    }
}
