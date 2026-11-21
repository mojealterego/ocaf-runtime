# OCAF AB-OM-10 — isolated SQLite research scheduler implementation

Date: 2026-11-21. Branch: main. Parent SHA: `30afd81eb2d9a3a5f13b83f2576be3eb00dfc9a5`.

## Scope actually implemented

- Source: `src/architect_research_schedule.rs`; registered as `pub mod architect_research_schedule` from `src/lib.rs`.
- Three UTC period identities: competitive research (weekly), market research (weekly), system evolution research (calendar monthly).
- Per-tenant/per-fleet/per-kind/per-period SQLite uniqueness and transactional claims.
- Monotonic lease epochs, bounded attempts (3), bounded lease duration (3600s), worker fencing, immutable success receipts, exact replay only.
- Expired lease requires explicit verifier-role reconciliation; unknown outcomes halt rather than auto-retry.
- No unattended scheduler/worker loop, network client, release admission, egress or AI researcher is claimed.

## Local verification evidence

Environment: isolated Sprite Linux; pinned Rust toolchain 1.99.0; `Cargo.lock` with `--locked`.

| Command | Observed result |
| --- | --- |
| `cargo test --locked --no-default-features --test architect_research_schedule` | 6 passed, 0 failed |
| `cargo test --locked --no-default-features --lib` | 354 passed, 0 failed |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked --no-default-features` | 728 passed, 0 failed across 115 summaries |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked` | 769 passed, 0 failed across 120 summaries |
| `cargo clippy --locked --no-default-features --test architect_research_schedule` | exit 0; 7 pre-existing warnings in unrelated modules; no new module warnings |
| `git diff --check` | exit 0 |

Rustfmt was run on only the two newly introduced Rust files to avoid unintentional formatting of unrelated repo files. Initial full-suite attempt ran out of disk space during compilation; generated `target` was cleaned using `cargo clean`, followed by both successful full suites with incremental/debug disabled.

## Remaining acceptance blockers

The module is a coordination kernel, not a live scheduled worker. The host must supply authenticated tenant/fleet/worker and role, trusted clock, independently controlled verifier, durable runner, explicit research tools and evidence; no caller-provided boolean is proof of no side effects. Full AB-OM-10 requires timezone/catch-up policies, bounded egress and costs, worker activation and checkpoint integration with OM Engine/OMAs/OMAt. AB-OM-06–09 require additional real research and gated evolution workflows. **Status:** TESTED_LOCAL_PARTIAL, AB-OM-10 OPEN_UNVERIFIED, production release NOT_ACCEPTED. No automatic promotion of agents or system evolution.
