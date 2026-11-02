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
