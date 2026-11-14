# Argon PDF-1.43/1.44/1.46 — Procurement Scout read-only preflight

Date: 2026-11-14. Repository: `mojealterego/ocaf-runtime`, branch `main`.

## Scope implemented in source

- `src/procurement_scout.rs`: bounded tenant-scoped SupplierDossier and screening against an operator-pinned watchlist snapshot.
- SHA-256 digest is domain separated, includes issuer/tenant/time, and is stable under record reordering. Duplicate listing IDs, empty/oversized feeds, invalid names and mismatched digests fail closed.
- Snapshot freshness/expiry is checked against a trusted time. Supplier and beneficial owner names use **exact ASCII-only matching**. Unsupported names are rejected for review, not marked clear.
- Trusted policy owns the monetary risk cap (USD cents as `u64`); the supplier cannot increase it. Potential watchlist matches or over-cap commitments produce `NotQualified`; missing/expired tax/insurance evidence and low financial score produce `ReviewNeeded`.
- `PassCandidate` means only that these bounded checks found no issue. `human_attestation_required` is always true and `purchase_authorized` is always false.
- Eleven Rust unit-test cases are present in this module; the Argon workflow has an explicit test step.

## Invariants and boundaries

All `trusted_tenant`, `trusted_now`, `ProcurementPolicy` and `independent_digest_pin` inputs **must come from authenticated ingress / operator-controlled storage**. Passing a digest inside an untrusted user request does not establish authenticity. This module never contacts providers or mutates external state. It does not represent a real sanctions-list check, sanctions clearance, legal determination, regulatory attestation, supplier approval or permission to commit money. Exact name matching can miss aliases, transliteration, diacritics and beneficial-ownership relationships.

## Evidence and acceptance

- Added source + tests and registered module in `src/lib.rs`.
- Added a dedicated cargo-test step to `.github/workflows/argon-pdf-115-154.yml`.
- Rust compiler and cargo test **not executed locally**: no Rust toolchain in this environment. No successful CI on final commit SHA claimed.
- No real sanctions provider, independently authenticated signed feed, human-approval source, durable OMAt purchase journal or end-to-end procurement execution tested.
- PDF-1.43, PDF-1.44, PDF-1.46: `IMPLEMENTED_CI_PENDING` (local-source partial capability; NOT complete). PDF-1.45 remains `OPEN`.
- Canonical roadmap numbering preserved; `production_approved=false`.
