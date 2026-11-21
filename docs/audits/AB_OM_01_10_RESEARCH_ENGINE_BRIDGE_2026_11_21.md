# AB-OM-01 / AB-OM-10: research workflow bridge and real OM Engine registration

Date: 2026-11-21. Branch: main. Baseline SHA: fe885f0f873c6636cb550ced08150feeca89acd5.

## Scope of implemented code

- src/architect_workflow_bridge.rs: register_due queries only READY, due jobs for a trusted tenant and fleet. It derives immutable workflow IDs using SHA-256 of fleet identity plus kind/period, avoiding cross-fleet collisions.
- Each research period becomes a real OM Engine WorkflowContract with an immutable source commit SHA, a bounded total cost (3..1,000,000 minor currency units), three typed steps (Planner: plan; Architect: research; Verifier: verify), DAG prerequisites and bounded 300-second step leases.
- Workflow deadline is deterministic: research period end + 120 days, including delayed periods. If already expired at registration, reject instead of silently renewing execution permission.
- OmEngine::register persists each contract in its actual SQLite journal with replayable SHA-256 contract binding. Re-registering the same verified source+budget is idempotent; changed base commit or policy fails with an immutable contract conflict.
- Registration occurs before any lease claim; no research job is marked complete. The two SQLite stores are not distributed ACID/2PC and can temporarily contain an OM contract without a corresponding scheduler lease. Idempotent retry repairs partial registration.
- src/bin/ocaf-architect-scheduler.rs supports optional explicit OM Engine configuration: OCAF_ARCHITECT_OM_DB, OCAF_ARCHITECT_BASE_COMMIT_SHA, OCAF_ARCHITECT_MAX_COST_MINOR. All three must be present, valid and trusted; incomplete/invalid policy fails before any database is opened.
- When configured, a real --once or periodic scheduler tick creates due periods and registers corresponding OM Engine workflows; without OM config, the existing queue-only behavior is unchanged.

## Observed verification

Rust 1.99.0, Cargo.lock --locked, actual SQLite and OM Engine replay; no synthetic substitute for OM Engine.
- RED test missing module: /tmp/ocaf-bridge-red.log (E0432).
- GREEN test: cargo test --locked --no-default-features --test architect_workflow_bridge: 5 passed / 0 failed.
- RED CLI: /tmp/ocaf-bridge-cli-red.log: 3 failed, because the runnable scheduler ignored OM configuration.
- GREEN CLI: cargo test --locked --no-default-features --test architect_scheduler_om_cli: 3 passed / 0 failed; actual subprocess --once, persisted three real OM contracts, unchanged registration after restart, fail-closed SHA/policy conflicts.
- Full cargo test --locked --no-default-features: 753 passed / 0 failed (121 test summaries).
- Full cargo test --locked: 794 passed / 0 failed (126 test summaries, default Zenoh enabled).
- Clippy on changed tests/binary: exit 0, seven pre-existing warnings in unrelated source files; new bridge/binary without warnings.
- rustfmt --check on changed Rust files: exit 0. git diff --check: exit 0.

## Security and architecture assessment

- Only authenticated host-supplied tenant/fleet/worker policies are trusted, not request bodies or agent-generated text.
- The base source commit SHA and total cost must be pinned externally. No untrusted source may decide its own release credentials or revise OM Engine contracts.
- The bridge does not call a model, OMAs, OMAt, tool gateway, Zenoh execution or external network. Test claims use OM Engine's actual role/fencing/receipt API only.
- Nothing marks any study completed, registers an ACTIVE agent, emits a certificate or authorizes a deployment.
- Outstanding for AB-OM-01 / AB-OM-10: authenticated execution dispatcher and effect sink, durable coordination between queue and workflow leases, source evidence with true research/benchmarks, independent verifier, cost/egress admission, IANA timezones, deployment integration and recovery/rollback across independent stores.
- AB-OM-01 and AB-OM-10 remain OPEN_UNVERIFIED for full acceptance; scoped integration TESTED_LOCAL_PARTIAL. Production approval remains false.

