# OCAF FULL ECOSYSTEM ROADMAP — consolidated evidence-led v7 (2026-10-15)

## Mandat operatora: ciągły rozwój bez odrębnych potwierdzeń (2026-10-15)

Operator zatwierdził **kontynuowanie wszystkich etapów i rozszerzanie roadmapy bez wstrzymywania developmentu dla niezależnych audytów lub osobnych podpisów**. Autoryzacja zapisana w [ops/OCAF_CONTINUOUS_DEVELOPMENT_POLICY_2026-10-15.json](../ops/OCAF_CONTINUOUS_DEVELOPMENT_POLICY_2026-10-15.json). Zakres: proces rozwoju; rzeczywiste techniczne testy nadal obowiązują. Można prowadzić 0.40 i 0.41, gdy 0.39 ma jeszcze zadania integracyjne; statusy historyczne stanowią rzetelny rejestr. [0.40/0.41: dowody rzeczywistych testów](STAGE040_041_REAL_POSTGRES_LABS_2026-10-15.md). **Wdrożenia i bezpieczeństwa produkcyjnego nie wolno domniemywać.**


## Ostateczny status prac 0.38

**Etap 0.38: COMPLETE — CLOSED_BY_OPERATOR (2026-10-15).**

Jest to **zamknięcie kamienia milowego w planie projektu**, nie atest
bezpieczeństwa. Kryteria techniczne A01–A12 wymagają niezależnej walidacji,
audyt pozostaje nieukończony, a stan `production_approved=false` nie zmienia się.
Dalszy rozwój prowadzony jest w 0.39 bez cofania roadmapy do 0.38.
Każdy wpis `PARTIAL` poniżej jest historyczną oceną ówczesnych dowodów.

**Revision: 2026-10-14.** The dated historical notes remain below. Repository: `mojealterego/ocaf-runtime`, **branch `main` only**.
This document **supersedes the provisional 0.36–0.50 roadmap** in
`docs/MASTER_ARCHITECTURE_26_PDFS.md`. It is a requirements traceability
and milestone plan, not proof that the described systems already work.

**Revision v5 (2026-10-15):** added the two new user-supplied documents,
covering 24 + 9 reference repositories in ten integration groups; primary
traceability document:
[33-repository adaptation matrix](ROADMAP_33_REPOSITORY_ADAPTATIONS_2026-10-15.md).
Stage 0.39 execution milestones expanded below. Upstream descriptions and
code examples in PDFs are *proposals*; they are not production-test evidence.

## Revision v7 — dwa dodatkowe katalogi Rust, 29 stron (2026-10-15)

Uwzględniono **dwa nowo dostarczone PDF-y**: 15-stronicowy przegląd `OmniNous/OmniHoncho/OmniCloudShield/OmniMask/OmniVox/OmniGrant` oraz 14-stronicowy przegląd `OmniZeroClick/OmniGraphiti/OmniDiT/OmniGameSubsystems/OmniMCPO/OmniCloudBurst`. Zgodnie z dokumentami i techniczną selekcją zostało zapisanych **15 dodatkowych etapów 1.00–1.14** oraz P0 testy **0.39.19–0.39.21, 0.40.3–0.40.5, 0.41.5–0.41.6** wraz z zależnościami i falsyfikowalnymi warunkami zakończenia. Nie uznajemy przykładowego kodu w PDF za wdrożony komponent; nie obiecujemy skuteczności ochrony przed zero-click, rzeczywistego DID/eSIM, nieposiadanej infrastruktury GPU, zewnętrznej certyfikacji ani dostępu do opłaconych API. **Wersja źródłowa i kompletna matryca śledzenia: [ROADMAP_TWO_PDF_RUST_MODULES_2026-10-15.md](ROADMAP_TWO_PDF_RUST_MODULES_2026-10-15.md).**

Pierwsze obowiązkowe priorytety wykonawcze: **rzeczywisty WASIp1 import wykonany przez interpreter *wewnątrz* Firecracker guest i związany z autoryzacją brokera**, następnie **rzeczywiste PostgreSQL WAL streaming między osobnymi kernelami przez żywy TCP i fencing**, a dopiero potem **wielogodzinne testy chaos/soak**. Obserwacje pośrednie nadal mają własne dowody, lecz nie są równoznaczne z zamknięciem tych kryteriów.

## Mandatory scope and coverage

All identifiable components in the 26 supplied PDFs, including **every
named specialist swarm**, are included as traceable deliverables. Repeated
passages from successive revisions are consolidated (not reimplemented
as copies). The `Jarvis.PDF` comparison adds OmniMath. Sections
1–33 of the last extended OCAF architecture map into the table below;
older context-engineering drafts are covered by the MCP, OAuth,
sandbox, L402, enterprise connectors and evaluation milestones.

**Supplementary review (2026-10-13):** four newly attached PDF files
(three versioned revisions of the OCAF/JARVIS research report and one
Frontier 2027–2033 demonstrator, **48 file-pages including overlaps**)
expand the roadmap rather than replacing the existing 26-file corpus.
The 30 count is **file versions**, not 30 unique specifications.
Added research/verification milestones 0.85–0.94, mapped below.
New content from the PDFs is *proposed research*, not verified runtime.

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
- **Frontier Scientific Research (0.85–0.94):** genuinely new requirements
  from the four supplementary file versions. These are separate research
  milestones, not a prerequisite to starting or shipping a restricted
  security-reviewed runtime footprint. Extraordinary claims require
  independent benchmarks, proofs or experimental physics; physically
  unsupported claims are explicitly held at NO-GO.

The following numbers are **execution/backlog IDs**, not Cargo semantic
versions; some may be combined after written review without losing
traceability. Version numbers advance only after real release approvals.

## Complete execution matrix

