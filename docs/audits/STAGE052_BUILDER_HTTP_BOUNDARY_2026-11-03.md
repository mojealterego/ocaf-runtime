# OCAF 0.52 — Cloud HTTP request boundary, evidence 2026-11-03

## Scope and actual source
- Main code SHA: `812d7411bd591ac2eb226d4b16949c5309bcffce`.
- `src/builder_http.rs`: transport-neutral parsed-request adapter, **not a socket listener**. A separate authenticated TLS gateway must provide `AuthenticatedSubject`. Request headers, path, and JSON are never identity authorities.
- `src/builder_cloud.rs`: `pub(crate)` authorization used before processing tenant-specific request content, with existing scoped OAuth service. Preview is inert; no tool execution.
- `tests/builder_cloud.rs`: 3 additional HTTP boundary tests on top of 4 existing Cloud OAuth tests. Checks same-origin Bearer request, project preview/save/read, CAS replay rejection, wrong Origin, cookies, duplicate authorization, cross-tenant spoof, oversized body, invalid method/path and unknown JSON fields.
- `.github/workflows/stage052-builder-cloud.yml`: rustfmt, integration test and Clippy on changes to the HTTP boundary.

## TDD and exact CI evidence
- RED: [run #41551314517](https://github.com/mojealterego/ocaf-runtime/actions/runs/41551314517), SHA `47d71116b7294b2651aac8461773fd2d28a2cc74`: rustfmt PASS; compilation fails with `E0432 unresolved import ocaf_runtime::builder_http` for the three new tests. This is an expected missing-implementation RED.
- Intermediate: [run #41551657256](https://github.com/mojealterego/ocaf-runtime/actions/runs/41551657256), SHA `ec1a1cbfa6e2eeeb764ece2ea99879ca40bf65a7`: 6 PASS, 1 FAIL (cross-tenant request returned 422 before OAuth denial). Corrected authorization ordering.
- Intermediate: [run #41552116661](https://github.com/mojealterego/ocaf-runtime/actions/runs/41552116661), SHA `33431f9211df8b39402d2d0914da300c9ab97e7b`: 7/7 tests PASS, rustfmt PASS, Clippy FAIL (needless lifetime in test helper). Corrected without suppressing lint.
- GREEN: [run #41552347121](https://github.com/mojealterego/ocaf-runtime/actions/runs/41552347121), SHA `812d7411bd591ac2eb226d4b16949c5309bcffce`: completed/success; **rustfmt PASS, 7/7 tests PASS, Clippy -D warnings PASS**.

## Unresolved acceptance / threat boundaries
- No HTTP parser, TCP socket, TLS listener, mTLS or identity gateway. Upstream MUST enforce request framing, connection limits, TLS, authenticated subject binding and header canonicalization. The adapter alone is not safe to expose directly on a public network.
- No browser session implementation, UI↔backend E2E, gateway CSRF session management, deployment, distributed rate limit, signed artifact export, sandbox execution or Cloud production evidence.
- Origin comparison and cookie rejection protect the bearer-only adapter, not a separate cookie-authenticated gateway.
- No change to any of the 10 Agents Builder product statuses: all `NOT_IMPLEMENTED`; `production_approved=false`.
- No independent penetration test or operator approval is claimed. Full Rust CI status on latest documentation commit must be checked separately.
