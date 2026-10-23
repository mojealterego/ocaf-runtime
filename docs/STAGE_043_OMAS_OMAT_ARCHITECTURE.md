# OCAF 0.43 — OMAs + OMAt (native agent/tool servers)

Status: **ACTIVE_TECHNICAL_OPEN**. This is the target architecture and acceptance contract, **not** evidence of production completion. Work on stages 0.00–0.42 is owned separately by Codex and is out of scope here.

## Architecture decision

Canonical names are taken from `02_SPECYFIKACJA_BUDOWY_AGENTOW_OCAF_RUNTIME_OMAs_OSTc_OMAt_2026-10-19.md` (19 Oct 2026). OSTc is a separate, not-yet-validated security integration; do not equate it with OMAt or bypass conventional cryptographic controls.

The historical stage 0.43 title **MCP** is superseded **for the native OCAF control plane** by two distinct services:

- **OMAs — OmniAgent Server (agent server)**: agent identities, registration, tenant-scoped discovery, signed agent manifests, lifecycle, authorization, agent-to-agent delegation and auditable decisions.
- **OMAt — OmniAgent Tools (tool server)**: tenant-scoped tool registry, signed/versioned tool manifests, JSON input/output schema verification, capability/risk authorization, bounded invocation, sandbox executor binding and verifiable receipts.

The primary internal transport is native OCAF/Zenoh. There is **no native MCP protocol dependency or presumed MCP wire compatibility**. An optional external protocol adapter would require a separate, explicitly authorized decision and its own security boundary; it is not part of stage 0.43 acceptance. Never silently trust imported tool metadata or schemas. Preserve the original MCP requirements in the historical ROADMAP record and map their outcomes to OMAt; do not renumber or delete stage 0.43.

## Current code evidence (2026-10-23; inspected on main)

- `src/omas_server.rs` implements bounded STAGED agent registration, replay-ID rejection and fail-closed ACTIVE admission while OSTc is disabled. `src/bin/ocaf-omas-server.rs` exposes health and stdin JSON-line admission, **not a networked service**.
- `src/omat_server.rs` implements manifest/schema/policy validation, tenant checks, resource bounds and admission receipts. `execute()` explicitly returns `ExecutorUnavailable`; it does **not** execute tools.
- `src/bin/ocaf-omat-server.rs` currently prints a static health JSON; it is **not** a functioning network listener or executor.
- Existing `Cargo.toml` declares both binaries. These initial observations preceded the stage 0.43 implementation changes described below; they are not claims of currently missing local manifest persistence.

## Acceptance gates (all mandatory)

1. **Native contracts:** versioned OMAs/OMAt protocol schemas; canonical encoding, size limits, strict validation, negative malformed-input tests; no hidden MCP requirement.
2. **Cryptographic identity:** tenant-bound server/agent/tool identity, signature verification over the entire authorization envelope (operation, target, digest, nonce, expiry, resource bounds); reject replay and tampering before state mutation.
3. **Registry integrity:** persistent tenant-scoped agent and tool identities, immutable versioned manifest digests, schema compatibility and cache invalidation; explicit disable/revoke and no cross-tenant discovery.
4. **Authorization:** deny-by-default policy, separate agent and tool capabilities, risk classification, external-side-effect confirmation and audited decisions. No OSTc bypass or automatic ACTIVE admission.
5. **Execution:** OMAt invokes only explicitly configured sandbox executors with time/output/memory budgets; verified execution receipts distinguish ADMITTED from EXECUTED; idempotency/fencing for side effects.
6. **Network E2E:** authenticated OMAs↔OMAt transport over real sockets/Zenoh, distinct processes and two tenants, failure/reconnect, restart/replay and privilege-isolation tests; capture run IDs and artifacts.
7. **Adversarial tests:** poisoned tool descriptions, schema rug-pull, signature/nonce replay, namespace confusion, cross-tenant attempts, oversized payloads, unavailable executor and revoked capability all fail closed.
8. **Operations:** independent health/readiness reflecting actual capabilities, structured audit logs without secrets, documented upgrade/rollback, resource ceilings and operator GO/NO-GO.

## Immediate implementation sequence

A. Contract and threat-model tests; B. cryptographic manifest binding and persistent tenant-scoped registries; C. authenticated network endpoints; D. real bounded executor integration; E. adversarial and multi-process E2E; F. CI evidence and operator review.


## Implemented and regression-tested stage 0.43 increments (2026-10-23)

- Native OMAs agent registration identities are tenant-scoped; manifest digests require full SHA-256 syntax; replay identifiers are bounded and rejected capacity submissions do not consume them.
- OMAt registry is tenant+tool-scoped; discovery respects tenant capability, side-effect and resource ceilings. The internal executor still fails closed.
- Signed OMAt manifest admission verifies Ed25519 signatures over the complete canonical manifest, signer identity and bounded validity window against a server-supplied trusted key and tenant.
- An operator-protected SQLite store persists verified OMAt manifest bytes, digest, tenant identity and revocation tombstones over restarts. The digest alone does not protect against a privileged database attacker; an external trust anchor remains required.
- An opt-in signed OMAs registration path binds full agent registration requests to an Ed25519-signed RPC sender, tenant/agent target and durable SQLite replay nonce; the stdin JSONL staged-registration diagnostic is still a local-only legacy entrypoint.
- Dedicated CI tests cover registration scope, forged OSTc readiness, malicious/mutated manifests, replay, SQLite integrity, revocation, symlink denial and the actual OMAs/OMAt binaries.
- Evidence reference: `.github/workflows/stage043-omas-omat.yml` and per-commit run IDs. Do not treat a passing unit-test subset as full production E2E.

## Mandatory unresolved acceptance blockers

- Actual authenticated Zenoh-based OMAs↔OMAt network integration between separately running processes on distinct hosts.
- Runtime identity and persistent agent registry reconciliation, keys and tenant policy sourced from independently authenticated operator-owned configuration.
- Active tool execution via a bounded, authenticated sandbox with expiry-checked manifest versions, structured execution receipts and fenced side effects.
- Real two-tenant transport partition/recovery tests, hostile metadata fuzzing at scale, restart/upgrade and operator GO. OSTc must not be silently enabled, and neither MCP nor a simulated backend may be substituted for physical evidence.

## Completion rule

Stage 0.43 remains **ACTIVE_TECHNICAL_OPEN** until all gates pass on a documented commit with positive/negative CI run URLs, artifact digests and explicit operator acceptance. This file is an additive stage specification; the canonical ROADMAP, its mirror, manifest and changelog must be reconciled without deleting the historical MCP record.