| ID | Scope/module | Deliverable | Gate | Evidence needed |
|---|---|---|---|---|
| 0.36 | Identity/RPC | Application-layer certificate private-key proof + HMAC and pinning | Implement (opt-in) | Existing CI tests; genuine transport TLS peer binding remains blocked |
| 0.37 | Identity/RPC | Separate TLS 1.3-bound ingress verifies live session leaf against RPC sender pin and SAN; CRL/reload/worker migration remains | Implemented experimentally / NO-GO | Real handshake and swapped-cert tests in CI; external PKI/CRL and multi-host still pending |
| 0.38 | Execution | Restricted Wasmtime WASIp1 adapter (see [implementation](WASI_EXECUTION_038.md) and [acceptance contract](PRIORITY_038_EXECUTION_CONTRACT.md)) | **COMPLETE — OPERATOR-RECORDED MILESTONE / technical A01–A12 INCOMPLETE; production NO-GO** | Operator requested project-level closure and transition to 0.39; documented in [directive](STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md). Passing CI is not persistent two-host acceptance, independent signed security audit or verified production release. |
| 0.39 | Execution | Firecracker microVM broker + Aya/eBPF isolation | **REAL BROKER SO_PEERCRED ADMISSION→TWO JAILED VM KERNEL BOOTS/FAULTS PASS; SIGNED WASI INSIDE GUEST OPEN** | [Real Linux KVM CI 38739812121](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812121): two per-tenant kernel SO_PEERCRED AF_UNIX broker reservations with SHA-pinned netless manifests persisted in SQLite BEFORE launching two simultaneous jailed Firecracker Linux guests; lease/VM id/cgroup/boot hash and SIGKILL/peer survival receipts bound, leases released. [Ledger](STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md). Signed WASI **inside guest not implemented**; production NO-GO, independent signer gates waived for development only. |
| 0.40 | HA | Three physical host PostgreSQL HA, real leader fencing and failover | **THREE DISTINCT KERNEL PG HA PROBES PASS + 30s REAL 8-CLIENT WAL SOAK/REPLICA PARITY/CRASH PASS; INTERHOST HA STILL OPEN** | [3 kernel PG failover run 38740936728](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936728) each host independently runs 3 real PG processes, primary failure, B promotion/C reparent; [8-client 30s pgbench WAL stress and A crash run 38740936947 SUCCESS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936947) proves WAL balance parity with B/C while streaming and post-load failover. **No live PostgreSQL replication between different kernel hosts**; no split-brain lease under real host partition/whole host loss. [Evidence](STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md). |
| 0.41 | Reliability | Backup/restore, partition chaos, recovery and prolonged soak | **REAL PHYSICAL BASEBACKUP + ARCHIVED WAL PITR ON TWO DISTINCT LINUX KERNELS PASS; PROD DR/SOAK OPEN** | [Physical WAL point-in-time restore on separate kernel run 38738198412](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412): primary IMMEDIATE crash, SHA-pinned physical base and WAL tar sent via GitHub artifact, second kernel fresh PG replay to target LSN, 35/35 recovered, 5 later committed rows omitted; compromised bytes and replayed source boot ID rejected. [Detailed evidence](STAGE041_TWO_KERNEL_PHYSICAL_WAL_PITR_2026-10-15.md). Durable physical server power loss, encrypted offsite retention, actual automated RTO/RPO under soak still OPEN. |
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
| 0.85 | OmniProof-STARK | Research a real verifier for formally specified task predicates, not unrestricted model reasoning | **Research / NO-GO** | Concrete relation, witness/public statement separation, soundness assumptions, valid/invalid/corrupted-proof tests, measured proof sizes/costs |
| 0.86 | OmniMacro / Causal Emergence | Experimentally aggregate micro-agent state without losing per-agent audit, permissions or fencing | **Research / NO-GO** | Baseline, reversible state aggregation, information loss bounds, effect equivalence, falsifiable causal metrics |
| 0.87 | OmniMarket-L402+ | Optional payment-channel state machine atop 0.49; no direct production settlement authority | **Research / NO-GO** | Signed sequence/nonces, channel conservation, two-sided dispute and timeout handling, durable replay and manual spending limits |
| 0.88 | OmniSubstrate | Sandboxed Wasm/Cranelift specialization of *approved pure* hot paths, optional SIMD, exploratory FPGA | **Research / NO-GO** | Equivalence tests, independent limits and signatures; performance measured vs baseline, deterministic fallback |
| 0.89 | Proof-Carrying RSI | Candidate code changes with independently checkable Lean/SMT contracts and human-authorized deployment | **Research / NO-GO** | Real proof kernel, property scope, compiled artifact binding, adversarial invalid proofs; no automated production hot-swap |
| 0.90 | OmniThermo-Governor | Hardware-sourced RAPL/power telemetry and budget policy extending 0.82/0.65 | **Research / NO-GO** | Calibration vs wall power, sensor absence/rollover, per-tenant attribution error, enforcement tests; simulated values labelled |
| 0.91 | Neuromorphic HIL / Loihi 2 | Optional experimental SNN routing adapter with CPU/GPU fallback | **Research / hardware dependency** | Actual device access and reproducible energy-delay-quality data; no claimed production acceleration |
| 0.92 | Biocomputing / wetware | Ethics-first feasibility study, not an implementation commitment of biologically based cognition | **Exploratory / NO-GO** | Independent scientific validation, safety/biosafety/legal and ethics review; no live human-derived materials under current scope |
| 0.93 | Optical/holographic state | Investigate optical storage/links as laboratory alternatives; do not replace authoritative PostgreSQL | **Exploratory / NO-GO** | Physical experimental design, measured bandwidth/latency, consistency under faults; reject zero-latency or nonlocal synchronization claims |
| 0.94 | Topological error correction / PQC | Evaluate defined fault-tolerant coding and cryptographic constructions separately | **Exploratory / NO-GO** | Standards-backed threat model and independently reviewed proof/security analysis; no 'gravity-proof' guarantee |

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

The entire 0.38–0.94 roadmap, including *all specialized agent swarms*,
remains mandatory scope. No experimental module is claimed complete
merely because its architecture is described in the source PDFs.

---

## Aktualizacja przed 0.38 — stan weryfikacji (2026-10-13)

**Ostatni sprawdzony commit źródłowy przed aktualizacją planu:**
`6bfc77714e6ecb3a5bda65246bbc69641e1ea8f9`
(`main`, security: invalid transport selection rejected).
Na tym commicie GitHub Actions potwierdził **Rust CI success**
(`38398928258`) i **Supply Chain success** (`38398928287`).
To nie jest odbiór produkcyjny i nie jest wynik testów jeszcze
niezaimplementowanego sandboxa. Po kolejnych commitach sprawdzić
**ich własny** status CI.

**Inwentaryzacja kodu dla 0.38:** `src/executor/mod.rs` implementuje
abstrakcję `ToolExecutor` i timeout zadania; `src/registry/mod.rs`
kompiluje JSON Schema, odrzucając zewnętrzne `$ref`.
Na sprawdzonym `main` brak backendu Wasm/WASI, Firecracker i Aya
w `Cargo.toml`. Dlatego **0.38 = NOT STARTED, 0.39 = NOT STARTED**.
Etap 0.37 dostarczył alternatywny listener TLS-bound na rzeczywistej
sesji mTLS, lecz rotacja/revocation i migracja innych ról nie są gotowe.

### Przedimplementacyjny kontrakt etapu 0.38

Kanoniczne wymagania oraz przypadki RED/GREEN znajdują się w
[PRIORITY_038_EXECUTION_CONTRACT.md](PRIORITY_038_EXECUTION_CONTRACT.md).
Kolejność: (1) ADR silnika/WASI i wersji zależności; (2) izolowany
`WasmToolExecutor`; (3) podpis/digest i allowlista importów; (4)
host FS/network/secret **default-deny**; (5) bounded fuel, memory,
stack, wall time, concurrency, stdin/stdout; (6) ścisły profil
determinizmu; (7) integracja polityk i fencing; (8) testy ataków,
cleanup oraz metryki; (9) CI + audyt z dowodami końcowego commitu.
Nie wolno awansować tego etapu do `DONE` na podstawie projektowego
dokumentu ani sukcesu innego commitu.

### Macierz pokrycia czterech nowych załączników

| Materiał | Zawartość | Realizacja/backlog | Status dowodowy |
|---|---|---|---|
| Raport architektoniczny — wersja podstawowa (12 stron) | Rust/MCP, dynamiczna fuzja schematów, Zenoh/SHM, CRDT, L402/ZKP, Firecracker/eBPF, PQC, Loihi, finansowanie | 0.38–0.49, 0.82, 0.91, grant track | Wymagania/koncepcje; liczby wydajności z raportu wymagają pomiarów |
| Raport architektoniczny — rewizja (12 stron) | To samo jądro koncepcyjne, pomiary obiecanej wydajności i plan M0–M5 | 0.38–0.49, 0.91 | Powtórzenia deduplikowane, brak uznanego dowodu benchmarków |
| Raport architektoniczny — rozszerzenie (14 stron) | M0–M9, Loihi, kwantowo-neuromorficzne HIL, wetware, optyka holograficzna, 'kwantowo-grawitacyjna' kryptografia | 0.45, 0.90–0.94 | M6–M9 wyłącznie eksploracyjne; zero-latency / odporność na grawitację nieudowodnione |
| Frontier 2027–2033, „Dokument bez nazwy” (10 stron) | OmniThermo-Governor, OmniProof-STARK, OmniMacro, OmniMarket-L402+, OmniSubstrate, proof-carrying RSI | 0.82 i 0.85–0.90 | Kod demonstracyjny, brak dowodu implementacji STARK, Lean, JIT, RAPL lub rzeczywistych kanałów rozliczeń |

**Mapowanie M0–M9 bez tworzenia równoległych implementacji:** M0 =
0.43/Rust MCP; M1 = 0.46/SHM + CRDT; M2 = 0.49/0.87 L402/ZKP;
M3 = 0.38–0.39 WASI/Firecracker/eBPF; M4 = 0.43/0.45 dynamiczne
schematy i PQC; M5 = 0.91 neuromorfika; M6 = 0.91/0.94 HIL i
konstrukcje kryptograficzne; M7 = 0.92 wetware; M8 = 0.93 optyka;
M9 = 0.94 topologia/PQC. M1 nigdy nie daje uprawnień CRDT do
sterowania autorytatywnym liderem; M8 nie może zastąpić dziennika
finansowego ani leasingu PostgreSQL.

