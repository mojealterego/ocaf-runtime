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

## PurchaseIntent and legacy buy alias hardening — 2026-11-14

`procurement_scout::prepare_purchase_intent` emits the canonical proposed-purchase payload digest, binding dossier, policy and trusted watchlist snapshot; no executor or purchase side effect exists. `PurchaseIntent::to_action_request` now explicitly requires `uses_external_provider=true` and an independently supplied `ProviderAccess`; without it the existing Governance preflight must DENY. A SQLite journal regression requires ProviderNotAuthorized and MissingApproval to be recorded separately, without a pending purchase intent.

`governance_contract` also treats the historical generic `buy` action as procurement-medium-risk, rather than allowing it to downgrade to low risk or spoof another domain. Non-procurement external-provider test fixtures were moved to the read-only `read` action to preserve their original purpose. Source regression tests and 17 procurement module unit tests are present; Rust compilation and CI PASS are **not confirmed**. Full authenticated OMAt control plane, live provider cost authorization, external sanctions issuer, operator attestation and actual purchase E2E remain OPEN. Canonical stages and `production_approved=false` unchanged.

## Final source hardening: provider gate after signed approval — 2026-11-14

Review found that independently signed human approval alone did not force an external-provider authorization check if an agent manually constructed an `ActionRequest` with `uses_external_provider=false`. The helper `PurchaseIntent::to_action_request` already sets this flag; `governance_contract::AgentContract::evaluate` now *also* enforces it for the canonical `procurement.commit_purchase_order` **and** legacy `buy` actions, regardless of caller context. Because high-risk procurement rejection precedes the new check, High/Critical still produce `RiskCapExceeded`. With provider flag set, a missing or non-authenticated `ProviderAccess` yields `ProviderNotAuthorized`; with a valid provider but no signed approval, `MissingApproval` remains mandatory.

Source test `canonical_purchase_cannot_omit_provider_gate_after_human_approval` (RED source commit `941672c0`) covers omission after a valid signed approval. Implementation commit `64c394aa`, plus `legacy_buy_requires_provider_check_even_with_signed_approval` (test commit `c3dd4586`), cover invalid/valid provider cases. Existing SQLite journal procurement-denial fixture now supplies a test-only trusted provider for the MissingApproval assertion (`b92e09f5`). These are **unexecuted Rust tests**; source review cannot establish compilation or actual end-to-end purchase authorization. No external side effects were enabled or executed.

The deterministic reference JSON at `docs/evidence/ARGON_PROCUREMENT_INTENT_SHA256_REFERENCE_2026-11-14.json` was independently checked by Python hashlib, with 644 synthetic hash-model assertions PASS. It does not authenticate a real sanctions feed, establish rustc compilation or close the canonical stage. PDF-1.45 remains `OPEN`, production_approved remains false.

## Integrity of test evidence (2026-11-14)

Recomputed Argon procurement reference vectors against the documented canonical byte encoding. Corrected one previously erroneous watchlist mutation digest and added a standalone stdlib Python test and a Rust mutation-vector assertion. The Python reference suite passed 6 methods with 644 deterministic mutation comparisons after correcting the vector; the original fixture produced a reproducible 1-test failure. **This does not prove Rust compilation, CI or production approval.** The canonical `PDF-1.45` stays `OPEN` and `production_approved=false`.
