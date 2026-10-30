# Stage 0.65 — OmniFin: durable per-agent spending ledger

Evidence basis: historical v9 roadmap, canonical v11 stage 0.65. This
document records a bounded implementation component, not acceptance of
the whole stage.

## Implemented component

- Rust module src/finops_ledger.rs with SQLite WAL, foreign keys and
  IMMEDIATE transactions for cross-process reservation serialization.
- Scoped budgets keyed by tenant and agent; signed 64-bit minor units
  with checked arithmetic, explicit deny when no budget is configured.
- Durable idempotency keyed by tenant, agent and request; reservation,
  commit and release transitions; a released request cannot be reused.
- Administrative budget reductions are denied below reserved + committed.
- Negative tests for overspend, replay, invalid identifiers, tenant
  separation, persistence and competing database connections.

## Explicitly not implemented or verified

- No network or agent-facing API, no actual payments, purchases or provider
  metering; budget administration is a trusted library operation, not
  authorization. It MUST be placed behind OCAF OMAt/identity capability
  enforcement before any untrusted caller is allowed to configure budgets.
- No per-provider billing reconciliation, escrow or operator approval flow.
- No cross-host PostgreSQL failover or independent production acceptance.
- A passing local/CI test would establish only this ledger primitive.
  Canonical stage 0.65 remains PLANNED_OR_UNVERIFIED pending full gates.

## Reproduction

cargo fmt --all -- --check
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features --test finops_ledger

CI receipts must be tied to the tested commit SHA. No test success is
claimed until the actual runner reports success.
