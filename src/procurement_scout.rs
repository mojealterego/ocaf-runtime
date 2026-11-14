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
    MissingBeneficialOwnerIdentity,
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
    #[error("supplier did not meet the bounded local purchase-intent criteria")]
    SupplierNotQualified,
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
    if supplier.beneficial_owners.is_empty() {
        findings.push(Finding::MissingBeneficialOwnerIdentity);
    }
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

/// A deterministic *proposed* purchase intent, not an order, payment, audit
/// receipt or approval. Its digest is bound to the exact vetted dossier,
/// operator policy, and the separately pinned watchlist snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchaseIntent {
    action: &'static str,
    tenant: String,
    supplier_id: String,
    /// USD cents, never floating point and never selected as a policy cap.
    amount_cents: u64,
    dossier_sha256: [u8; 32],
    watchlist_sha256: [u8; 32],
    policy_sha256: [u8; 32],
    payload_sha256: [u8; 32],
    /// Never true here: only a separate authenticated execution controller
    /// can decide to commit an external effect after an operator approval.
    purchase_authorized: bool,
}

impl PurchaseIntent {
    /// Fields remain private: only `prepare_purchase_intent` can construct
    /// a valid digest-bound intent; downstream callers receive read-only views.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    pub fn supplier_id(&self) -> &str {
        &self.supplier_id
    }

    pub fn amount_cents(&self) -> u64 {
        self.amount_cents
    }

    pub fn payload_sha256(&self) -> [u8; 32] {
        self.payload_sha256
    }

    pub fn is_purchase_authorized(&self) -> bool {
        self.purchase_authorized
    }

    /// Construct the canonical action for the existing governance/journal
    /// gates. Caller supplies authenticated actor/request ID and an approval
    /// issued by an independent human approval service. The separately verified
    /// provider access must also be supplied by the authenticated ingress,
    /// never from untrusted supplier or model output.
    ///
    /// This builder cannot mint approvals, reserve durable idempotency, or
    /// execute any tool. GovernanceJournal::begin still needs trusted peer
    /// identity, a verified approval authority, and independent executor gates.
    pub fn to_action_request<'a>(
        &'a self,
        request_id: &'a str,
        actor: &'a str,
        provider: Option<&'a crate::governance_contract::ProviderAccess>,
        approval: Option<&'a crate::governance_contract::ApprovalArtifact>,
    ) -> crate::governance_contract::ActionRequest<'a> {
        crate::governance_contract::ActionRequest {
            request_id,
            tenant: &self.tenant,
            actor,
            action: self.action,
            domain: crate::governance_contract::Domain::ProcurementScout,
            risk: crate::governance_contract::RiskTier::Medium,
            payload_sha256: self.payload_sha256,
            uses_external_provider: true,
            provider,
            approval,
        }
    }
}

fn digest_policy(policy: ProcurementPolicy) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"OCAF-PROCUREMENT-POLICY-v1");
    hash_field(&mut digest, &policy.medium_risk_cap_cents.to_be_bytes());
    hash_field(&mut digest, &[policy.min_financial_score]);
    hash_field(&mut digest, &policy.max_feed_age_secs.to_be_bytes());
    hash_field(&mut digest, &[u8::from(policy.require_iso27001_claim)]);
    digest.finalize().into()
}

fn hash_optional_expiry(digest: &mut Sha256, expiry: Option<u64>) {
    match expiry {
        Some(timestamp) => {
            hash_field(digest, b"some");
            hash_field(digest, &timestamp.to_be_bytes());
        }
        None => hash_field(digest, b"none"),
    }
}

