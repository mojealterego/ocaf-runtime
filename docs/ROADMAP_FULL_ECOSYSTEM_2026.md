# OCAF FULL ECOSYSTEM ROADMAP — 26 PDF + 4 supplementary file versions / v3

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
| 0.38 | Execution | Restricted Wasmtime WASIp1 adapter (see [implementation](WASI_EXECUTION_038.md) and [acceptance contract](PRIORITY_038_EXECUTION_CONTRACT.md)) | **PARTIALLY IMPLEMENTED / P0 / NO-GO** | Operator-pinned Wasmtime CLI, Ed25519 manifest, tenant/agent binding, TLS worker and HMAC receipt implemented; real WASIp1 tested in CI. Independent crash/soak, host threat review and final security acceptance pending |
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
