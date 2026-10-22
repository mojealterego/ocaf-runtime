# OCAF Engineering Legion — repository-grounded knowledge baseline

Source: `mojealterego/ocaf-runtime`, branch `main`, read 2026-10-17.
This is a **partial verified knowledge baseline**, not a complete codebase ingestion or a deployed agent fleet.

## Authoritative documents
- `README.md`: pinned roadmap and non-negotiable "ZAKAZ SAMOWOLKI" governance. Contains older 289-position summaries alongside a newer 407-position header; treat older figures as stale and reconcile against canonical manifest before edits.
- `ROADMAP.md`: current headline 333 main + 64 technical + 10 Agents Builder products = 407; next stage 334. Internal archival sections retain earlier counts and should not be treated as current.
- `docs/roadmap/canonical_v11_inventory.json`: main_count=333, technical_count=64, production_approved=false.
- `docs/roadmap/agents_builder_products_v11.json`: ten Code/NoCode products across Cloud, Windows, macOS, Android and iOS; all NOT_IMPLEMENTED in the inspected manifest; parent stages 0.50–0.55.
- `Cargo.toml`: Rust package `ocaf_runtime` version 0.26.0, edition 2021.
- `src/lib.rs`: modules auth, cluster, coordinator, executor, jobs, metrics, ops, registry, rpc, runtime, sink, transport_policy; feature-gated zenoh transport.

## Critical implementation gates (from ROADMAP.md)
- 0.39: real broker → jailed Firecracker guest → signed WASI execution with guest-side proof, two tenants and negative tests; technical completion not verified.
- 0.40: PostgreSQL WAL replication across independent kernels/hosts, witness/fencing, recovery; technical completion not verified.
- 0.41: soak/chaos, RTO/RPO, p95/p99, offsite backup and real restore; technical completion not verified.
- 0.42: supply chain, signed release, rollback and operator GO/NO-GO.
- Production approval is false. Historical administrative closure is not technical or production acceptance.

## Roles and routing

The current organizational directive expands the operating catalog to 33 swarms
and 111 separate agent packages. The packages have individual identity and structure; they are not yet deployed runtime processes, production agents, or completed stages. See
`docs/agents/AGENT_REGISTRY_111.md` and `agent_registry_111.json`.

| Agent | Repository expertise |
|---|---|
| architect-prime | typed contracts, Rust crate structure, architecture boundaries |
| rust-titan | Rust implementation, safe async, code quality |
| zenoh-mesh | transport_policy, transport, authenticated RPC |
| wasi-forge | executor, Firecracker/WASI acceptance 0.39 |
| tenant-sentinel | tenant isolation, identity, authorization |
| postgres-ha | cluster, fencing, PostgreSQL acceptance 0.40 |
| mcp-gateway | registry, tool gateway, MCP adapters |
| security-sentinel | auth, dependency/supply-chain security |
| test-forge | tests, fault injection, acceptance evidence |
| devops-atlas | CI, infrastructure reproducibility |
| sre-guardian | ops, metrics, DR/SLO acceptance 0.41 |
| agent-builder-cloud | Cloud Code/NoCode products, 0.50–0.55 |
| agent-builder-desktop | Windows/macOS Code/NoCode products |
| agent-builder-mobile | Android/iOS Code/NoCode products |
| roadmap-curator | canonical inventory and append-only stage history |
| release-commander | release evidence, audit and operator acceptance |

## Mandatory operating policy
1. Refresh `main` commit SHA and read affected files before making a code change; this document is not a substitute for source inspection.
2. Never create another branch. No force pushes. Use optimistic SHA-bound writes.
3. Preserve the pinned README roadmap, historical IDs, manifest, mirror and changelog; append truly new stages only after the latest verified ID.
4. Require reproducible tests and evidence. No invented latency, security-audit signatures, hardware, deployment or acceptance.
5. No uncontrolled model-generated code execution; use isolated sandboxes, least privilege and explicit approval for production effects.
6. Distinguish structured individual agent packages from real LLM-connected runtime activation; each package needs its own model, endpoint, artifact and evidence.
7. Keep original historical documentation intact; conflicting counters must be reported, not silently overwritten.

## Ingestion coverage and outstanding work
Verified reads: `README.md`, `ROADMAP.md`, `Cargo.toml`, `src/lib.rs`, canonical v11 inventory (partial excerpt), Agents Builder manifest (partial excerpt).
Not yet indexed: complete source tree, full test corpus, history, CI runs, all PDF source materials, and remaining manifest entries.
No claim of completed full ingestion, passing CI, independent security audit, or production rollout.