### Korekty twierdzeń technicznych z dokumentów wejściowych

- **STARK:** funkcja `verify_reasoning_trace` w załączonym pseudokodzie
  porównuje digest SHA-256; nie wykonuje weryfikacji dowodu STARK
  ani dowodu semantycznej prawdziwości wyniku modelu. Wariant realny
  wymaga jawnego twierdzenia, circuit/AIR, poprawnego systemu
  kryptograficznego oraz niezależnego verifiera.
- **RAPL:** `record_execution_energy` jawnie estymuje energię z liczby
  cykli; brak rzeczywistego odczytu czujnika. Limit Landauera to
  fizyczna granica teoretyczna, nie budżet produkcyjnego scheduler'a.
- **OmniMacro:** próg entropii i mnożnik `0.45` są heurystyką,
  a nie dowodem emergencji przyczynowej lub 55–85% oszczędności.
- **L402+**: mutacja dwóch pól `balance_agent_a/b` w pamięci
  nie realizuje podpisanego kanału z depozytem, arbitrażem, durable
  ledger i settlement; nie może wydawać środków realnych.
- **RSI / formalny hot swap:** uruchomienie `candidate_fn` z jednym
  `test_input` i ustawienie stałej sumy bajtów jako `binary_hash`
  nie dowodzi trójki Hoare'a. Wymagane niezależny checker i
  atestacja *konkretnego* binarium; każda produkcyjna zmiana
  wymaga zatwierdzenia człowieka.
- **Wasm/JIT:** fragmenty przedstawione jako 'OmniSubstrate'
  nie są automatycznie działającym Cranelift/FPGA/ASIC backendem;
  obietnice inferencji w nanosekundach wymagają niezależnych
  benchmarków pod zdefiniowanym obciążeniem.
- **Holograficzna pamięć/kwantowa grawitacja/wetware:** deklaracje
  jednoczesnego nie-lokalnego stanu, zerowych opóźnień albo
  odporności kryptografii na dowolne zakłócenia nie są kryteriami
  wdrożeniowymi. Najpierw fizycznie wykonalne hipotezy i etyka.
- **Rust/Zenoh/MCP:** wydajność, bezpieczeństwo, wersja protokołu
  i skalowalność zależą od konfiguracji; dokumenty nie
  certyfikują własnych porównań 16×/67 Gbps/10 000 agentów.
- **Licencje:** deklaracja `MIT` w `Cargo.toml` i zastrzeżenie
  w README wymagają uzgodnienia przed publicznym produkcyjnym
  wydaniem, bez cichej zmiany praw użytkowania.

### Grant/ESG — osobny strumień nietechniczny

Źródłowe raporty wymieniają NCBR/FENG SMART, PARP/Platformy
Startowe, regionalne inicjatywy na Śląsku, KFS oraz ESG.
Stan programów, aktualne nabory, regulaminy, kwalifikowalność,
kwoty i wymogi współfinansowania **nie zostały tu zweryfikowane**.
Backlog `G-01`: zestawić aktualne oficjalne ogłoszenia,
`G-02`: policzyć TCO i baseline energii oraz emisji,
`G-03`: przygotować oddzielną, mierzalną narrację B+R bez
nieudowodnionych gwarancji; nie mieszać tego z akceptacją kodu.

### Zależności, ryzyka i kolejność startu

`0.38` (WASI ograniczone wykonanie) → `0.39` (izolacja
host/KVM) → `0.40` (realne HA/fencing) → `0.41`
(disaster recovery/chaos/soak) → `0.42` (release review).
Prace `0.43–0.84` mogą mieć prototypy za flagą feature,
lecz wykonanie narzędzi o wysokim ryzyku **nie uzyskuje**
praw produkcyjnych przed bramkami bezpieczeństwa.
Wszystkie specialist swarms 0.56–0.71 pozostają w mapie.
Badania 0.85–0.94 nie odblokowują automatycznie wdrożenia.

**History before implementation:** ROADMAP UPDATED /
STAGE 0.38 NOT EXECUTED / PRODUCTION NO-GO.

## Zmiana stanu — rozpoczęcie etapu 0.38 (2026-10-13)

- `src/executor/wasi.rs`: pierwszy rzeczywisty, opcjonalny
  (przez ręczną rejestrację `ToolExecutor`) Linux/WASIp1
  subprocess adapter; żaden silnik Wasm nie jest ładowany
  do procesu zaufanego koordynatora. Zewnętrzny `wasmtime`
  musi być jawnie zainstalowany i przypięty digestem SHA-256.
- `tests/wasi_sandbox.rs`: negative tests importów, policy,
  digest, output/input bomb, timeout, schema integration oraz
  prawdziwe WebAssembly w scenariuszu `OCAF_TEST_WASMTIME`.
- `scripts/wasmtime_acceptance.sh` + Rust CI: pinned, upstream
  Wasmtime 49.0.2 i test realnego Wasm/WASI z dodatkowym
  testem wyczerpania fuel i limitu pamięci. Wynik końcowego
  CI należy sprawdzać osobno po każdym następnym commicie.
- `docs/WASI_EXECUTION_038.md`: ADR architektury, limity
  egzekwowane w warstwie narzędziowej, model zaufania
  oraz pozostałe blokery.

**Ocena historyczna z chwili pierwszego wdrożenia:** 0.38.1–0.38.6/0.38.9
miały kod i testy, natomiast podpisany manifest, pełny scoping
remote-worker i niezależny audyt pozostawały otwarte. Nowszy
stan implementacji i aktualne blokery opisano poniżej.
`WASI` nie oznacza `Firecracker`.
**Etap 0.38: PARTIAL / NO-GO; następny etap wymagany do
zamknięcia: 0.38 security hardening and acceptance,
dopiero potem 0.39.** Wszelkie przyszłe podpisanie bramki
musi mieć evidence konkretnych artefaktów.

## Aktualizacja etapu 0.38 — signed-WASI i polityka operatora (2026-10-13)

**Fakty implementacyjne (main):** izolowany proces Wasmtime/WASIp1;
importy default-deny; deklaratywne limity fuel, pamięci, czasu, wejścia,
wyjścia i współbieżności; podpisany przez operatora manifest Ed25519;
powiązanie tenant/agent z uwierzytelnionym RPC TLS; zdalny worker;
opcjonalne poświadczenia wykonania HMAC-SHA256. Operatorowo dostarczone
pliki modułu, podpisu, manifestu i klucza są obecnie otwierane przez
deskryptor `O_NOFOLLOW` z walidacją regularnego inode, właściciela
effective UID, uprawnień, liczby dowiązań i rozmiaru. Dodano testy
negatywne odrzucenia symlinków, hardlinków, plików czytelnych przez innych
oraz naruszenia limitów wejścia.

**Dowód wcześniejszego HEAD:** `5e3c08af62579cb1660ff72280ef06a141a5548a`
posiada zakończone `Rust CI` i `Supply Chain and Provenance` ze
statusem `success`. Nowe commity muszą mieć własne potwierdzone bramki CI;
poprzednich wyników nie należy przenosić na nowe HEAD.

**Wymagania do technicznego zamknięcia 0.38:** niezależne testy
concurrent multi-tenant, crash/reaping, zasobów hosta i prób eksfiltracji,
walidacja łańcucha publikacji Wasmtime oraz proces rotacji kluczy, kontrola
uprawnień katalogów nadrzędnych i brak TOCTOU przy wykonywaniu binarium
Wasmtime. Izolacja seccomp/cgroups/microVM jest osobnym zakresem 0.39.
Dopóki kryteria 0.38 nie są odbierane niezależnie, status pozostaje
**PARTIAL / NO-GO**, bez przejścia do 0.39.

## Etap 0.38.8–0.38.9 — dowody i pozostające blokery

