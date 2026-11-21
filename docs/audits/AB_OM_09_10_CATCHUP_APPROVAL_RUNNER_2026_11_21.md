# AB-OM-09 / AB-OM-10 — Research Scheduler and Signed Operator Approval

Date: 2026-11-21. Branch: main. Baseline commit SHA: a5cdf14a219de61a4acd926138811dd1e13b5eab.

## Delivered implementation

### AB-OM-10 — durable research schedule

- src/architect_research_schedule.rs: SQLite scan watermark per tenant/fleet, ISO Monday UTC weekly periods, calendar month UTC periods, duplicate avoidance via primary key and SQLite IMMEDIATE transactions.
- tick() performs bounded offline catch-up (90 days maximum), never silently advances the cursor when capacity is exceeded; first use registers the current period only.
- list_due() provides stable ordered READY periods with 1..64 cap, without changing a previously leased or completed run.
- src/bin/ocaf-architect-scheduler.rs: runnable Tokio process with an immediate scheduler tick, periodic scans and SIGTERM/Ctrl+C shutdown; --once mode permits a real subprocess test.
- Required environment: OCAF_ARCHITECT_DB absolute path, OCAF_ARCHITECT_TENANT, OCAF_ARCHITECT_FLEET, OCAF_ARCHITECT_TIMEZONE=UTC; optional OCAF_ARCHITECT_INTERVAL_SECONDS (10..3600).
- Non-UTC zones are rejected deliberately to avoid incorrect DST semantics.

### AB-OM-09 — pinned operator consent

- src/architect_evolution_approval.rs: SQLite-backed immutable staged candidate, exactly bound to tenant, fleet, proposal ID, candidate SHA-256, evaluated revision SHA-256 and a max 31-day expiration.
- Domain-separated Ed25519 approval message with length-prefixed fields. Public operator key is pinned at startup; no private signer key is loaded.
- stage(), signing_message(), approve(), consume_for_release() and snapshot() enforce exact signature, candidate/revision match, expiration, one-time consumption, concurrency and tenant scope.
- Production admission, signer key lifecycle, approved operator UI, real rollout/rollback and cryptographic protection by OSTc/PQC remain separate open requirements.

## Reproducible verification

Pinned Rust toolchain 1.99.0, Cargo.lock with --locked, isolated Linux Sprite. Test runner used CARGO_TARGET_DIR=/tmp/ocaf-work/repo/target, CARGO_INCREMENTAL=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_PROFILE_DEV_DEBUG=0 for full suites.

| Command | Observed result |
| --- | --- |
| cargo test --locked --no-default-features --test architect_research_catchup | 6 passed, 0 failed |
| cargo test --locked --no-default-features --test architect_evolution_approval | 8 passed, 0 failed |
| cargo test --locked --no-default-features --test architect_scheduler_cli | 3 passed, 0 failed |
| cargo test --locked --no-default-features | 745 passed, 0 failed (119 test summaries) |
| cargo test --locked | 786 passed, 0 failed (124 test summaries) |
| cargo clippy --locked --no-default-features --bin ocaf-architect-scheduler --test architect_research_catchup --test architect_evolution_approval --test architect_scheduler_cli | exit 0; seven preexisting warnings in unrelated sources; no new module warnings after correction |

RED: /tmp/ocaf-catchup-red.log (E0599), /tmp/ocaf-approval-red.log (E0432), /tmp/ocaf-sched-cli-red.log (missing CARGO_BIN_EXE).
GREEN: /tmp/ocaf-catchup-green2.log, /tmp/ocaf-approval-green3.log, /tmp/ocaf-sched-cli-green.log and both full-suite logs in isolated Sprite. Full raw logs were not published to GitHub; their test counts are cited here.

## Explicit scope and release decision

Not implemented: live autonomous Deep Research worker, OM Engine scheduler dispatch, model invocation, OMAs/OMAt tools, egress/compute budget enforcement, historical catch-up beyond 90 days, IANA/DST timezone scheduling, GUI operator approval, signed release admission, external SQLite rollback anchor, OSTc/PQC protected releases, rollout, health gate or rollback.

The scheduler only creates durable due work and reports it. The approval gate only produces a verified single-use authorization, not permission for the agent to self-deploy.

AB-OM-09 and AB-OM-10 full acceptance statuses remain OPEN_UNVERIFIED. Scoped implementation is TESTED_LOCAL_PARTIAL. GATE-09 / GATE-10 NOT_ACCEPTED. production_approved=false. No production deployment performed.

## Live local process and integrity evidence

Resident Tokio scheduler executable launched with an isolated temporary SQLite database and actual Linux OS clock. Its first tick wrote exactly 3 due runs and 1 durable scan watermark; SIGTERM shutdown exited with code 0. This proves the ticker process and SQLite writes only, not research execution.

Local SHA-256 of implementation files (at verified source checkout):
- src/architect_evolution_approval.rs: 5ef3bd31887897d1cd0b3ab108d180e4284615ccfd9bde470cbffbd0ccc55f21
- src/architect_research_schedule.rs: 553122d4cf0babfa4aad46a76dbf65123e75f6d57c761978fe69666631c53fb6
- src/bin/ocaf-architect-scheduler.rs: 700cef58fd74b3ece95ed14cc74ebf571376d05b926ccf8c89269798d66d6672

Final Clippy check: exit 0 with seven legacy warnings from unrelated source files; new modules produced no warnings. Final complete Rust regression suites repeated after the last source change: 745 PASS and 786 PASS, respectively; 0 FAIL. JSON manifest parsed successfully and git diff --check exited 0.
