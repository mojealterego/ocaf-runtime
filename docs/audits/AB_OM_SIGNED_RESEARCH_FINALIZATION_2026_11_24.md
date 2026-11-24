# OCAF — AB-OM-01 / AB-OM-10: signed research finalization

Date: 2026-11-24. Branch: main. Base commit SHA: e1f885790dbd67a099e5ad64254d829d651c095c.

## Implemented

- New Rust module src/architect_research_finalizer.rs validates all three signed evidence records (Planner, Architect, Verifier) against persisted OM Engine receipts. The roles, passed outcomes, epoch, attempt, cost, evidence digest and verifier's exact review-of digest must agree.
- The scheduler's new crate-private seal_verified method atomically changes a READY period to SUCCEEDED in a real SQLite IMMEDIATE transaction. Same-digest replay is safe. Conflicting digest, a leased period and HALTED state cannot be silently overridden.
- finalize_ready scans up to 64 due READY periods. Incomplete/unregistered jobs remain pending, while integrity and non-idempotent conflicts fail closed.
- New executable src/bin/ocaf-architect-finalizer.rs supports only --once and requires three pre-existing SQLite journals, authenticated tenant and fleet, plus three distinct pinned role-specific Ed25519 public keys. It cannot execute research, call OMAt/OMAs or deploy a release.
- Journal aliases are rejected by canonical-path comparison and, on Unix, device/inode comparison. Leaf symlinks and unknown arguments are refused.

## Actual verification results

Environment: isolated Linux Sprite, Rust 1.99.0, pinned Cargo.lock, real SQLite and OM Engine, test debug symbols disabled to bound disk usage.

| Verification | Result |
| --- | --- |
| RED test of missing finalizer module | Failed with E0432 as expected |
| Focused architect_research_finalizer integration | 3 passed / 0 failed |
| Focused architect_finalizer_cli with compiled subprocess | 2 passed / 0 failed |
| RED hard-link alias regression | Failed due to unexpected SQLite disk I/O error |
| GREEN hard-link alias regression | Explicit, deterministic rejection PASS |
| Full cargo test --locked --no-default-features | 770 passed / 0 failed (127 suite summaries) |
| Full cargo test --locked | 811 passed / 0 failed (132 suite summaries; default Zenoh) |
| cargo clippy on finalizer binary and two tests | Exit 0; seven warnings from pre-existing unrelated modules, zero new warnings |
| rustfmt --check on changed sources; git diff --check | Exit 0 |

Security scenarios covered: replay and restart; unsigned OM Engine Passed is rejected; verifier binds exact research SHA-256; tenant/fleet mismatch and live lease denied; wrong public keys and tampered evidence JSON rejected; alias, symlink and duplicate journals rejected; incomplete workflow remains pending.

All example.org source URLs in tests are *fixtures* and do NOT represent real fetched research or evidence that external claims are true.

## SHA-256 for verified source files

- src/architect_research_schedule.rs: 0f7e6f780ad4ec479bf38197f8888cbb66d3ed257933fc07bdba931ec054edc6
- src/architect_workflow_bridge.rs: 0366e751795837433b476f804763c5323a0a65309dcf45e311019b79b82f4c21
- src/architect_research_finalizer.rs: 9abd28da3f9f94f4ff034060f9edfa672050a6b4aedf7345ecbfedb345b2147b
- src/bin/ocaf-architect-finalizer.rs: 4603d518910edf4914f1102e325d01ae9539f9c62d6bd5d1c224c98454504261
- tests/architect_research_finalizer.rs: 5da6f99207cc8b1a85818895408a2e64fc79f7ef7345ecbfedb345b2147b
- tests/architect_finalizer_cli.rs: c346b766ae7dada57a4878c2638e0d4f97ea74a34986efc8d370d3db82f96772

## Outstanding production requirements

The trust boundary is still the authenticated service manager: tenant/fleet values and signer public keys must not be controlled by an agent. SQLite journals must be physically protected and externally anchored against malicious coordinated rollback. Parent-path swaps and SQLite TOCTOU require OS controls. Signed evidence proves a signature over metadata, not that any cited webpage was fetched or benchmarked independently.

Live Deep Research workers (Rust/Zenoh), OMAt/OMAs execution, source-content verification, cost/egress gates, distributed HA, automated research, operator-approved update rollouts, OSTc/PQC and end-to-end releases across all Agents Builders remain open.

Scoped state: IMPLEMENTED + TESTED_LOCAL + SQLITE_INTEGRATED. Full AB-OM-01 and AB-OM-10 are OPEN_UNVERIFIED. Full GATE-09/GATE-10 are NOT_ACCEPTED. No production deployment. production_approved=false.

CLI requires OCAF_FINALIZER_TENANT, OCAF_FINALIZER_FLEET, OCAF_FINALIZER_SCHEDULE_DB, OCAF_FINALIZER_OM_DB, OCAF_FINALIZER_EVIDENCE_DB and pinned OCAF_FINALIZER_PLANNER_PUBKEY_ED25519, OCAF_FINALIZER_ARCHITECT_PUBKEY_ED25519, OCAF_FINALIZER_VERIFIER_PUBKEY_ED25519. Optional OCAF_FINALIZER_LIMIT is 1..64.
