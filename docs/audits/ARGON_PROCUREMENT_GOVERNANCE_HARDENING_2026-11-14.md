# PDF-1.45 — Procurement governed-action spoofing hardening (2026-11-14)

## Scope and source mapping
Argon PDF section 1.45 requires explicit Procurement Scout governance and limits purchase commitments to at most MEDIUM risk with human attestation. Source actions: `procurement.evaluate_supplier_dossier` and `procurement.commit_purchase_order`. Canonical OCAF stages are unchanged.

## Defect confirmed by source inspection
Prior to this change `AgentContract::evaluate` bound canonical domain/risk only for contract extraction and verification. It accepted the allowlisted `procurement.commit_purchase_order` name with a caller-declared `CoreRetrieval/Low` classification and returned `Continue`, bypassing procurement human-approval preflight. Since `Continue` is additional preflight rather than an executor authorization, this is a critical missing defense-in-depth gate, not evidence of an externally executed unauthorized purchase.

## Changes on main
- `src/governance_contract.rs`: binds the named contract, compliance and procurement actions to canonical domains and risk tiers; unknown actions starting with `procurement.`, `contract.` or `compliance.` fail closed even if accidentally allowlisted.
- `procurement.commit_purchase_order` must declare `ProcurementScout` and risk at least `Medium`. Procurement high/critical risk is still prohibited; medium risk requires a cryptographically verified approval with the proper human role.
- `procurement.evaluate_supplier_dossier` must declare `ProcurementScout/Low`. `compliance.validate_evidence_pack` must declare `ComplianceEvidence/Low`; the previously governed contract action shapes remain enforced.
- Three dedicated action-spoofing tests and one unknown-action test were added to `governance_contract.rs`. A new SQLite journal test in `governance_journal.rs` verifies that low-risk/domain spoofing is rejected with durable DENY before any PENDING external-effect intent is reserved, and that an untrusted tenant cannot silently reattribute admission.

## Verification and remaining gates
- Source/static assertions: known action checks and tests exist, unknown scoped names are rejected, workflow includes both governance test selections. **Static only**; this does not establish Rust compiler correctness.
- Rust toolchain pinned by `rust-toolchain.toml`: 1.99.0. Container lacks `cargo` and `rustc`; no executable RED/GREEN, Rustfmt, Clippy or full Rust test was observed.
- Latest earlier GitHub Actions runs on prior SHA terminated within seconds with failure and no reported job steps, so no green CI receipt can be claimed for these changes.
- `GovernanceJournal::begin` still only reserves an intent; it is not permission to execute a purchase. Full issuer-authenticated sanctions sourcing, trusted operator identity, durable approval and authenticated external effects remain OPEN.
- Unscoped generic names (e.g. `buy`) are still subject to ordinary allowlist but do not have authoritative per-tool semantic risk binding here: callers MUST NOT expose these as a way around canonical action names. Proper action-to-manifest binding across the authenticated OMAt ingress remains required.
- PDF-1.45 remains `OPEN`, canonical stages unchanged and `production_approved=false`.

## Reproduction and acceptance commands
```bash
cargo test --locked --no-default-features --lib governance_contract::tests
cargo test --locked --no-default-features --lib governance_journal::tests
cargo test --locked --no-default-features --lib procurement_scout::tests
cargo fmt --all -- --check
cargo clippy --locked --all-targets --no-default-features -- -D warnings
```

No passing result is asserted until these commands are actually executed with receipts on the final SHA.

## Code-review correction on 2026-11-14
An intermediate form of the canonical action match used conditional Rust match arms for invalid shapes and then a generic deny for scoped action prefixes. A *valid* named action fell through to the generic denial. This was corrected in commit `32f8acab24a4f40205cd0a8891c74f55343e1ed7` by computing `action_shape_valid` from explicit named arms and returning DENY only when false. Additional positive canonical-action and high-risk negative regression cases were committed in `6a90bbc1d8d436880f3d11aa289f64509b9be8e3`. Existing contract-verification success tests also cover this behavior. No Rust test execution is claimed.
