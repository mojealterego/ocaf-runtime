# Stage 0.52 — OAuth-bound Builder Cloud application service (2026-11-02)

Status: **IMPLEMENTED_UNVERIFIED** until dedicated GitHub Actions CI confirms build and tests on the commit SHA. This is a **partial application-service increment**, not a Cloud deployment or completion of AB-CLOUD-CODE / AB-CLOUD-NOCODE.

## Source and acceptance
- \`src/builder_cloud.rs\` binds each operation to a token validated by existing \`OAuthService.validate_access\` against the ingress-authenticated subject, exact OAuth audience and \`builder:read\` / \`builder:write\` scopes.
- \`SqliteBuilderStore\` provides tenant-scoped revision history, optimistic concurrency and hash chain. Read supports externally pinned heads.
- \`preview\` compiles a strictly bounded inert DAG; it cannot execute tools, make payments or publish.
- \`tests/builder_cloud.rs\` exercises authorized preview/write/read, stale-head CAS, read/write scope separation, cross-tenant spoofing, wrong audience and token revocation.
- Dedicated CI: \`.github/workflows/stage052-builder-cloud.yml\`. Capture workflow run URL and exact green SHA after execution, not before.

## Explicitly not delivered
- No HTTP listener/Axum routing, trusted ingress identity enforcement or browser-to-backend transport.
- No Cloud UI deployment, authenticated browser session, CSRF protection, rate limiting or service telemetry.
- No sandbox execution, remote build, artifact signing, deployment approvals or ten platform-specific deliverables.
- No independent production security audit. \`production_approved=false\`.

The operator must provision the OAuth database, protected Builder SQLite database, registered OAuth clients and verified ingress identity. Do not derive the subject from JSON payload, URL path, arbitrary request headers or agent-generated content.

## CI regression and remediation

- Commit `bbbd530c73ac55c638893f27cdb95282debe33b6`, run [#41536595192](https://github.com/mojealterego/ocaf-runtime/actions/runs/41536595192): Rust formatting PASS, 4/4 integration tests FAIL at OAuth fixture initialization (`Configuration`), before any service assertions. Root cause under investigation; fixture now explicitly provisions private 0700 parent directory as required by `private_database`. No test is claimed green until a later run confirms it.


### 2026-11-02 — dowód CI etapu 0.52 (ograniczony zakres)

- **Przypięty SHA:** `0be306742f7444b4c9719fba282ba4b14682430d`; **GitHub Actions:** [OCAF 0.52 Builder Cloud OAuth admission](https://github.com/mojealterego/ocaf-runtime/actions/runs/41536940370) — `completed/success`.
- **Potwierdzone:** Rust formatting PASS; 4/4 testy integracyjne OAuth / tenant / CAS / scopes / revocation PASS; Clippy `--locked --no-default-features --test builder_cloud -- -D warnings` PASS.
- **Zakres:** jedynie `src/builder_cloud.rs`, `tests/builder_cloud.rs`, istniejące OAuth/SQLite/typed IR. Status komponentu `TESTED_LOCAL_PARTIAL`; brak HTTP ingress, wdrożenia Cloud, sandbox execution i platform E2E. Żaden z 10 produktów nie został odebrany; `production_approved=false`.
