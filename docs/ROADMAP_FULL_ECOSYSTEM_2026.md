# OCAF FULL ECOSYSTEM ROADMAP — 26 PDF corpus / v2

**State: 2026-10-13.** Repository: `mojealterego/ocaf-runtime`, **branch `main` only**.
This document **supersedes the provisional 0.36–0.50 roadmap** in
`docs/MASTER_ARCHITECTURE_26_PDFS.md`. It is a requirements traceability
and milestone plan, not proof that the described systems already work.

## Mandatory scope and coverage

All identifiable components in the 26 supplied PDFs, including **every
named specialist swarm**, are included as traceable deliverables. Repeated
passages from successive revisions are consolidated (not reimplemented
as copies). The `Jarvis.PDF` comparison adds OmniMath. Sections
1–33 of the last extended OCAF architecture map into the table below;
older context-engineering drafts are covered by the MCP, OAuth,
sandbox, L402, enterprise connectors and evaluation milestones.

**No element is silently dropped.** Every row must subsequently have:
owner, dependencies, implementation issue, threat model, executable
tests, negative acceptance case, docs and traceable commit. Today
most modules are **not implemented**, despite their names/code sketches
appearing in PDFs. Research is explicitly separated from deployable
products. All changes are committed to `main` only.

## Phase boundaries, order and gates

- **Foundation Security (0.36–0.42):** finish identity, actual transport
  binding, execution isolation, fencing/HA, recovery, review. Production
  stays NO-GO until independently demonstrated.
- **Core Platform (0.43–0.55):** MCP tool plane, OAuth, optional PQC,
  non-authoritative CRDT, OTLP, provenance, economics, and portable
  Cloud/Desktop/Mobile Agents Builder.
- **Specialist Agent Swarms (0.56–0.71):** implement functional
  secure-by-default domain modules behind common capability gates.
- **Reasoning & Research (0.72–0.83):** provide testable experimental
  interfaces; never mistake conceptual pseudocode for mathematical
  proof, live autonomous RSI or safety certification.
- **Integration and Final Acceptance (0.84):** gated artifacts for the
  selected release footprint. Unfinished research modules remain marked
  experimental instead of blocking a precisely scoped runtime release.

The following numbers are **execution/backlog IDs**, not Cargo semantic
versions; some may be combined after written review without losing
traceability. Version numbers advance only after real release approvals.

## Complete execution matrix

