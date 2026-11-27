# AB-OM-03/14, stage 0.51 — cache and source traceability

Date: 2026-11-27. Base: 8809912737e7bbd066e1c2d6d3c74fd983fc1d1d. TESTED_LOCAL_PARTIAL. No requirement or release acceptance.

The previous dossier changes did not invalidate the old PWA cache. A global CacheStorage.match also permitted lookup outside the intended release cache. Bump version v12 to v13; query only that named cache, preserve network fallback and existing allowlist. Node VM tests use deterministic service-worker and CacheStorage doubles; no real browser claim.

Base sw.js blob f3b28f6b2559df47a25e7f7d52fc4079124dec70 matched Git.
Node v22.16.0, commands from apps/agents-builder-web:
- RED node --test tests/service-worker-cache.test.mjs: 0 PASS / 3 FAIL, exit 1.
- GREEN node --check sw.js: exit 0.
- GREEN node --test tests/service-worker-cache.test.mjs tests/agent-catalog-ui.test.mjs: 8 PASS / 0 FAIL, exit 0.
Tested blobs: sw.js be1c780b601cf50d7b0bf40710a893d96321a2ac; tests/service-worker-cache.test.mjs ac4d579902ee37960843f55378ba1fc94b3447f9.
TAP SHA-256 RED 7f1bf0b2cd7a24ad2b1c6024f8fa95f286d703aaeffc7611baf2bebe0477b563; GREEN 3b093861a67fbeb76144bc541239713cc05bdde81a1c9b62055a63949f36ccb6. Logs not embedded.

Correction of previous commit: updating the evolution manifest with evidence changed its Git blob away from the immutable traceability matrix pin. Restore only the original manifest blob 313aa1471ee6a9f2a3142de67e1b21af5135940b, retain the implemented dossier, tests, ROADMAP and audit evidence. All statuses remain unchanged. Verify pinned source blobs against the resulting Git tree before publishing. This is not execution of the full Python traceability gate.

Full project regression, Rust builds, physical browser/PWA upgrade and production deployment not executed.