Szczegółowy [kontrakt odbioru](STAGE_038_8_9_ACCEPTANCE.md).
Wprowadzone kontrole Linux child obejmują grupowe wygaszanie
procesów także po anulowaniu future, `PR_SET_NO_NEW_PRIVS`,
`RLIMIT_CORE=0`, negatywne testy stderr, skryptowany test
mutacji binariów oraz pełen zestaw scenariuszy `wasi_sandbox`
z przypiętym prawdziwym Wasmtime. Wcześniejsze testy
wykazały problem pozostawiania procesów potomnych po
anulowaniu; regresję objęto testem przed poprawką.
Nie stanowi to zakończenia testów crash/soak, izolacji
multi-tenant ani usunięcia TOCTOU przy exec silnika.
**Status 0.38: PARTIAL / NO-GO; 0.39 zablokowany.**

## Dostęp i domknięcie 0.38 — aktualizacja 2026-10-13

- Linux WASIp1: procesowa grupa kill, `PR_SET_NO_NEW_PRIVS`, `RLIMIT_CORE=0`, `PR_SET_PDEATHSIG` dla bezpośredniego potomka.
- Silnik Wasmtime: egzekucja uszczelnionej, zweryfikowanej SHA-256 kopii `memfd` (`F_SEAL_WRITE/GROW/SHRINK/SEAL`), bez ponownego `execve` zmiennej ścieżki. Obowiązują limity właściciela/uprawnień/hardlinków.
- Scenariusze CI: odmowy, anulowanie, root/tenant, równoległa atrybucja 2 tenantów, rzeczywisty Wasmtime i bounded stress z raportem JSON jako artefakt Actions.
- Wzorcowy unit systemd ustanawia host cgroup limits i `KillMode=control-group`; **nie został wdrożony ani przetestowany na docelowej infrastrukturze**.
- Utrzymujemy **0.38 = PARTIAL / PRODUCTION NO-GO** aż do hostowego crash/soak, rzeczywistego wielohostowego staging, rotacji trust anchor i podpisanego niezależnego odbioru. Formalna decyzja zależy od [macierzy 0.38.8–0.38.9](STAGE_038_8_9_ACCEPTANCE.md).
- Nie rozpoczynać 0.39 na podstawie samego CI; odrębna infrastruktura dla Firecracker/KVM i eBPF.

---

## Evidence-controlled update — 2026-10-14 (v4, priority 0.38)

**Source of truth:** only `main`. All evidence below is for the observed immutable
Git commit `361fefece39ed0f34675c8e3d0fd5f8054da41e0` and becomes
**historical** when `main` advances. Any subsequent patch requires a fresh
CI/security audit on its *own* SHA. **Stage 0.38 is NOT signed off; 0.39
implementation is blocked.** No CI or AI self-review may impersonate an
independent auditor or authorized operator.

### Verified GitHub Actions (as of 2026-10-14)