| ID | Scope/module | Deliverable | Gate | Evidence needed |
|---|---|---|---|---|
| 0.36 | Identity/RPC | Application-layer certificate private-key proof + HMAC and pinning | Implement (opt-in) | Existing CI tests; genuine transport TLS peer binding remains blocked |
| 0.37 | Identity/RPC | Separate TLS 1.3-bound ingress verifies live session leaf against RPC sender pin and SAN; CRL/reload/worker migration remains | Implemented experimentally / NO-GO | Real handshake and swapped-cert tests in CI; external PKI/CRL and multi-host still pending |
| 0.38 | Execution | Default-deny deterministic WASI runner for untrusted tool execution | P0 / NO-GO | No network/host FS, bounded CPU/memory/time, hostile payload tests |
| 0.39 | Execution | Firecracker microVM broker + eBPF Aya controls + isolation attestations | P0 / NO-GO | Linux/KVM negative isolation, teardown, syscall/network constraints; mobile separate |
| 0.40 | HA | Three physical host PostgreSQL HA, real leader fencing and failover | P0 / NO-GO | Host power loss, split-brain, former leader effect-writes rejected |
| 0.41 | Reliability | Backup/restore, partition chaos, recovery and prolonged soak | P0 / NO-GO | RTO/RPO SLO, restore proof, job audit recovery, operator acceptance |
| 0.42 | Release | Production hardening, SBOM/provenance, license and audit GO/NO-GO | P0 / NO-GO | External security review; release version not mistaken for priority number |
| 0.43 | MCP | Dynamic discovery, semantic routing, manifest signature, Tree-sitter & JSON Schema | P1 | Untrusted tool metadata cannot inject policy; unknown tool never executes |
| 0.44 | OAuth | OAuth 2.1 authorization code + PKCE, scoped tenant tokens, secret broker | P1 | PKCE, CSRF, confused deputy, issuer/audience/replay test matrix |
| 0.45 | Crypto | Hybrid PQC ML-KEM/ML-DSA interoperability, ZKP caveat proof experiments | P2 / research | No replacement of audited mTLS until standards/tests pass |
| 0.46 | Transport | Zenoh SHM typed zero-copy, CRDT Automerge/Yrs for non-authoritative state | P1 | Benchmarks and CRDT convergence; NEVER for leadership/payment authority |
| 0.47 | Telemetry | W3C tracing propagation, OpenTelemetry/OTLP metrics, SLA and audit logs | P1 | No secret leakage; trace replay across actual host boundaries |
| 0.48 | Data | Enterprise ingest, connectors, retrieval, provenance, bitemporal policy | P1 | Tenant isolation, document provenance, deterministic authorization |
| 0.49 | Economics | L402/x402 adapter, testnet invoice/preimage verification, offline escrow | P2 | Idempotent ledger, upper spend limits, explicit human confirmation |
| 0.50 | Builder Core | ocaf-builder-core canonical typed DAG/IR, manifest compiler | P1 | JSON/DSL round-trip property tests, graph/effect policy consistency |
| 0.51 | Builder UX | No-Code canvas <-> bounded supported DSL and conflict-aware CRDT | P1 | Preserve editable DSL structure; reject unsupported Rust round trips |
| 0.52 | Cloud Builder | Axum multi-tenant workspace backend, RBAC and audit | P1 | Cross-tenant access probes, quotas, auth, controlled deployments |
| 0.53 | Desktop Builder | Tauri v2 local-first offline desktop IDE | P2 | Offline edit, local runner restrictions, signed update flow |
| 0.54 | Mobile Builder | Rust/UniFFI + SwiftUI/Compose + bounded mobile agent runtime | P2 | iOS/Android builds, no Firecracker assumption, offline behavior tests |
| 0.55 | Builder Safety | Graph static verification, typed edges, critical caps, WASI preview | P1 | Cycle/privilege-edge/cost-exhaustion negative fixtures |
| 0.56 | OmniDev | AST/CST code agents, repo analysis, codegen, test harness | P1 | Compilable patches, tests, human-gated protected updates |
| 0.57 | OmniUX | Figma design integration, accessible component UI and visual diff | P2 | Design token checks, visual regression, accessibility |
| 0.58 | OmniE2E | Cross-platform app build and E2E test swarm | P2 | Reproducible Android/iOS/Desktop builds and QA artifacts |
| 0.59 | OmniForge | Game development/CAD/GPU shader swarm | P3 | Sandboxed shader builds, GPU budgets and asset policy |
| 0.60 | OmniWeb | Authorized web extraction, workflow automation, site generation | P2 | Permissions, rate limits, provenance; no default anti-bot bypass |
| 0.61 | OmniStudio | Image/audio/video media orchestration swarm | P2 | Model/provider adapters, asset provenance, rights, spending quotas |
| 0.62 | OmniDoc | Typst/report/code-aware documentation swarm | P2 | Reproducible PDFs and citations; source-map tests |
| 0.63 | OmniSec | Purple Team validation in authorized scoped targets | P1 | Authorization checks, non-destructive default, explicit approval |
| 0.64 | OmniGov | Governance/approval/constitutional policy control plane | P1 | Tamper-evident approvals, manual override constraints |
| 0.65 | OmniFin | FinOps and cost-exhaustion defenses | P1 | Per-agent quotas and denied over-budget purchases |
| 0.66 | OmniPhys | Robotics and physical-action connector swarm | P3 | Simulation-first, e-stop, separate physical permissions |
| 0.67 | OmniCog + OmniGraph | Bitemporal memory and provenance-aware knowledge DAG | P1 | Retention, merge lineage, tenant boundaries, stale-memory tests |
| 0.68 | OmniSleep + OmniPrune | Scheduled consolidation, semantic cache, knowledge decay | P2 | Safe pruning with rollbacks, benchmark vs baseline |
| 0.69 | OmniArbitrage | Model SLA/BATNA routing, fallback, epsilon exploration | P1 | Measured quality/cost/latency under adversarial outages |
| 0.70 | OmniContract + OmniChaos | Typed runtime contracts, circuit breakers, load shedding | P1 | Bounded retries and negative dependency/chaos tests |
| 0.71 | OmniStrategy | DAG planning, counterfactual checks, error memories | P2 | Bounded search and independent step outcome verification |
| 0.72 | OmniSense + OmniEvolve | Graph planning and governed self-improvement candidates | Research | Shadow-only candidates, no unreviewed promotion to production |
| 0.73 | OmniVerify + OmniHomeostasis | SMT-defined invariants and homeostatic resource budgets | Research | Proof scope explicit; arbitrary program termination not provable |
| 0.74 | OmniCausal | Counterfactual simulation and immutable deployment approvals | Research | Modelled causal assumptions and no unattended hot-swap |
| 0.75 | OmniLatent + OmniAdaptive | Typed embeddings, work stealing and sandboxed ephemeral tools | P2/research | Model/dimension binding, leased ownership and isolation tests |
| 0.76 | OmniMath | Independent Lean 4 proof kernel and selected Z3/cvc5 e-graph cases | P2/research | Certified selected theorems; LLM consensus not evidence |
| 0.77 | OmniInference + OmniTDA | Active inference, epistemic topology and diagnostics | Research | Repeatable metrics with falsifiable hypotheses |
| 0.78 | OmniHDC + Sparse SAE | Hyperdimensional computing and sparse feature analysis | Research | Interpretability benchmarks and leakage checks |
| 0.79 | OmniFractal + OmniNARS | Hierarchical delegation, uncertainty logic, bounded recursion | Research | No unconstrained self-replication; bounded termination budgets |
| 0.80 | OmniCategory + OmniLogic | Category-theoretic mapping, multi-valued logic | Research | Precisely defined type-preserving mappings and theorem checks |
| 0.81 | OmniTranspile + PCC | Verified Wasm/Cranelift modules, Hoare pre/postconditions | Research | Independent checker for defined properties; signature not a proof |
| 0.82 | OmniThermo + OmniQUBO | Energy cost models, optimization constraints and QUBO experiments | Research | Real power telemetry, optimization reproducibility |
| 0.83 | OmniAlign | Human-aligned safety gates, conformal uncertainty, refusals | Research | Adversarial evaluation, audited policy, human approval for risky actions |
| 0.84 | End-to-end acceptance | System integration, external review and product release readiness | Release gate | Signed acceptance for each supported optional domain pack |

