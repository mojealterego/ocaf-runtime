# OCAF — signed research evidence gate (AB-OM-01, research integration foundation)

Date: 2026-11-21. Branch: main. Baseline: 19cbe2ca4828043604f500fd51aa3bb2db4848fb.

## Delivered implementation

- src/architect_research_evidence.rs: validates strictly typed, bounded research records and domain-separated Ed25519 signatures. Separate pinned public keys for Planner, Architect and Verifier; matching signing authorities cannot be reused for more than one role.
- Each submission binds exact tenant, workflow, step, epoch, attempt_id, outcome, observed time, cost and evidence JSON. Research reports require at least two distinct recent HTTPS source metadata references (content SHA-256). These are citation declarations only, NOT evidence that a web page was fetched, truthful or benchmarked.
- A Verifier record must reference the exact SHA-256 of the successfully completed research step in the genuine OM Engine journal. Failed, unclaimed, expired or cross-tenant steps cannot be marked passed just by submitting arbitrary JSON.
- Immutable SQLite evidence store: SHA-256 of raw canonical serialized JSON, independent Ed25519 signature verification on every read, conflict prevention and bounded record size (32 KiB). Rejects tampered JSON even when altered whitespace is accompanied by a recomputed raw digest.
- OM Engine: read-only last_receipt accessor replays the tamper-checked journal. Exact replay of a signed failed receipt is idempotent; changed evidence/epoch is rejected. No signing keys or model credentials are stored in the evidence gate.
- src/bin/ocaf-architect-evidence.rs: working local --submit CLI. Reads at most 40 KiB of a strict JSON envelope containing evidence plus lowercase hex Ed25519 signature; verifies identity, role-specific trusted public keys, existing OM Engine lease, signature, integrity and cost constraints. Successfully recorded or replayed receipts print the evidence digest. No network access, no arbitrary commands, no self-approval.
- Dependency: URL parser 2.5.8 pinned directly; already present transitively in the reviewed Cargo.lock. Outcome gains Serde support for signed typed evidence.

## Trust boundaries

Required service-manager settings for CLI: OCAF_EVIDENCE_DB, OCAF_EVIDENCE_OM_DB, OCAF_EVIDENCE_TENANT, OCAF_EVIDENCE_PLANNER_PUBKEY_ED25519, OCAF_EVIDENCE_ARCHITECT_PUBKEY_ED25519 and OCAF_EVIDENCE_VERIFIER_PUBKEY_ED25519. The keys are 32-byte Ed25519 public keys encoded as 64 lowercase hex characters. Paths must be absolute and distinct. The privileged service manager MUST authenticate tenant and maintain filesystem/SQLite ownership. The command does not implement mTLS, remote authentication or privilege separation by itself.

The submitted signature is evidence of possession of the configured private key, not independent source verification or proof of legitimate billing. Actual external source retrieval, network SSRF/DNS rebinding control, independent benchmarking, source-content audit, signed spending receipts and isolated OMAs/OMAt execution are still missing. The OMAt executor is currently fail-closed pending verified OSTc and a real backend; this work does not bypass that policy. SQLite administrator rollback requires an independently anchored high-water mark before production. A two-database evidence/OM Engine transaction cannot be assumed atomic: the signed evidence row is committed before the OM completion; an incomplete completion leaves a safely replayable or inspectable evidence record.

The CLI does not provision identity keys, run research, claim workflow leases, promote agents, approve evolutionary updates, deploy binaries or alter release policy. Source URLs in tests use example domains as inert fixtures and are never fetched.

## Deterministic verification

Environment: Rust 1.99.0, pinned Cargo.lock (--locked), isolated Linux Sprite, real SQLite, real OM Engine and real CLI subprocesses. Environment for whole suites: CARGO_TARGET_DIR=/tmp/ocaf-work/repo/target, CARGO_INCREMENTAL=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_PROFILE_DEV_DEBUG=0.

- RED: missing module import E0432 at /tmp/ocaf-evidence-red.log.
- RED/green regression: failed signed receipt could not replay after a failed step (LeaseNotActive); corrected by read-only OM last_receipt replay.
- RED/green regression: semantically identical but noncanonical stored JSON accepted after digest rewrite; fixed by exact byte equality against canonical serialization.
- Focused library integration: 8 PASS / 0 FAIL.
- Focused real CLI subprocess: 4 PASS / 0 FAIL, including three-role Planner -> Architect -> independent Verifier signed evidence.
- Entire Rust suite without default features: 765 PASS / 0 FAIL across 124 summaries.
- Entire Rust suite with default Zenoh: 806 PASS / 0 FAIL across 129 summaries.
- Clippy result: exit 0; seven pre-existing unrelated warnings, new gate and CLI warnings corrected.
- GitHub CI: job failures observed with zero recorded steps; workflow job log download returned 404 (BlobNotFound). Root cause unverified, cannot claim CI PASS. Local test results are not production E2E.

## Security audit / release gate

- Role- and tenant-binding PASS in scoped local tests. Signature tamper/forgery and stale lease tests PASS.
- OMAt real network execution BLOCKED (verified OSTc / backend unavailable).
- Real Deep Research, independently validated citations and benchmark metrics BLOCKED.
- Multi-host HA, external anchoring and deployment health/rollback NOT VERIFIED.
- AB-OM-01 remains OPEN_UNVERIFIED for full scope. AB-OM-06, AB-OM-07, AB-OM-08 remain OPEN_UNVERIFIED.
- GATE-09 and GATE-10 NOT_ACCEPTED. production_approved=false. No deployed service or production activation is claimed.

## Source SHA-256 before publication

src/architect_research_evidence.rs: 9331dd3f18e8d84343567c58bc088676b5abe63e9709d349932062dcc8ecb43b
src/bin/ocaf-architect-evidence.rs: f1df560eff2d806498257f15c02f1fe143e09f3fc7c312a79a781cb6b3d88a3f
tests/architect_research_evidence.rs: 43cbd15b97ea9e9729bd8a3c9afb912b1aa44196ac77af5deaf225cd2d631426
tests/architect_evidence_cli.rs: b3e147d18ee261e0cf4032f8dcdc69b8fa93eb3b22bf4085275f2a13c1a89070
