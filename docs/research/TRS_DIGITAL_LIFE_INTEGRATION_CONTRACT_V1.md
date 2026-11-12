# TRS → Digital-Life / NOEVARIS Ω: integration contract v1

Status: **PREPARATORY / NOT INTEGRATED**. Destination: `mojealterego/Digital-Life`, branch `main`. This document does not authorize changes to that repository. Final ZIP must only be produced after acceptance gates, not at intermediate milestones.

## Architectural decision

Digital-Life declares **one IntegratedEngine**, with mathematical reasoning, DGM/RSI and multimedia modules in a single orchestration. TRS is a subordinate, capability-restricted library/service adapter, **not** a second engine, agent supervisor or self-updating subsystem.

## API v1

Source: `src/trs_digital_life.rs`. JSON fields: `version` (currently 1), `tenant`, `sender`, `operation`, `payload` (byte array). Strict version checking, nonempty bounded identities, deny unknown fields, payload <= 65536 bytes, explicit allowlist:

- `trs.geometry.fisher_rao`
- `trs.geometry.hodge_laplacian`
- `trs.math.sat_evaluate`
- `trs.health`

These are **names only**. No dispatcher or typed payload schema is implemented. A successful `validate()` is **not** authentication, authorization, replay protection, or permission to execute.

## Required Digital-Life integration

1. Identify the existing IntegratedEngine module/plugin registry, dependency injection, trust boundary and checkpoint schema in Digital-Life before changing it.
2. Implement typed request/response schemas per operation; validate dimensions, finite numbers, payload size, algorithm version and resource budgets.
3. Bind authenticated transport principal to tenant/sender and capability allowlist; reject mismatches before deserialization/dispatch side effects.
4. For remote calls, use authenticated encrypted transport, TRS-SEAL replay/fencing where appropriate, and a durable authoritative fencing epoch.
5. Keep DGM/RSI proposer **read-only** to TRS trust policies, keys, proofs and admission rules. Explicit human approval for promotions.
6. Test with the actual IntegratedEngine: contract compatibility, authorization denial, tenant isolation, malformed payloads, restart/replay, failure injection, and end-to-end output equivalence.
7. Provide SBOM, pinned dependency hashes, build provenance, offline deployment instructions, rollback and measurable resource ceilings.

## Release gates

- `cargo fmt --all -- --check`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Actual Digital-Life adapter integration tests on pinned source revisions
- Security review of cryptographic protocol and runtime permissions
- Final source ZIP with SHA-256 manifest and verified reproducible build

**Unverified:** Rust compilation, all integration tests, security certification, Digital-Life runtime compatibility. No final ZIP released.

## Implementation update — 2026-11-12

The local mathematical dispatcher now exists in `src/trs_digital_life.rs`, supporting bounded Fisher–Rao distance, graph Hodge 0-Laplacian, exact finite SAT evaluation, and health responses. `dispatch_authorized` checks claimed tenant/sender against a caller-supplied `AuthenticatedPrincipal` and an explicit capability list. **The caller must derive the principal from a genuinely authenticated channel**; passing user-controlled principal fields defeats this boundary. SAT evaluation is capped at 12 variables to bound computation and output. Tests are recorded in `tests/trs_digital_life_dispatch.rs` and `tests/trs_digital_life_authorization.rs`, but compilation and execution remain unverified.

Remaining mandatory work: authenticated transport adapter; replay-resistant invocation envelope; stable typed schemas and resource ceilings; actual Digital-Life IntegratedEngine registration; E2E integration and Rust verification; security review. No release ZIP before these gates.

## Durable bridge admission — 2026-11-12

`src/trs_digital_life_durable.rs` now accepts a TRS-SEAL HMAC-authenticated envelope containing the entire versioned JSON request, validates tenant/sender binding and operation capabilities, and calls `verify_and_record_authorized` against SQLite pinned fencing epochs before local dispatch. Tests in `tests/trs_digital_life_durable.rs` specify replay rejection across reopened SQLite connections and rejection of identity mismatches without claiming the replay slot.

**Security limitations:** HMAC does not provide confidentiality; the external transport must encrypt and authenticate. The caller must independently authenticate the principal and provision trusted fencing epochs. SQLite provides local durability, not multi-node consensus. A crash after replay admission but before computation can lose the response; this is at-most-once admission, not exactly-once delivery. A durable result/outbox transaction is needed before effectful operations. The bridge remains restricted to bounded, local, side-effect-free math operations. Compilation, test execution and Digital-Life E2E remain unverified. No final ZIP.
