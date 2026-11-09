# Agents Builder native CI trigger coverage — 2026-11-09

- Requirement: 0.53 / native Builder dependency-trigger integrity; this audit does not close any product acceptance.
- Source baseline: `fae7ce68307fbb5f73d6b3c6ecc166695ef7dfbb` on `main`.
- Missing exact `on.push.paths` inputs: desktop — `Cargo.toml`, `Cargo.lock`; mobile — `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`; Linux — `Cargo.toml`, `Cargo.lock`.
- New read-only verifier: `python3 scripts/ci/check_native_trigger_coverage.py --root .`. A nonzero result is expected until workflow filters are repaired.
- Unit test: `cd scripts/ci && python3 -m unittest -v test_check_native_trigger_coverage.py`. Five tests passed locally on Python 3.13, including malformed inputs, positive coverage and negative coverage. This is not Rust/CI/E2E evidence.
- CI run ID: UNVERIFIED. Runner: local Python 3.13. Artifact digest: UNVERIFIED. Reviewer: UNVERIFIED. Operator approval: NOT_GRANTED.
- Cost guard: this commit intentionally does not modify native workflow files or trigger paid native runners. The native filters remain open and require an explicitly cost-bounded repair. No historical stage number or product status changes.
