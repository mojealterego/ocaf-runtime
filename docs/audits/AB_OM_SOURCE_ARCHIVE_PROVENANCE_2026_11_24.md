# OCAF Runtime — Source Archive and Verified Research Provenance
Date: 2026-11-24. Base remote main: fd0a34df68128a72b8bf65e6c63eef00880b35e6.

## Implemented

New src/architect_source_archive.rs persists actual bounded HTTP response bytes, source URL, retrieval time and SHA-256 in tenant-scoped SQLite WAL. It uses IMMEDIATE transactions, exact replay, conflict rejection, 4096-record per-tenant limit, length and UTF-8 checks, digest verification on each read and restart safety. Archive custody must remain with the authenticated operator.

New CLI modes:
- ocaf-architect-fetch --archive-only: performs actual pinned public IPv4 TLS-verified HTTPS GET and stores the exact returned body in SQLite with OCAF_SOURCE_ARCHIVE_DB.
- ocaf-architect-evidence --submit-archived: requires an existing OCAF_EVIDENCE_ARCHIVE_DB; checks every signed SourceRecord against retained body bytes before committing the signed OM Engine receipt.
- ocaf-architect-finalizer --once-archived: requires existing OCAF_FINALIZER_ARCHIVE_DB; revalidates the full signed Planner-Architect-Verifier evidence chain and archived source contents before sealing a period as SUCCEEDED.

The old submit and once modes are retained for compatibility. They must not be used as the production source-backed acceptance path. This increment does not activate OMAt while OSTc remains disabled.

## Actual test evidence

Environment: isolated Linux Sprite; Rust 1.99.0; pinned Cargo.lock enforced with --locked; real SQLite and actual CLI subprocesses.

- TDD RED: missing SourceArchive module and strict API (E0432 / E0599), missing CLI mode; strict finalizer rejected unsupported mode; hardlink path attack was rejected for the wrong reason before correction.
- New archive integration: 4/4 PASS.
- Archive CLI admission tests: 4/4 PASS.
- Actual signed evidence CLI E2E: 1/1 PASS.
- Actual finalizer CLI integration: 2/2 PASS.
- Full cargo test --locked --no-default-features: 788 PASS, 0 FAIL, 133 suite summaries.
- Full cargo test --locked: 829 PASS, 0 FAIL, 138 suite summaries including Zenoh.
- Clippy for relevant bins/tests: exit 0; seven earlier library warnings, no new module warnings.
- rustfmt --check; git diff --check: exit 0.

Real network archive smoke: pinned external IPv4 104.20.23.154; HTTPS host example.com; GET /; TLS verified; 577 bytes; retrieval Unix 1795487121; source SHA-256 25ddf2c883e0d1958ea971d279a7e4f0fd446724ee3db7db19dadabd4a62e484. Independently retrieved SQLite BLOB bytes and Python hashlib.sha256 agreed. Real HTTP data, not simulated model output. Temporary test directory was removed.

Actual E2E: signed research was not accepted until two retained content bodies existed under the same authenticated tenant; replay succeeded only with intact archive; BLOB tampering was denied. The stronger finalizer rejected missing or altered archived contents even if prior Ed25519 research/verifier receipts were valid. A hard-linked source journal alias of OM Engine was refused before signer processing.

## Source file SHA-256

- src/architect_source_archive.rs: 22968216a55a973b7cbf2af25719c8fc9368a93b320d450e359af95bc567a2e4
- src/architect_research_evidence.rs: da164c7c536407a024a115543555643aeb13e7b441c18e06b474b176aaf1a532
- src/architect_research_finalizer.rs: adea6ff7ccde6a99d39b948e4c52a6fe86369dfef37c14c726f80dfd1d4f2ab6
- src/bin/ocaf-architect-evidence.rs: dc3f26462fe62856d05fb52a6f14c55030cf3f00ff114ae97a49464f6b6d2a6e
- src/bin/ocaf-architect-fetch.rs: 91648a926f56211dc4968b09d089e173d90dd12d6c9f9ae316ca2910886cf72b
- src/bin/ocaf-architect-finalizer.rs: f03104e2b0cc9a94c155c485140165caae2a5f45f1ce6c94f636d9b28715d7a3
- tests/architect_source_archive.rs: fa003af152e3f3476649fd2a64663836cf8d5cd8b6ddae1adbf5c7f9fa6e587b
- tests/architect_archive_cli.rs: da5d37c46e3750eab4c4b188164bb3bbe92249986814d525165ac86be37ccb59
- tests/architect_archive_end_to_end.rs: 997659e0b1d81ebc8b7d9b6c3c4ff5da6074c59d173b80a13d2beef990adbcde
- tests/architect_finalizer_cli.rs: 961bf00c3cff2ad6d5fcdb68a2caac9643ca32a68ee1a5c628fee71d9f9cae98

## Unfinished security and system scope

Hash consistency does not prove that a web page was reached: manual trusted-API imports are possible. Only the approved pinned TLS fetch route has actual network acquisition properties. An external monotonic audit anchor is required against privileged multi-DB rollback; cross-DB operations are not atomic; filesystem races require OS isolation. Live model execution, OMAs/OMAt authorized workers, automated search, Deep Research, benchmark comparisons, catalog promotion, OSTc/PQC deployment and operator-approved releases remain unimplemented in this increment. Legacy CLIs must be denied by policy for stronger production evidence assurance.

Scoped: IMPLEMENTED + TESTED_LOCAL + SQLITE_INTEGRATED. Full AB-OM-01, AB-OM-06, AB-OM-07, AB-OM-10 remain OPEN_UNVERIFIED. GATE-09/GATE-10 NOT_ACCEPTED. production_approved=false. No deployment.
