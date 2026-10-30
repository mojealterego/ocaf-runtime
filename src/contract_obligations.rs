//! Source PDF-1.31–1.33: deterministic, bounded contract obligation extraction.
//! Supports an explicit, intentionally narrow clause grammar only:
//! one normative "shall|must|will" and "within N days" per already-cited clause.
//! It does not interpret arbitrary legal language or execute a tool.
//! PDF-1.34 (evidence-backed fulfillment) requires independently validated
//! attestation and approval; it is NOT implemented here.
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAX_CLAUSE_BYTES: usize = 4096;
const MAX_WINDOW_DAYS: u64 = 3650;
const MAX_WARNING_SECS: u64 = 30 * 86400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationCategory {
    SecurityAssurance,
    DataProtection,
    SlaDelivery,
    FinancialReporting,
    GeneralCompliance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationLifecycle {
    ExtractedPending,
    ActionInProgress,
    VerifiedFulfilled,
    DeadlineBreached,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractObligation {
    pub obligation_id: String,
    pub tenant_id: String,
    pub contract_id: String,
    pub clause_citation: String,
    pub debtor_id: String,
    pub creditor_id: String,
    pub source_clause: String,
    /// Heuristic category is not legal or regulatory classification.
    pub category: ObligationCategory,
    pub reference_unix: u64,
    pub due_at_unix: u64,
    pub state: ObligationLifecycle,
}

#[derive(Debug, Clone, Copy)]
pub struct CitedClause<'a> {
    pub tenant_id: &'a str,
    pub contract_id: &'a str,
    pub clause_citation: &'a str,
    pub debtor_id: &'a str,
    pub creditor_id: &'a str,
    pub text: &'a str,
    pub reference_unix: u64,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ClauseError {
    #[error("invalid identity, citation, or clause size")]
    InvalidSource,
    #[error("ambiguous or unsupported normative clause grammar")]
    UnsupportedGrammar,
    #[error("deadline arithmetic overflow or unsupported duration")]
    InvalidDeadline,
    #[error("unsupported monitoring warning window")]
    InvalidWarning,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

fn canonical_hash(fields: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"OCAF-CONTRACT-OBLIGATION-v1");
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    format!("obl_{:x}", hasher.finalize())
}

fn classify(words: &[String]) -> ObligationCategory {
    if words.iter().any(|w| {
        matches!(
            w.as_str(),
            "certificate" | "security" | "penetration" | "cyber"
        )
    }) {
        ObligationCategory::SecurityAssurance
    } else if words
        .iter()
        .any(|w| matches!(w.as_str(), "data" | "gdpr" | "privacy"))
    {
        ObligationCategory::DataProtection
    } else if words
        .iter()
        .any(|w| matches!(w.as_str(), "delivery" | "service" | "sla"))
    {
        ObligationCategory::SlaDelivery
    } else if words
        .iter()
        .any(|w| matches!(w.as_str(), "finance" | "financial" | "audit"))
    {
        ObligationCategory::FinancialReporting
    } else {
        ObligationCategory::GeneralCompliance
    }
}

/// The caller supplies authenticated tenant/actor bindings and a trusted UTC
/// reference timestamp. No datetime inference or LLM extraction is performed.
/// Non-normative clauses return None; unsupported binding clauses error rather
/// than inventing a commitment or deadline.
pub fn extract_cited_obligation(
    clause: CitedClause<'_>,
) -> Result<Option<ContractObligation>, ClauseError> {
    if !valid_id(clause.tenant_id)
        || !valid_id(clause.contract_id)
        || !valid_id(clause.clause_citation)
        || !valid_id(clause.debtor_id)
        || !valid_id(clause.creditor_id)
        || clause.text.is_empty()
        || clause.text.len() > MAX_CLAUSE_BYTES
        || clause.text.contains('\0')
    {
        return Err(ClauseError::InvalidSource);
    }
    let words: Vec<String> = clause
        .text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    let norm_positions: Vec<usize> = words
        .iter()
        .enumerate()
        .filter(|(_, word)| matches!(word.as_str(), "shall" | "must" | "will"))
        .map(|(index, _)| index)
        .collect();
    if norm_positions.is_empty() {
        return Ok(None);
    }
    if norm_positions.len() != 1
        || words
            .get(norm_positions[0] + 1)
            .is_some_and(|word| word == "not")
    {
        return Err(ClauseError::UnsupportedGrammar);
    }
    let positions: Vec<usize> = words
        .iter()
        .enumerate()
        .filter(|(_, word)| word.as_str() == "within")
        .map(|(index, _)| index)
        .collect();
    if positions.len() != 1 {
        return Err(ClauseError::UnsupportedGrammar);
    }
    let pos = positions[0];
    let Some(days) = words.get(pos + 1).and_then(|v| v.parse::<u64>().ok()) else {
        return Err(ClauseError::UnsupportedGrammar);
    };
    if words.get(pos + 2).is_none_or(|unit| unit != "days") {
        return Err(ClauseError::UnsupportedGrammar);
    }
    if days == 0 || days > MAX_WINDOW_DAYS {
        return Err(ClauseError::InvalidDeadline);
    }
    let due_at_unix = clause
        .reference_unix
        .checked_add(
            days.checked_mul(86400)
                .ok_or(ClauseError::InvalidDeadline)?,
        )
        .ok_or(ClauseError::InvalidDeadline)?;
    let obligation_id = canonical_hash(&[
        clause.tenant_id.as_bytes(),
        clause.contract_id.as_bytes(),
        clause.clause_citation.as_bytes(),
        clause.debtor_id.as_bytes(),
        clause.creditor_id.as_bytes(),
        clause.text.as_bytes(),
        &clause.reference_unix.to_be_bytes(),
        &due_at_unix.to_be_bytes(),
    ]);
    Ok(Some(ContractObligation {
        obligation_id,
        tenant_id: clause.tenant_id.to_owned(),
        contract_id: clause.contract_id.to_owned(),
        clause_citation: clause.clause_citation.to_owned(),
        debtor_id: clause.debtor_id.to_owned(),
        creditor_id: clause.creditor_id.to_owned(),
        source_clause: clause.text.to_owned(),
        category: classify(&words),
        reference_unix: clause.reference_unix,
        due_at_unix,
        state: ObligationLifecycle::ExtractedPending,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineEvaluation {
    pub evaluated_state: ObligationLifecycle,
    pub hours_remaining: i128,
    pub is_at_risk: bool,
    pub is_breached: bool,
}

/// Deterministic evaluation using an explicit trusted instant and warning bound.
/// A prior breach is sticky even if clocks move backwards. Only a separate,
/// authorized evidence-backed verifier may set VerifiedFulfilled.
pub fn evaluate_deadline(
    obligation: &ContractObligation,
    now_unix: u64,
    warning_secs: u64,
) -> Result<DeadlineEvaluation, ClauseError> {
    if warning_secs > MAX_WARNING_SECS {
        return Err(ClauseError::InvalidWarning);
    }
    let remaining = i128::from(obligation.due_at_unix) - i128::from(now_unix);
    let evaluated_state = match obligation.state {
        ObligationLifecycle::VerifiedFulfilled => ObligationLifecycle::VerifiedFulfilled,
        ObligationLifecycle::DeadlineBreached => ObligationLifecycle::DeadlineBreached,
        _ if remaining <= 0 => ObligationLifecycle::DeadlineBreached,
        previous => previous,
    };
    let is_breached = evaluated_state == ObligationLifecycle::DeadlineBreached;
    let is_at_risk = !is_breached
        && evaluated_state != ObligationLifecycle::VerifiedFulfilled
        && remaining <= i128::from(warning_secs);
    Ok(DeadlineEvaluation {
        evaluated_state,
        hours_remaining: remaining / 3600,
        is_at_risk,
        is_breached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REF: u64 = 1_800_000_000;
    fn cited(text: &'static str) -> CitedClause<'static> {
        CitedClause {
            tenant_id: "tenant_a",
            contract_id: "agreement_1",
            clause_citation: "clause_4_2",
            debtor_id: "supplier",
            creditor_id: "customer",
            text,
            reference_unix: REF,
        }
    }

    fn extracted() -> ContractObligation {
        extract_cited_obligation(cited(
            "Supplier shall provide a security certificate within 14 days.",
        ))
        .unwrap()
        .unwrap()
    }

    #[test]
    fn extracts_deterministic_bounded_obligation_and_sha_id() {
        let first = extracted();
        let second = extracted();
        assert_eq!(first, second);
        assert_eq!(first.due_at_unix, REF + 14 * 86400);
        assert_eq!(first.category, ObligationCategory::SecurityAssurance);
        assert_eq!(first.obligation_id.len(), 68);
        assert_eq!(first.state, ObligationLifecycle::ExtractedPending);
    }

    #[test]
    fn no_modal_returns_none_without_fabricating_an_obligation() {
        let result = extract_cited_obligation(cited(
            "The parties may discuss a renewed security certificate.",
        ))
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn negative_normative_and_multiple_modal_clauses_fail_closed() {
        assert_eq!(
            extract_cited_obligation(cited("Supplier must not disclose data within 10 days."))
                .unwrap_err(),
            ClauseError::UnsupportedGrammar
        );
        assert_eq!(
            extract_cited_obligation(cited("Supplier shall and must report within 10 days."))
                .unwrap_err(),
            ClauseError::UnsupportedGrammar
        );
    }

    #[test]
    fn missing_or_malformed_deadlines_cannot_be_guessed() {
        for text in [
            "Supplier shall produce reports promptly.",
            "Supplier shall report within two days.",
            "Supplier will act within 3 months.",
            "Supplier shall act within 2 days and within 3 days.",
        ] {
            assert_eq!(
                extract_cited_obligation(cited(text)).unwrap_err(),
                ClauseError::UnsupportedGrammar
            );
        }
    }

    #[test]
    fn rejects_zero_huge_or_overflowing_deadlines() {
        for text in [
            "Supplier must report within 0 days.",
            "Supplier must report within 3651 days.",
        ] {
            assert_eq!(
                extract_cited_obligation(cited(text)).unwrap_err(),
                ClauseError::InvalidDeadline
            );
        }
        let mut clause = cited("Supplier must report within 10 days.");
        clause.reference_unix = u64::MAX - 1;
        assert_eq!(
            extract_cited_obligation(clause).unwrap_err(),
            ClauseError::InvalidDeadline
        );
    }

    #[test]
    fn tenant_and_citation_change_resulting_id() {
        let first = extracted();
        let mut clause = cited("Supplier shall provide a security certificate within 14 days.");
        clause.tenant_id = "tenant_b";
        let another = extract_cited_obligation(clause).unwrap().unwrap();
        assert_ne!(first.obligation_id, another.obligation_id);
        assert_eq!(another.tenant_id, "tenant_b");
    }

    #[test]
    fn deadline_monitor_flags_risk_and_hard_breach_at_exact_expiry() {
        let record = extracted();
        let early = evaluate_deadline(&record, record.due_at_unix - 7 * 86400, 3 * 86400).unwrap();
        assert!(!early.is_at_risk);
        let warning = evaluate_deadline(&record, record.due_at_unix - 3600, 3 * 86400).unwrap();
        assert!(warning.is_at_risk);
        assert_eq!(warning.hours_remaining, 1);
        let expired = evaluate_deadline(&record, record.due_at_unix, 3 * 86400).unwrap();
        assert!(expired.is_breached);
        assert_eq!(
            expired.evaluated_state,
            ObligationLifecycle::DeadlineBreached
        );
    }

    #[test]
    fn verified_and_breached_lifecycle_states_are_monotonic() {
        let mut obligation = extracted();
        obligation.state = ObligationLifecycle::DeadlineBreached;
        assert!(
            evaluate_deadline(&obligation, REF, 86400)
                .unwrap()
                .is_breached
        );
        obligation.state = ObligationLifecycle::VerifiedFulfilled;
        let result = evaluate_deadline(&obligation, obligation.due_at_unix + 10, 86400).unwrap();
        assert!(!result.is_breached);
        assert!(!result.is_at_risk);
    }

    #[test]
    fn warning_window_and_source_size_are_bounded() {
        let obligation = extracted();
        assert_eq!(
            evaluate_deadline(&obligation, REF, MAX_WARNING_SECS + 1).unwrap_err(),
            ClauseError::InvalidWarning
        );
        let too_long = "x".repeat(MAX_CLAUSE_BYTES + 1);
        let clause = CitedClause {
            text: &too_long,
            ..cited("shall within 1 days")
        };
        assert_eq!(
            extract_cited_obligation(clause).unwrap_err(),
            ClauseError::InvalidSource
        );
    }
}
