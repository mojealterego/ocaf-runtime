# OCAF-AGENT-001 — evidence run 2026-10-21

## Commands

```text
cargo test --manifest-path "OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/Cargo.toml"
cargo run --manifest-path "OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/Cargo.toml" -- \
  "OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/config/agent.toml"
make agent001-package
```

## Results

- Agent crate: 2 integration tests + 4 security tests passed; doc-tests passed.
- Runtime output: `state=Staged`, `active_admission=Err(OstcDisabled)`.
- Package profile: Cargo `dev`; installed binary digest recorded in
  `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/packaging/BUILD_EVIDENCE.md`.
- `ACTIVE` cannot be enabled by readiness claims because admission checks the
  manifest and rejects disabled OSTc before evaluating caller evidence.
- Root workspace verification: `cargo test --workspace --no-default-features
  --lib --tests` passed 559 tests with 0 failures using one test thread. The
  separate doc-test pass was not completed on this constrained host after the
  process was killed by the host (OOM, exit 137).

This file records a local run. It is not an external certification or production
approval.