| Control | Actual run | Status | Scope restriction |
|---|---|---|---|
| Rust full CI | [38618072351](https://github.com/mojealterego/ocaf-runtime/actions/runs/38618072351) | SUCCESS | Code/test build, not production deployment |
| CodeQL | [38618072199](https://github.com/mojealterego/ocaf-runtime/actions/runs/38618072199) | SUCCESS | Automated scanner, not independent certification |
| Supply Chain and Provenance | [38618072116](https://github.com/mojealterego/ocaf-runtime/actions/runs/38618072116) | SUCCESS | Ephemeral build provenance and SBOM |
| Linux host preflight | [38618072200](https://github.com/mojealterego/ocaf-runtime/actions/runs/38618072200) | SUCCESS | Ephemeral synthetic host only |
| Two distinct Linux kernels + signed WASI mTLS RPC | [38618072312](https://github.com/mojealterego/ocaf-runtime/actions/runs/38618072312) | SUCCESS | Cross-VM, **one synthetic tenant**, no persistent staging |
| Two tenants + systemd + fault simulation | [38618072122](https://github.com/mojealterego/ocaf-runtime/actions/runs/38618072122) | SUCCESS | Both tenants on **one** disposable VM |
| 100-cycle Wasmtime soak (5 x 20) | [38618072521](https://github.com/mojealterego/ocaf-runtime/actions/runs/38618072521) | SUCCESS | Five CI runner shards, **not** one 100-cycle production-staging soak |

### Mandatory stage 0.38 closure register (NO-GO until EVERY row independently accepted)

| ID | Evidence identifier | Acceptance condition | Current result |
|---|---|---|---|
| A01 | `host_a_systemd` | Stable Linux host A; independent machine/failure domain; signed systemd/cgroup resource report | NOT VERIFIED ON TARGET STAGING |
| A02 | `host_b_systemd` | Same for independently operated host B | NOT VERIFIED ON TARGET STAGING |
| A03 | `multi_host_mtls_rpc` | Real inter-host signed WASI TLS-bound RPC and live peer identity, not loopback | CI PASS ONLY |
| A04 | `two_tenant_isolation` | Both tenant identities, trust stores, logs, receipts and deny cases concurrently **across two hosts** | ONE-HOST CI PASS ONLY |
| A05 | `network_partition_recovery` | Inter-host partition, restore, lease reconciliation, retained denial and no unauthorized replay | NOT VERIFIED ON TWO TARGET HOSTS |
| A06 | `sigkill_replay_fencing` | SIGKILL/host loss, systemd cleanup and no double-commit of an uncertain external effect | PARTIAL DISPOSABLE CI |
| A07 | `publisher_tls_revocation` | Operator-controlled publisher key and TLS CA rotation; old identity rejected on both hosts | ONE-HOST CI PARTIAL |
| A08 | `soak_100_cycles` | One uninterrupted 100-cycle run per host with cgroup peak telemetry and concurrent two-tenant SLO | 5 x 20 CI PASS; STAGING MISSING |
| A09 | `supply_chain_provenance` | Exact final SHA; signed attestation/SBOM; advisory review and reproducible verification | PASS FOR HISTORICAL SHA ONLY |
| A10 | `backup_restore` | Verified restore and durable replay/uncertain-effect accounting in staging | NOT VERIFIED ON TARGET STAGING |
| A11 | `independent_security_audit` | Different real auditor, adversarial report, residual risk, attachment hashes and separate valid signature | **zchiz444 nominated**; independence and identity not verified, audit report and independent signature missing; NO-GO |
| A12 | `operator_signoff` | Authorized real operator reviews identical final SHA, host evidence, SLO, rollback and audit, signs separate approval | **Andrzej Mikulski nominated**; final evidence not approved, operator signature missing; NO-GO |

**Acceptance sources:** [external acceptance procedure](STAGE038_EXTERNAL_ACCEPTANCE_RUNBOOK.md),
[independent audit brief](STAGE038_INDEPENDENT_AUDIT_HANDOFF.md),
[operator receipt](STAGE_038_OPERATOR_SIGNOFF.md),
[fail-closed production verifier](../scripts/stage038_production_gate.py).
The verifier deliberately returns `production_approved: false`; even a valid signed
packet is not an automatic change-control authorization. No private keys, host
credentials, personal identity assertions or confidential host logs belong in this
public repository. If a final SHA changes, restart the evidence/signoff clock.

### Role assignment update — 2026-10-15

The user designated **zchiz444** as independent security auditor (A11)
and **Andrzej Mikulski** as operator (A12). These are **administrative
nominations only**, not signed acceptance or proof of reviewer
independence/authority. The previous assistant-generated provisioning
signatures are not auditor/operator signatures on `acceptance.json`.
A01–A10 persistent cross-host evidence and separate A11/A12 approval
remain mandatory; **0.38 NO-GO, 0.39 blocked**.

### Security engineering review (independent TOOLS, not an independent PERSON)

A local read-only scan with third-party **zizmor v1.30.1** on the
`361fefe` source identified 67 workflow warnings in auditor profile.
Proposed fixes (pin action SHAs and PostgreSQL image digest; reduce global
OIDC; disable persisted checkout credentials) reduced that scanner's
*offline regular-profile* count to zero and passed four local Python regression
tests. This is **not** evidence that those changes have landed on `main`,
that remote CI is green on a patched SHA, or that an independent qualified
person has audited the runtime. The scanner profiles and installation context
are different and cannot be numerically compared as a certification score.

### Next executable sequence — NO paid infrastructure or fictional GO

1. Finish CI supply-chain hardening on `main`; verify actual triggered jobs on
   its final SHA, then freeze code and publish a SHA/digest-only audit manifest.
2. Obtain **already authorized, no-new-charge** two-host *persistent* Linux staging
   with PID1 systemd, separate failure domains, cgroup v2, private mTLS networking,
   independently administered trust stores and recoverable database/sink.
   Do not purchase a server, enable billable marketplace services, or promote
   disposable GitHub runners to a "persistent host".
3. Execute A01–A10 on that *frozen SHA* and store signed private host reports;
   do not run destructive chaos outside an isolated test deployment.
4. An auditor **separate from implementers** validates code, Wasmtime and host
   hardening, signs A11 with an independently trusted Ed25519 identity.
5. The designated operator independently accepts measured risk/SLO/rollback,
   signs A12 with a separate identity. Verify the exact signed evidence packet
   and record a separately authorized production change-control decision.
6. **Only after these gates** set a verified 0.38 COMPLETE entry and unblock
   implementation of 0.39 Firecracker/KVM + Aya/eBPF. Until then: `NO-GO`.

### Traceability of four additional PDFs provided 2026-10-14

These are **four input file versions**, overlapping the previous material,
not four independently verified software deliveries. All source-provided
runtime implementations and performance/safety claims are **proposals** until
tested; a code block or a named specialist swarm is not installation evidence.

| Source file | Technical additions and mapping | Verification gate |
|---|---|---|
| `Notes_261014_134626.pdf` (52 p.) | OmniThermo-Governor 0.82/0.90; OmniProof-STARK 0.85; causal macro-swarmed state 0.86; OmniMarket-L402+ 0.87; OmniSubstrate 0.88; proof-carrying RSI 0.89 | RAPL calibration, formal relation/verifier, reversible aggregation, payment-state conservation, deterministic equivalence and independently checked proofs |
| `Dokument bez nazwy.PDF` (14 p.) | 25 repository candidates consolidated into OmniSeek-Local, OmniFlow-Pieces, OmniDroid-GUI, OmniEval-Lab / OmniFleet-C2, OmniVoice-Duplex / OmniCinema / OmniAgro-KG; see extension 0.95–0.99 | Real modules, sandbox, permissions, bounded DAG, latency and privacy benchmarks; do not copy unverified claims |
| `Architektura OmniContext Agent Framework (OCAF) w ..(1).PDF` (61 p.) | Expanded specialist module corpus including OmniSys, OmniHFT/OmniCAD, OmniLex/OmniSocioSec, OmniFold/OmniBioClinical, OmniPhysics, OmniBroadcast/OmniTutor, OmniCrowd/OmniVet | Trace each domain to 0.56–0.84; require legal/data/operator guards and separate domain-specific evidence |
| `Architektura OmniContext Agent Framework (OCAF) w ...PDF` (58 p.) | Earlier adjacent revision of the same extended architecture; consolidate variant-only additions instead of duplicating entire modules | Diff exact interfaces and requirements before implementation; independent security/benchmark evidence |

**New supplementary delivery IDs** (not a change to 0.39 start gate):

| ID | Scope | Concrete, testable delivery |
|---|---|---|
| 0.95 | OmniSeek-Local + bounded multi-agent DAG | Local adapter, budgeted recursion, explicit approved web/file/tool capabilities, no tenant leak |
| 0.96 | OmniFlow-Pieces | Trigger/action typed interface, idempotency, durable checkpoint, cancellation and reproducible latency baseline |
| 0.97 | OmniDroid-GUI | Android permissioned Accessibility bridge with user-visible approval for sensitive actions; test on real Android ARM64 |
| 0.98 | OmniEval-Lab + OmniFleet-C2 | Replayable trajectory datasets, tamper-evident evals, secure skill provenance/hot reload with automatic rollback |
| 0.99 | OmniVoice-Duplex + OmniCinema + OmniAgro-KG | Opt-in low-latency media and domain packs; real-device and consent tests, rights/provenance and domain model validation |

**Important:** physics/latency claims in supplied notes (e.g. universally
sub-microsecond execution, 85% RAM savings, nanosecond full inference,
negligible proof verification) are hypotheses, not proven OCAF guarantees.
Do not remove systemd/WASI sandbox controls or spend money to chase
experimental targets without an approved study. Foundation stage 0.38
remains the sole current execution priority.

## Aktualizacja 2026-10-15 — uruchomiono development etapu 0.39 na polecenie operatora

Rozpoczęto implementację **0.39.1 (fail-closed Firecracker/KVM/Aya admission i host preflight)** bez aktywowania mikromaszyn i bez produkcyjnego GO. Wykonane moduły: `scripts/stage039_microvm_gate.py`, `scripts/tests/test_stage039_microvm_gate.py`, `.github/workflows/stage039-microvm-preflight.yml` i kontrakt `docs/STAGE039_MICROVM_EBPF.md`. Tester kontroluje prawdziwe wymagania KVM/eBPF hosta i poprawność pinowania czterech artefaktów, ale świadomie nie fabrykuje wyniku uruchomienia VM, attach programu Aya ani izolacji produkcyjnej. Pozostałe funkcje 0.39.2–0.39.6 są otwarte.

**Zachowujemy obowiązujące ograniczenia odbiorowe:** 0.38 = PARTIAL / NO-GO (A01–A12 niewypełnione), 0.39 = STARTED / NO-GO, zatwierdzenie wdrożenia = NO-GO. Wcześniejsze wpisy „0.39 zablokowany” dokumentowały regułę następstwa dla produkcyjnego zaliczenia; ta jawna dyspozycja rozpoczyna prace nad kodem, nie znosi wymagań bezpieczeństwa.

## Decyzja administracyjna i postęp 0.39.2 — 2026-10-15

Na wyraźne polecenie operatora prace backlogowe 0.38 zostały **administracyjnie zamknięte**, a zespół przeszedł do 0.39. Jest to status zarządczy, a nie deklaracja rzeczywistych PASS dla A01–A12 ani zgoda systemu na ominięcie produkcyjnego release gate. Szczegóły w [dyspozycji](STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md). 0.39.2 dostarcza lokalny, trwały rejestr przyjęć `scripts/stage039_broker_ledger.py` z anty-replay, per-workload fencing leader_epoch, kontrolą capacity, TTL, trwałą SQLite WAL i odrzuceniem nieautoryzowanych konfiguracji. Udowodniono 17 testów broker ledger oraz 12 wcześniejszych testów statycznego admission w GitHub Actions. Nadal **nie** uruchomiono microVM ani nie podpięto Aya/eBPF; autoryzacja wdrożeniowa pozostaje NO-GO.


## Operator release authorization and 0.39.3 — 2026-10-15

Owner issued an explicit follow-up command to **remove the administrative production release hold**. Machine-readable decision:
`ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json` sets
`administrative_release_blocked=false`. An explicit, two-flag
`scripts/release_readiness.py --operator-override-policy <file>
--ack-unverified-production-risk` execution authorizes administrative
release continuation without deleting any technical failed/unknown checks.
Default release readiness and external security verification remain
fail-closed; this is **not** assurance certification or proof of deployment.

Stage 0.39.3 now includes a bounded, kernel `SO_PEERCRED`-authenticated
AF_UNIX IPC broker with per-UID and one-tenant/agent policies, framing and
SQLite admission/release integration. Tested with 10 real socket regressions
in addition to 29 earlier tests. Guest KVM launch and Aya enforcement
remain incomplete. Development milestone started, production microVM
isolation **NO-GO**.


## Rozszerzenie v5 — 33 repozytoria i krytyczna ścieżka 0.39 (2026-10-15)

Dostarczone dwa PDF mają łącznie **27 stron** i grupują **24 oraz 9
referencyjnych nazw repozytoriów**. Zostały przepisane jako **10 pakietów
architektonicznych**, bez kopiowania przykładowego kodu jako zatwierdzonej
implementacji. Komplet nazw, odwołanie do etapów, kryteria i zagrożenia:
[ROADMAP_33_REPOSITORY_ADAPTATIONS_2026-10-15.md](ROADMAP_33_REPOSITORY_ADAPTATIONS_2026-10-15.md).

### Macierz nowe wymagania → istniejące etapy

| Pakiet z materiałów | Odpowiednik OCAF | Priorytet / istniejący etap | Stan |
|---|---|---|---|
| P24-01, 5 repo | OmniAutoGen-ActorBus, DampedBabyAgiPlanner | 0.46, 0.50, 0.70–0.71 | REQUIREMENTS ONLY |
| P24-02, 3 repo | OmniBolt-Workstation, OmniFreeApi-Rotator | 0.50–0.56, 0.69, 0.49 | REQUIREMENTS ONLY |
| P24-03, 5 repo | OmniBookToSkill-Compiler, OmniAzureMcp-Bridge | 0.43–0.44, 0.48, 0.62 | REQUIREMENTS ONLY |
| P24-04, 5 repo | OmniPredator-AwdDefenseGrid / Aya telemetry | 0.39, 0.47, 0.63, 0.70 | REQUIREMENTS ONLY; Aya not attached |
| P24-05, 5 repo | OmniVoip-RtpEngine, OmniSeedance-Director, OmniQuantum-Qasm | 0.61, 0.75–0.76 | REQUIREMENTS ONLY |
| P24-06, 1 repo | OmniBrave-ContextShield | 0.48, 0.60, 0.83 | REQUIREMENTS ONLY |
| P09-01, 2 repo | OmniAider-RepoMap, OmniArchGen-Contract | 0.50, 0.55–0.56 | REQUIREMENTS ONLY |
| P09-02, 3 repo | OmniArtemis-AuditMCP | 0.43, 0.63 | REQUIREMENTS ONLY |
| P09-03, 2 repo | OmniAppStudio-Engine | 0.50–0.55, 0.58 | REQUIREMENTS ONLY |
| P09-04, 2 repo | OmniVertical-Matcher | 0.67, 0.69, 0.71 | REQUIREMENTS ONLY |

### 0.39 — kolejność wdrożenia od aktualnego stanu

| WBS | Zadanie / artefakt | Twarde wymaganie i test negatywny | Stan |
|---|---|---|---|
| 0.39.1 | `stage039_microvm_gate.py`: typed manifest, pinned digests, host preflight | Odrzuca fałszywe /dev/kvm, dane tenant, mutable paths; nigdy nie raportuje boot | **PASS 12 tests**, CI host NO-GO |
| 0.39.2 | `stage039_broker_ledger.py`: SQLite WAL, fencing leader_epoch, anti-replay | Odrzuć rollback epoch, concurrent double-claim, błędny owner, DB perms | **PASS 17 tests** |
| 0.39.3 | `stage039_broker_ipc.py`: AF_UNIX, kernel SO_PEERCRED | Zła UID i tenant denied; limit ramki, bez implicit exec | **PASS 10 tests** |
| 0.39.4 | **KVM kernel ioctl probe** i konfiguracja jails/VM | Real KVM API_VERSION=12, CREATE_VM succeeded; char device nie wystarcza | **REAL KVM API 12 and CREATE_VM PASS as root, run 38704086975** |
| 0.39.5 | Pin Firecracker **i jailer tej samej wersji**, signed/provenance assets; generator staging chroot | No stale artifact path, immutable verified staged copy, no symlink / TOCTOU | **Static chroot config + 6 tests PASS; live jailer / TOCTOU runtime proof OPEN** |
| 0.39.6 | Realny boot Firecracker za jailer na host Linux/KVM | Kernel/rootfs readonly; VM Running + kernel serial + cgroup proof + cleanup | **LIVE EXECUTOR WRITTEN; 6 contract tests PASS; host KVM NO-GO** |
| 0.39.7 | Program Rust `aya-ebpf` cgroup_skb/egress + userspace Aya loader | Realne BPF verifier load, program ID, tenant A block/B unaffected, recovery after detach | **REAL kernel Aya attach and A/B negative+recovery PASS, run 38704086975** |
| 0.39.8 | Spójny broker lifecycle: claim → jailer → eBPF link → VM → receipt → teardown | Block until eBPF enforcement; kill/replay/restore cleanup, no orphan VM | PLANNED |
| 0.39.9 | Dwa odrębne trwałe hosty KVM, tenant A/B, chaos i bounded 100-cycle soak na każdym | Rzeczywiste UUID hosta, failure-domain, restart, revoked-key rejection | **NO-GO / hosts unavailable** |
| 0.39.10 | Safety & supply chain: SBOM, vulnerability/TOCTOU tests, independent scope review | Reproduce SHA manifest-to-running inode, guest escape and egress attempts | **NO-GO** |
| 0.39.11 | Produkcyjny operator acceptance dla 0.39 | Oddzielne zatwierdzenie, report ze środowiska i podpis; nie mylić z administracyjnym unblock | **NO-GO** |

**Krytyczna zależność:** nie ma kontrolowanego hosta z `/dev/kvm`
i `systemd` PID1 / cgroup v2 / BTF / bpffs udostępnionego narzędziom.
Ostatni rzeczywisty GitHub Actions Linux preflight dał `NO_GO`.
Brak sprzętowych capability **nie zostanie naprawiony** przez zmiany
w manifestach, dodanie zależności Cargo lub sfałszowanie wyniku CI.
Należy nieodpłatnie użyć dostępnego już, zatwierdzonego hosta KVM
albo osobno uzyskać autoryzację finansowania; nie tworzymy
niezamówionych płatnych VM.

### Prawidłowo interpretowane KPI z PDF

- Deklaracje „<1ms API routing”, „<2ms RTP”, „zero downtime”,
  „gwarantowana skończoność” oraz „usuwanie prompt injection w
  mikrosekundach” są **hipotezami do benchmarków** z warm-up, p50/p95/p99,
  replayable workload i fail conditions — nie SLA w roadmapie.
- Wykładnicze tłumienie punktacji zadania nie ogranicza całkowitej
  liczby zadań; trzeba twardych limitów.
- Nazwanie ciągu znaków 'eBPF patch' nie wykonuje BPF attach; log
  musi zawierać zweryfikowany link i negatywny test pakietu.
- Stateless stopień dopasowania ATS nie zastępuje referencji do
  potwierdzonych faktów CV; DOM string-replace nie gwarantuje
  neutralizacji injected instructions.
- Finansowy fallback API nie może uruchamiać L402 bez jawnego,
  odrębnego zatwierdzenia użytkownika i limitu kosztów.

**Zarządzanie wydaniem:** blokada *administracyjna* 0.38 zdjęta
na dyspozycję operatora. Techniczne testy i audyt A01–A12
nadal niewystarczające do poświadczenia gotowości. `0.39`
jest wykonywany wyłącznie na `main`; **nie twierdzimy,
że VM Firecracker albo Aya działają produkcyjnie.**


### 2026-10-15 — dowody inkrementacji 0.39.4/0.39.5/0.39.7

`scripts/stage039_kvm_ioctl.py` używa **rzeczywistego** `KVM_GET_API_VERSION`
oraz `KVM_CREATE_VM` na char device 10:232, zamykając deskryptor.
Na runnerze GitHub Actions /dev/kvm pozostaje nieudostępnione:
**KVM_CREATE_VM = NO-GO**; nie podmieniamy tego wyniku na PASS.

`scripts/stage039_firecracker_spec.py` konstruuje konfigurację
Firecracker z `network-interfaces: []`, read-only rootfs oraz pinowanymi
przez SHA-256 kopiami guest kernel/rootfs w chroot jailer.
**To specyfikacja statyczna, a nie uruchomiony Firecracker.**

`stage039/aya-ebpf-egress/src/main.rs` zawiera
`#[cgroup_skb(egress)]` i deny-all return 0; 
`stage039/aya-guard/src/main.rs` realizuje prawdziwe wywołania Aya
`Ebpf::load -> CgroupSkb::load -> attach -> detach` dla jednego
jawnie wybranego service cgroup, z kontrolą SHA-256 ELF. Sam kod
programu eBPF nie jest jeszcze skompilowany do ELF dla docelowego
kernela ani podpięty. Nawet przyszłe attach nie wystarczy jako dowód
odcięcia ruchu bez negatywnego testu pakietów.

[GitHub Actions run 38663469207](https://github.com/mojealterego/ocaf-runtime/actions/runs/38663469207):
65 testów Pythona (12+17+10+12+8+6) i 3 testy Rust Aya
**PASS**. Host preflight i actual KVM ioctl **NO-GO**, zgodnie z realną
infrastrukturą CI. Brak sztucznego hosta, audytu czy raportu GO.


## 2026-10-15 — sprzętowe testy Firecracker / Aya: wykonawcza automatyzacja bez fikcyjnego PASS

Dostarczono skrypty rzeczywistego testu `scripts/stage039_live_firecracker.py`
i `scripts/stage039_live_egress.py` oraz manualny workflow
`.github/workflows/stage039-live-kvm-aya.yml`, który może uruchomić
je jedynie na istniejącym self-hosted Linux x64 z etykietą
`ocaf-kvm-ebpf`. Są testy realnego API KVM/Firecracker, boot console,
PID w dedykowanym cgroup i dwóch osobnych procesów network loopback
(A blokowany przez Aya, B nieblokowany, A ponownie dostępny po detach).
CI wykonało 16 nowych unit/regression tests i 68 wcześniejszych,
łącznie **84 PASS** ([run 38665774854](https://github.com/mojealterego/ocaf-runtime/actions/runs/38665774854)).
**Sprzętowy Firecracker/KVM oraz kernel Aya attach nie były uruchomione**:
brak dostępnego hosta z uprawnieniami KVM i eBPF; żadnych dodatkowych
płatnych VM nie stworzono. Plan akceptacji produkcyjnej nie zmienia
warunków GO/NO-GO.


### Aktualizacja 0.39.7 — Aya eBPF ELF zweryfikowany i gotowy do użycia na hoście

`stage039/aya-ebpf-egress` kompiluje się do natywnego
`bpfel-unknown-none` (test `readelf` PASS, SHA-256
`992cd7a365067f52bce5afe5ca9758dad0e15ad9291b71991914040faaa55c95`,
992 B). Pobrano i zweryfikowano `bpf-linker v0.11.0` po
SHA-256; odrębny eBPF `Cargo.lock` jest w `main` i kolejny
CI buduje z `--locked`. Wynik
[GitHub Actions 38666442623](https://github.com/mojealterego/ocaf-runtime/actions/runs/38666442623)
= **SUCCESS na cross-compile**, nie sukces sprzętowego
`BPF_PROG_LOAD`/attach, negatywnego testu ruchu lub boot Firecracker.
Self-hosted workflow `.github/workflows/stage039-live-kvm-aya.yml`
pozostaje ścieżką rzeczywistych testów KVM/BPF.


### 0.39 — host KVM: CircleCI Android machine (kandydat, 2026-10-15)

Dla braku /dev/kvm w GitHub i Sprite wybrano **CircleCI Android
machine image `android:current` na `medium.gen2`**, którego
oficjalna dokumentacja deklaruje wsparcie nested virtualization.
W `.circleci/config.yml` dodano `stage039-hardware-nested-kvm`:
realne `KVM_GET_API_VERSION/KVM_CREATE_VM`, test wymagania cgroup
v2+BTF+bpffs, budowanie sprawdzonego loadera Aya i test A/B
odłączenia egress na rzeczywistym jądrze. Artefakty: 
`ocaf039-circleci-real-kvm-aya`; fail-closed przy braku właściwego hosta.
**Uruchomienie VM nie zostało potwierdzone:** GitHub nie pokazał
statusu CircleCI dla commitu `d3a406d651b3bbcb0fbb5d21260c315f1b6b5d0e`; brak
bezpośrednio autoryzowanego dostępu do konta/projektu CircleCI.
Domyślna flaga `run_stage039_kvm=false` chroni przed przypadkowym
przyszłym zużyciem kredytów; jednorazowo ustawić ją na `true` w
ręcznym wyzwoleniu w projekcie po weryfikacji darmowego limitu.
**Host ma charakter CI ephemeral, a nie trwałego serwera produkcyjnego.**


## 2026-10-15 — rzeczywiste KVM i Aya w GitHub Actions Linux

Dowód: [STAGE039_REAL_KVM_AYA_PROOF_2026-10-15.md](STAGE039_REAL_KVM_AYA_PROOF_2026-10-15.md)
i [workflow 38704086975](https://github.com/mojealterego/ocaf-runtime/actions/runs/38704086975).
Uprzywilejowany runner Ubuntu 24.04 pozwolił wywołać **rzeczywisty**
`KVM_CREATE_VM` z API_VERSION 12 i **rzeczywisty**
`CgroupSkb::attach` Aya, program ID 93, do cgroup A.
Próby: A/B dostępne przed attach, A zablokowane, B nadal dostępne,
A odzyskane po detach — **PASS**. Wszystkie raporty ograniczają
zakres do tego tymczasowego hosta; runtime Firecracker guest
i długotrwały host nadal wymagają osobnych potwierdzeń.


## Postęp 0.39 — jednoczesny boot Firecracker/KVM i negatywny test Aya (2026-10-15)

Działający `jailer` + VM na realnym KVM i kernelowy `CgroupSkb::attach`
zostały połączone w pojedynczej próbie na tym samym efemerycznym
hoście. VM host cgroup A: połączenie TCP hostowego próbnika zablokowane
po attach, peer B nadal działa; A odzyskuje po detach; zweryfikowano
tożsamość cgroup, inode, proces VM, podpisane/digest-pinned artefakty
oraz flagi `production_approved=false`.
[CI 38711385989](https://github.com/mojealterego/ocaf-runtime/actions/runs/38711385989),
[raport](STAGE039_COMBINED_VM_CGROUP_EGRESS_PROOF_2026-10-15.md).
**To nie jest test sieci gościa** (VM ma `network-interfaces=[]`),
a brak zewnętrznego dwu-hostowego i niezależnego odbioru nadal blokuje GO.

**Status roadmapy: 0.38 COMPLETE / CLOSED_BY_OPERATOR, 0.39 IN PROGRESS.**
Kryteria bezpieczeństwa 0.38 A01–A12 wciąż `INCOMPLETE`,
produkcja `NO-GO`. Zapisy starsze niż ta sekcja odnoszą się
do historycznych commitów i ich ograniczonych wyników.


## Wynik 0.39.8–0.39.9 — rzeczywisty resource envelope + SIGKILL (2026-10-15)

Na pojedynczym, efemerycznym Ubuntu 24.04 GitHub Actions uruchomiono
prawdziwą jailed VM Firecracker 1.17.0 na KVM, podpięto Aya do hostowego
cgroup procesu, wymuszono `memory.max`, `memory.swap.max=0`,
`memory.oom.group=1`, `pids.max=96`, `cpu.max=200000 100000`,
odczytano peak memory/pids i liczniki OOM, następnie wykonano
**SIGKILL rzeczywistego procesu VMM** i sprawdzono `cgroup.events`
`populated=0` obejmujące cgroup potomne. [Weryfikowalny run
38713156606](https://github.com/mojealterego/ocaf-runtime/actions/runs/38713156606),
[rejestr dowodowy](STAGE039_VM_RESOURCE_FAULT_PROOF_2026-10-15.md).
Kod i automatyczne testy negatywne w
`scripts/stage039_vm_resources.py`, `scripts/stage039_vm_crash.py`,
`scripts/tests/test_stage039_vm_resources.py` i
`scripts/tests/test_stage039_vm_crash.py`.

Granica: crash dotyczył procesu VMM na jednym runnerze, nie awarii
fizycznego hosta, failoveru dwuhostowego ani niezależnego staging.
Próbnik Aya wykonywał hostowy TCP loopback, nie sieć gościa.
**Roadmapa: 0.38 COMPLETE (operator project closure), 0.39 IN PROGRESS,
produkcja NO-GO do odbioru bezpieczeństwa i infrastruktury.**


### 0.39 — rzeczywiste testy dwóch tenantów, ruchu guest UDP i oddzielnych hostów CI (2026-10-15)

Zrealizowano **PASS dwóch równoczesnych jailed VM Firecracker/KVM**
o różnych tenantach i inode cgroup; VM B pozostaje sprawna po SIGKILL VM A.
W odrębnym rzeczywistym teście **dwa goście Firecracker inicjują UDP**
na oddzielnych TAP; `nftables` blokuje tylko A, B działa, po zdjęciu
blokady A odzyskuje ruch. [CI 38715926542](https://github.com/mojealterego/ocaf-runtime/actions/runs/38715926542).

W dwóch oddzielnych jobach GitHub-hosted Linux uzyskano różne kernel
`boot_id`, niezależne `KVM_CREATE_VM`, boot jailed Firecracker,
SIGKILL VM i opróżnienie cgroup na **obu hostach**; osobny weryfikator
porównał dowody i zakończył się PASS:
[CI 38720667372](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372).
Szczegółowy [raport](STAGE039_TWO_TENANTS_GUEST_NETWORK_INDEPENDENT_HOSTS_2026-10-15.md).

**Rozgraniczenie:** test UDP korzystał z gości **bez jailer** i blokady
`nftables`, nie Aya guest egress. Dwa hosty to efemeryczne VM CI,
nie produkcyjna infrastruktura HA. Nie dowiedziono rzeczywistej partycji
między hostami, przełączenia usługi, host power loss lub podpisanego
niezależnego audytu. **0.38 COMPLETE (decyzja operatora);
0.39 IN PROGRESS; produkcja NO-GO.**


## 0.39.12–0.39.14 — verified JAILED guest TAP TC BPF and cross-host cold restart (2026-10-15)

`0.39.12: PASS` — actual guest-originated UDP with **both** Firecracker machines jailed; different tenant UID/VM/cgroup and host `nftables` negative/restore + A crash/B isolation. [38721552535](https://github.com/mojealterego/ocaf-runtime/actions/runs/38721552535).

`0.39.13: PASS` — artifact-mediated checkpoint nonce preserved after SIGKILL and recovered by a **new** jailed guest on a **different** GitHub Actions Linux kernel; separate host `boot_id`, same nonce in guest UDP, both jobs real, [38722139701](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722139701). Cold restart only; no online HA or durable host test.

`0.39.14: PASS` — actual guest packet drops by **kernel TC eBPF** loaded on the TAP of a jailed tenant A while jailed tenant B packets still arrive; remove TC and prove A recovers, [38722682521](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722682521). This is NOT Aya CgroupSkb filtering guest traffic.

**Open release gates:** independent durable production host pair + authenticated interhost RPC/partition/failover and recovery, per-tenant network namespaces and soak tests, independent signed audit and real operator signature. [Full source-backed evidence ledger](STAGE039_NETWORK_EBPF_CROSS_HOST_EVIDENCE_2026-10-15.md). **Milestone 0.38 COMPLETE project-status, 0.39 still IN PROGRESS; prod NO-GO.**

## 0.39.15–0.39.17 — TLS1.3 mTLS + network partition lab and signed acceptance integrity (2026-10-15)

- `0.39.15–0.39.16: PASS, LAB ONLY`: real Linux netns/veth TLS1.3 mutual-cert client/server, privileged kernel blackhole route, rogue client cert rejection, SQLite durable leader fencing and restart recovery, [workflow 38724640542](https://github.com/mojealterego/ocaf-runtime/actions/runs/38724640542). **Not** two-host live RPC/HA.
- `0.39.17: PASS, CRYPTO UNIT TEST ONLY`: two distinct test-generated Ed25519 key pairs, hashed evidence artifacts and strict evidence dossier validation; missing/same-key/tampered signatures are rejected, [workflow 38725162927](https://github.com/mojealterego/ocaf-runtime/actions/runs/38725162927). **Not** an independent signed external audit or human operator signoff.
- Scope table and exact remaining release gates: [stage 0.39 consolidated ledger](STAGE039_ACTUAL_BOUNDARY_CLOSURE_2026-10-15.md). **0.39 IN PROGRESS / NO-GO** until durable actual live two-host HA and independent external signed formal production acceptance.

## 0.41.2 — rzeczywisty restore na drugim Linux kernel (2026-10-15)

`PASS`: [workflow 38734668015](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015), osobne joby Ubuntu 24.04 i osobne `/proc/sys/kernel/random/boot_id`. W źródłowym PostgreSQL 16 wykonano `pg_dump --format=custom` po wpisaniu 25 rekordów i zatrzymano proces DB; do drugiego kernela przeniesiono kontrolowany SHA-256/manifest GitHub Actions i faktycznie przywrócono 25 rekordów na nowej instancji PostgreSQL. Test negatywny **na rzeczywiście pobranym pliku** wykrył modyfikację bajtu i ponownie wykorzystany kernel ID; końcowy rekordowy SHA-256 był identyczny. `production_approved=false`, `stage_041_complete=false`, ponieważ nie testowano całego host power loss, zaszyfrowanego off-site backupu ani długotrwałego SLO. [Pełna dokumentacja](STAGE041_CROSS_KERNEL_RESTORE_2026-10-15.md).

## 0.41.3 — rzeczywisty PostgreSQL PITR do punktu WAL (2026-10-15)

**PASS**: [GitHub Actions 38737008767](https://github.com/mojealterego/ocaf-runtime/actions/runs/38737008767). `pg_basebackup` po 25 rekordach, `pg_current_wal_lsn()` po 35 rekordach, kolejne 5 **zatwierdzonych** transakcji poza punktem odtworzenia, archiwizacja rzeczywistego segmentu WAL, IMMEDIATE crash primary, nowy postmaster z `recovery.signal` i `recovery_target_lsn`. Przywrócono dokładnie **35/35** rekordów z identycznym SHA-256, a zapisane później rekordy **36–40 były nieobecne**. To jeden kernel, nie fizyczna awaria hosta ani zewnętrzne archiwum WAL. [Pełny raport](STAGE041_REAL_WAL_PITR_2026-10-15.md).

## 0.41.4 — rzeczywisty physical WAL PITR na innym kernelu Linux (2026-10-15)

**PASS**: [GitHub Actions 38738198412](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412). Oddzielny Linux Linux kernel host A wykonał `pg_basebackup`, archiwizację WAL, zapis 35 rekordów do LSN i dalszych pięciu transakcji, IMMEDIATE source crash. Oddzielny Linux kernel B pobrał SHA-pinned fizyczną kopię i WAL, odrzucił uszkodzone archiwum i ponownie użyty source kernel ID, odtworzył **rzeczywisty PostgreSQL do docelowego LSN**; 35/35 rekordów, pięć późniejszych nieobecnych, identyczny SHA-256. **Nie** jest to całkowita awaria fizycznego hosta ani produkcyjny off-site DR. [Pełna dokumentacja](STAGE041_TWO_KERNEL_PHYSICAL_WAL_PITR_2026-10-15.md).

## 0.39.18 / 0.40.1 — broker bound to jailed guest kernel and three host-local PG HA labs

**PASS:** [Real broker→two jailed VM kernel evidence 38739812121](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812121): kernel SO_PEERCRED broker reservations **before VM launch**, SHA-pinned guest digest/tenant ID/UID/cgroup/SIGKILL correlated, both leases released. **Signed WASI inside guest still OPEN**. [Three real distinct Linux runner kernels PG failover 38740089797](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740089797): each host independently uses 3 real PG postmasters, WAL replication and standby promotion/reparent, all 4 jobs green, but no inter-host PG streaming. [Full ledger](STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md).

## 0.40.2 — PostgreSQL WAL reliability under real 8-client pgbench load

**PASS**: [real Ubuntu24 PostgreSQL16 stress+crash CI run 38740936947](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936947). Actual pgbench (8 clients, 2 threads, 30 sec) generates committed WAL while B/C stream; both replicas catch up to matching primary account/branch/teller balances. After stress A receives IMMEDIATE stop, B promotes, C reparent and WAL streaming resumes. `pg_basebackup --checkpoint=fast --wal-method=stream` resolves earlier stressed-backup timeout. Local **single-kernel** topology, not physical multi-host HA or weeklong soak. [Primary evidence](STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md).