## Mapping to final extended PDF (chapters 1–33)

| Original sections | Coverage in roadmap |
|---|---|
| 1.1 Zenoh SHM, 1.2 CRDT, 1.3 OpenTelemetry | 0.46, 0.47; trusted core 0.40 |
| 2 MCP Tree-sitter schema fusion | 0.43 |
| 3 OAuth, Macaroons, ZKP | 0.44–0.45 |
| 4 Firecracker and eBPF | 0.38–0.39 |
| 5 L402/offline payment | 0.49 |
| 6 PQC and neuromorphic acceleration | 0.45, 0.82 experiments |
| 7 OmniDev | 0.56 |
| 8 OmniUX | 0.57 |
| 9 OmniE2E | 0.58 |
| 10 OmniForge | 0.59 |
| 11 OmniWeb | 0.60 |
| 12 OmniStudio/OmniDoc | 0.61–0.62 |
| 13 OmniGov/OmniFin/OmniPhys | 0.64–0.66 |
| 14 OmniSec | 0.63 |
| 15 OmniCog | 0.67 |
| 16 OmniEvolve | 0.72 |
| 17 OmniSense | 0.72 |
| 18 OmniArbitrage | 0.69 |
| 19 OmniContract/OmniChaos | 0.70 |
| 20 OmniStrategy/OmniGraph | 0.71/0.67 |
| 21 OmniVerify/OmniHomeostasis | 0.73 |
| 22 OmniCausal and governed RSI | 0.74 |
| 23 OmniSleep/OmniPrune | 0.68 |
| 24 OmniLatent/OmniAdaptive | 0.75 |
| 25 OmniInference/OmniTDA | 0.77 |
| 26 OmniHDC/Sparse SAE | 0.78 |
| 27 OmniFractal/OmniNARS | 0.79 |
| 28 OmniCategory/OmniLogic | 0.80 |
| 29 OmniTranspile/Proof-Carrying Code | 0.81 |
| 30 OmniThermo/OmniQUBO | 0.82 |
| 31 OmniAlign | 0.83 |
| 32 Cloud/Desktop/Mobile Agents Builder, Code/No-Code, UniFFI | 0.50–0.54 |
| 33 DAG validator, cost ceiling, WASI preview | 0.55 |
| Separate Jarvis.PDF: OmniMath and theorem checkers | 0.76 |

