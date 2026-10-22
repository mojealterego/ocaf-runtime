# Katalog 33 wyspecjalizowanych grup agentowych

**Źródło:** `docs/agents/swarm_groups_v1.json` • **Stan:** rejestr operacyjny, nie dowód ukończenia etapów.

Każda grupa ma jedną specjalizację. Po zamknięciu etapu z dowodem pobiera następny kwalifikowalny etap; nie może przeskoczyć zależności ani oznaczyć pracy jako ukończonej bez artefaktu i testu.

| ID | Specjalizacja | Zakres | Handoff |
|---|---|---|---|
| G1 | `orchestration-runtime` | leases, dependencies and stage handoff | claim the next eligible stage after verified completion |
| G2 | `rust-toolchain` | cargo, rustc, rustfmt and clippy verification | claim the next eligible stage after verified completion |
| G3 | `core-runtime` | Rust runtime, async execution and lifecycle | claim the next eligible stage after verified completion |
| G4 | `rpc-transport` | RPC identity, TLS and transport boundaries | claim the next eligible stage after verified completion |
| G5 | `postgres-ha` | PostgreSQL leases, failover and persistence | claim the next eligible stage after verified completion |
| G6 | `zenoh-messaging` | Zenoh transport, SHM and topic isolation | claim the next eligible stage after verified completion |
| G7 | `security-policy` | authorization, privilege edges and fail-closed policy | claim the next eligible stage after verified completion |
| G8 | `supply-chain` | dependency pinning, SBOM and provenance | claim the next eligible stage after verified completion |
| G9 | `sandbox-execution` | WASI, Firecracker and isolated execution | claim the next eligible stage after verified completion |
| G10 | `agent-ir` | typed graph IR and schema evolution | claim the next eligible stage after verified completion |
| G11 | `builder-code` | supported Code DSL parsing and diagnostics | claim the next eligible stage after verified completion |
| G12 | `builder-nocode` | visual graph model and NoCode persistence | claim the next eligible stage after verified completion |
| G13 | `builder-conversion` | Code to NoCode round-trip and lossiness checks | claim the next eligible stage after verified completion |
| G14 | `cloud-workspace` | tenant workspaces, RBAC and audit | claim the next eligible stage after verified completion |
| G15 | `desktop-windows` | Windows desktop packaging and offline state | claim the next eligible stage after verified completion |
| G16 | `desktop-macos` | macOS packaging, signing and entitlements | claim the next eligible stage after verified completion |
| G17 | `mobile-android` | Android build, lifecycle and offline persistence | claim the next eligible stage after verified completion |
| G18 | `mobile-ios` | iOS build, lifecycle and sandbox constraints | claim the next eligible stage after verified completion |
| G19 | `publish-verification` | artifact signing, publish approval and receipts | claim the next eligible stage after verified completion |
| G20 | `e2e-platform` | cross-platform E2E fixtures and run evidence | claim the next eligible stage after verified completion |
| G21 | `ui-accessibility` | design tokens, accessibility and visual contracts | claim the next eligible stage after verified completion |
| G22 | `shader-gpu` | shader validation, limits and deterministic builds | claim the next eligible stage after verified completion |
| G23 | `ast-cst` | AST/CST repository analysis and patch structure | claim the next eligible stage after verified completion |
| G24 | `patch-admission` | CAS patches, path policy and protected files | claim the next eligible stage after verified completion |
| G25 | `test-harness` | negative tests, fixtures and reproducible harnesses | claim the next eligible stage after verified completion |
| G26 | `observability` | traces, metrics, logs and operator evidence | claim the next eligible stage after verified completion |
| G27 | `data-provenance` | document lineage, tenant scope and deletion | claim the next eligible stage after verified completion |
| G28 | `mcp-integrations` | MCP schemas, tool grants and injection resistance | claim the next eligible stage after verified completion |
| G29 | `cost-governance` | quotas, token budgets and cost exhaustion | claim the next eligible stage after verified completion |
| G30 | `recovery-chaos` | crash recovery, retries and fault injection | claim the next eligible stage after verified completion |
| G31 | `release-engineering` | release bundles, CI gates and rollback | claim the next eligible stage after verified completion |
| G32 | `independent-review` | adversarial review and acceptance evidence | claim the next eligible stage after verified completion |
| G33 | `integration-qa` | cross-stage integration and regression triage | claim the next eligible stage after verified completion |

## Wspólna bramka

Status grupy oznacza gotowość do pracy, nie ukończenie produktu. Każdy handoff musi zawierać identyfikator etapu, commit lub digest artefaktu, polecenie testu, wynik oraz listę znanych ograniczeń. Dla wariantów Agents Builder wymagany jest osobny build platformy; `OCAF-AGENT-001` nie zastępuje dziesięciu wariantów.

Rozszerzony rejestr 111 osobnych pakietów agentów zgodny z bieżącą dyspozycją 33 rojów: [`AGENT_REGISTRY_111.md`](AGENT_REGISTRY_111.md) oraz
[`agent_registry_111.json`](agent_registry_111.json). Wszystkie pakiety agentów startują jako
`DRAFT/NOT_DEPLOYED`; nie są aktywnymi agentami runtime.