fn digest_dossier(dossier: &SupplierDossier<'_>) -> Result<[u8; 32], ProcurementError> {
    let name = canonical_name(dossier.legal_name)?;
    let owners = dossier
        .beneficial_owners
        .iter()
        .map(|owner| canonical_name(owner))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut digest = Sha256::new();
    digest.update(b"OCAF-PROCUREMENT-DOSSIER-v1");
    for value in [
        dossier.tenant.as_bytes(),
        dossier.supplier_id.as_bytes(),
        name.as_bytes(),
        b"USD",
        &dossier.proposed_commitment_cents.to_be_bytes(),
    ] {
        hash_field(&mut digest, value);
    }
    hash_field(&mut digest, &(owners.len() as u64).to_be_bytes());
    for owner in owners {
        hash_field(&mut digest, owner.as_bytes());
    }
    hash_optional_expiry(&mut digest, dossier.tax_evidence_expires);
    hash_optional_expiry(&mut digest, dossier.insurance_evidence_expires);
    hash_field(&mut digest, &[u8::from(dossier.iso27001_claimed)]);
    hash_field(&mut digest, &[dossier.financial_score]);
    Ok(digest.finalize().into())
}

/// Prepare a proposed purchase from the *same* supplier dossier and trusted
/// policy used during screening. No caller-supplied maximum amount is accepted.
/// A REVIEW_NEEDED or NOT_QUALIFIED supplier cannot produce an intent.
/// This never authorizes an order, even for a PassCandidate.
pub fn prepare_purchase_intent(
    trusted_tenant: &str,
    trusted_now: u64,
    policy: ProcurementPolicy,
    supplier: &SupplierDossier<'_>,
    feed: &WatchlistSnapshot<'_>,
    independent_digest_pin: [u8; 32],
) -> Result<PurchaseIntent, ProcurementError> {
    if supplier.proposed_commitment_cents == 0 {
        return Err(ProcurementError::InvalidInput);
    }
    let assessment = assess_supplier(
        trusted_tenant,
        trusted_now,
        policy,
        supplier,
        feed,
        independent_digest_pin,
    )?;
    if assessment.recommendation != Recommendation::PassCandidate {
        return Err(ProcurementError::SupplierNotQualified);
    }
    let dossier_sha256 = digest_dossier(supplier)?;
    let policy_sha256 = digest_policy(policy);
    let mut payload = Sha256::new();
    payload.update(b"OCAF-PROCUREMENT-PURCHASE-INTENT-v1");
    for bytes in [
        &b"procurement.commit_purchase_order"[..],
        trusted_tenant.as_bytes(),
        supplier.supplier_id.as_bytes(),
        b"USD",
        &supplier.proposed_commitment_cents.to_be_bytes(),
        dossier_sha256.as_slice(),
        independent_digest_pin.as_slice(),
        policy_sha256.as_slice(),
    ] {
        hash_field(&mut payload, bytes);
    }
    Ok(PurchaseIntent {
        action: "procurement.commit_purchase_order",
        tenant: trusted_tenant.to_owned(),
        supplier_id: supplier.supplier_id.to_owned(),
        amount_cents: supplier.proposed_commitment_cents,
        dossier_sha256,
        watchlist_sha256: independent_digest_pin,
        policy_sha256,
        payload_sha256: payload.finalize().into(),
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
    fn purchase_intent_matches_independently_calculated_sha256_test_vector() {
        // Python stdlib hashlib reference; a model check, not Rust runtime evidence.
        let records = [RECORD];
        let snapshot = feed(&records);
        let owners = ["KNOWN OWNER"];
        let intent = prepare_purchase_intent(
            "tenant-A",
            NOW,
            policy(),
            &supplier(&owners),
            &snapshot,
            digest_snapshot(&snapshot).unwrap(),
        )
        .unwrap();
        assert_eq!(intent.watchlist_sha256, [0xe0, 0xa2, 0x39, 0x7d, 0xb9, 0xb2, 0xb9, 0x5b, 0x8f, 0x34, 0x87, 0xdf, 0xf8, 0xd0, 0x48, 0x3c, 0x6c, 0xc8, 0x4b, 0x5c, 0xb9, 0xf9, 0x6f, 0x0f, 0x48, 0x7d, 0xda, 0x26, 0xa1, 0xde, 0x01, 0x55]);
        assert_eq!(intent.policy_sha256, [0xa2, 0xd7, 0x4b, 0xb6, 0xb4, 0xa5, 0xdb, 0xf6, 0x72, 0x50, 0x51, 0x42, 0x88, 0x84, 0x4d, 0xe7, 0x40, 0xfa, 0x14, 0x3b, 0xc4, 0x88, 0x9c, 0x61, 0xf0, 0x41, 0x25, 0xd9, 0xa9, 0x5d, 0x14, 0x0a]);
        assert_eq!(intent.dossier_sha256, [0xee, 0x4f, 0x0b, 0xbc, 0xdc, 0x34, 0xae, 0x70, 0xd8, 0xdf, 0x30, 0xb2, 0xac, 0x13, 0xdc, 0x29, 0x22, 0x39, 0x51, 0xcb, 0xc7, 0x7c, 0xad, 0x76, 0x81, 0x63, 0x2b, 0x11, 0xc6, 0xb8, 0xe1, 0x15]);
        assert_eq!(intent.payload_sha256, [0x61, 0x52, 0xf3, 0xef, 0xa2, 0x2f, 0xd5, 0x91, 0x12, 0xec, 0x37, 0x1d, 0x9a, 0xb8, 0xe9, 0x60, 0x59, 0xcf, 0x9e, 0xe3, 0x89, 0xa8, 0xa8, 0xce, 0xf0, 0xed, 0x03, 0xd4, 0x4c, 0x9c, 0x49, 0x89]);
    }

    #[test]
    fn purchase_intent_binds_exact_supplier_amount_and_operator_policy() {
        let records = [RECORD];
        let snapshot = feed(&records);
        let owners = ["KNOWN OWNER"];
        let base = supplier(&owners);
        let first = prepare_purchase_intent(
            "tenant-A", NOW, policy(), &base, &snapshot, digest_snapshot(&snapshot).unwrap()
        ).unwrap();
        assert_eq!(first.amount_cents, 9_000);
        assert_eq!(first.tenant, "tenant-A");
        assert_eq!(first.supplier_id, "supplier-1");
        assert_eq!(first.action, "procurement.commit_purchase_order");
        assert!(!first.purchase_authorized);
        let request = first.to_action_request("req-1", "agent-1", None, None);
        assert_eq!(request.domain, crate::governance_contract::Domain::ProcurementScout);
        assert_eq!(request.risk, crate::governance_contract::RiskTier::Medium);
        assert_eq!(request.payload_sha256, first.payload_sha256);

        let again = prepare_purchase_intent(
            "tenant-A", NOW, policy(), &base, &snapshot, digest_snapshot(&snapshot).unwrap()
        ).unwrap();
        assert_eq!(first.payload_sha256, again.payload_sha256);
        let mut changed_amount = supplier(&owners);
        changed_amount.proposed_commitment_cents += 1;
        let after_amount = prepare_purchase_intent(
            "tenant-A", NOW, policy(), &changed_amount, &snapshot, digest_snapshot(&snapshot).unwrap()
        ).unwrap();
        assert_ne!(first.payload_sha256, after_amount.payload_sha256);
        let mut changed_policy = policy();
        changed_policy.medium_risk_cap_cents += 1;
        let after_policy = prepare_purchase_intent(
            "tenant-A", NOW, changed_policy, &base, &snapshot, digest_snapshot(&snapshot).unwrap()
        ).unwrap();
        assert_ne!(first.payload_sha256, after_policy.payload_sha256);
        let mut changed_owner = supplier(&owners);
        changed_owner.beneficial_owners = &["DIFFERENT OWNER"];
        let after_owner = prepare_purchase_intent(
            "tenant-A", NOW, policy(), &changed_owner, &snapshot, digest_snapshot(&snapshot).unwrap()
        ).unwrap();
        assert_ne!(first.payload_sha256, after_owner.payload_sha256);
    }

    #[test]
    fn purchase_intent_is_denied_on_nonqualified_supplier_or_zero_amount() {
        let records = [RECORD];
        let snapshot = feed(&records);
        let pin = digest_snapshot(&snapshot).unwrap();
        let mut dossier = supplier(&[]);
        assert_eq!(
            prepare_purchase_intent("tenant-A", NOW, policy(), &dossier, &snapshot, pin).unwrap_err(),
            ProcurementError::SupplierNotQualified
        );
        dossier.beneficial_owners = &["KNOWN OWNER"];
        dossier.proposed_commitment_cents = 0;
        assert_eq!(
            prepare_purchase_intent("tenant-A", NOW, policy(), &dossier, &snapshot, pin).unwrap_err(),
            ProcurementError::InvalidInput
        );
        dossier.proposed_commitment_cents = 10_001;
        assert_eq!(
            prepare_purchase_intent("tenant-A", NOW, policy(), &dossier, &snapshot, pin).unwrap_err(),
            ProcurementError::SupplierNotQualified
        );
        dossier.proposed_commitment_cents = 9_000;
        assert_eq!(
            prepare_purchase_intent("tenant-B", NOW, policy(), &dossier, &snapshot, pin).unwrap_err(),
            ProcurementError::InvalidInput
        );
    }

    #[test]
    fn purchase_commitment_requests_require_external_provider_authorization() {
        let records = [RECORD];
        let snapshot = feed(&records);
        let intent = prepare_purchase_intent(
            "tenant-A", NOW, policy(), &supplier(&["KNOWN OWNER"]),
            &snapshot, digest_snapshot(&snapshot).unwrap()
        ).unwrap();
        let request = intent.to_action_request("req-provider", "agent-1", None, None);
        assert!(request.uses_external_provider);
        assert!(request.provider.is_none());
    }

    #[test]
    fn purchase_intent_cannot_be_admitted_without_human_approval() {
        use crate::governance_contract::{Accountability, AgentContract, DenyReason, SocialCommitment};
        use crate::governance_journal::{Admission, GovernanceJournal};
        let records = [RECORD];
        let snapshot = feed(&records);
        let intent = prepare_purchase_intent(
            "tenant-A", NOW, policy(), &supplier(&["OWNER LLC"]),
            &snapshot, digest_snapshot(&snapshot).unwrap()
        ).unwrap();
        let provider = crate::governance_contract::ProviderAccess {
            authenticated: true,
            region_model_authorized: true,
            explicit_cost_approved: true,
            estimated_cost_micros: 1,
            approved_cost_micros: 1,
        };
        let request = intent.to_action_request("request-1", "agent-1", Some(&provider), None);
        let agent_contract = AgentContract {
            contract_id: "contract-1".into(),
            version: "1".into(),
            tenant: "tenant-A".into(),
            actor: "agent-1".into(),
            obligations: vec![SocialCommitment {
                debtor: "agent-1".into(),
                creditor: "operator".into(),
                condition: "Explicit human purchase authorization".into(),
            }],
            permissions: std::collections::BTreeSet::from([intent.action.into()]),
            prohibitions: std::collections::BTreeSet::new(),
            accountability: Accountability {
                owner: "operator".into(),
                escalation_reviewer: "reviewer".into(),
                audit_authority: "auditor".into(),
            },
        };
        let database = tempfile::NamedTempFile::new().unwrap();
        let journal = GovernanceJournal::open_existing(database.path()).unwrap();
        let missing_provider = intent.to_action_request("request-1", "agent-1", None, None);
        assert_eq!(
            journal.begin(
                &agent_contract, &missing_provider, None,
                ("tenant-A", "agent-1"), NOW, "order-idem"
            ).unwrap(),
            Admission::Denied(DenyReason::ProviderNotAuthorized)
        );
        assert_eq!(
            journal.begin(
                &agent_contract, &request, None,
                ("tenant-A", "agent-1"), NOW, "order-idem"
            ).unwrap(),
            Admission::Denied(DenyReason::MissingApproval)
        );
        assert_eq!(journal.inspect("tenant-A", None).unwrap().sequence, 2);
        assert!(!intent.purchase_authorized);
    }

    #[test]
    fn non_ascii_names_are_not_marked_clear() {
        let mut dossier = supplier(&[]);
        dossier.legal_name = "ŻURAW SA";
        assert_eq!(assess(&dossier, &feed(&[RECORD])).unwrap_err(), ProcurementError::UnsupportedName);
    }
}