## Shared non-functional constraints across **all** swarms

1. Per-agent, per-tenant identity, signed capabilities, mTLS,
   OAuth audience/scope, explicit egress allowlists.
2. No tool has authority merely because its LLM says it needs it.
   Independently enforce budgets, approvals, effect fencing, nonce
   replay and credential expiry.
3. Each plugin/specialist swarm runs in a constrained execution
   plane, not as privileged code within `ocaf_runtime` core.
4. Signed supply-chain artifacts, testable integrations, dependency
   audits, pinned trust anchors and reversible deployments.
5. Domain-specific policies: rights for media, consent for scraping,
   explicit budget for L402/x402, authorization for security testing,
   and simulation/emergency-stop for robotics.
6. CRDT cannot decide single-writer leadership, privileged
   authorization or financial settlement; only strongly consistent
   durable ledgers govern those fields.
7. Performance is measured, never asserted from vendor benchmarks.
   Claims such as sub-millisecond Zenoh, limitless scale, zero
   hallucination and mathematically guaranteed unrestricted
   code safety require scoped experiments or must be removed.
8. Graph↔Code synchronization can only guarantee round-trip for
   a formalized DSL subset, **not arbitrary Rust programs**.
9. Self-modifying agents can propose, test and document
   changes but cannot deploy unreviewed patches autonomously.
10. Every stage updates **README.md** (including historical 0.31),
    documentation, deterministic tests and GitHub Action evidence.

## Release interpretation

- **Runtime candidate:** after 0.36–0.42 *and* independently
  verified physical HA and transport identity acceptance.
- **Agents Builder candidate:** after 0.43–0.55 platform
  functionality is proven on all explicitly supported devices.
- **Specialist swarms:** selectively promoted after individual
  domain-specific acceptance, without elevating kernel privileges.
- **Advanced cognition:** independent research track until the
  stated mathematical and security hypotheses are actually verified.

**Current production decision: NO-GO.**


## Execution status — 0.37 (2026-10-13)

Added opt-in TCP/mTLS ingress with actual certificate identity from
the same TLS session as the RPC request. Unlike the Zenoh query API,
the rustls handler exposes WebPKI-verified client certificates and
maps them to configured RPC sender fingerprints and exact SANs.
The new endpoint is **exclusive** with the legacy Zenoh queryable
within `ocaf-coordinator`. End-to-end test uses distinct real
client TLS certificates, including a valid HMAC+application PoP
with the *wrong* mTLS certificate (must reject). CRL/OCSP,
certificate renewal, migration of remote worker roles, real multi-host
host isolation and external acceptance remain **open blockers**.
See [TLS_PEER_IDENTITY_037.md](TLS_PEER_IDENTITY_037.md).

The entire 0.38–0.84 roadmap, including *all specialized agent swarms*,
remains mandatory scope. No experimental module is claimed complete
merely because its architecture is described in the source PDFs.
