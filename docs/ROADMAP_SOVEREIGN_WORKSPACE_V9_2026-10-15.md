# OCAF ROADMAP v9 — Sovereign Cargo Workspace / 8 kroków badań
**2026-10-15, tylko gałąź `main`, wszystkie poniższe punkty = PLANNED.**

**Nowe pozycje:** 40 etapów głównych **1.55–1.94** oraz 16 podetapów **0.39.26–0.39.29, 0.40.11–0.40.14, 0.41.11–0.41.14, 0.42.1–0.42.4** = 56 nowych pozycji. Po rozszerzeniu ciąg numeracji głównej 0.00–1.94 obejmuje **195 identyfikatorów**; nie jest to liczba wdrożonych funkcji. Starsze podetapy liczyć oddzielnie.

## Dokumentacja źródłowa i zakres

Przesłany **`Dokument bez nazwy(3).PDF`**, 13 stron treści (+ pusta ostatnia strona w pliku PDF). Strony 1–2: plan ośmiu etapów badawczych; strony 2–5: hipotezy, porównania, ograniczenia i wyniki podane w dokumencie; strony 5–13: propozycja `ocaf-sovereign-workspace` i demonstracyjny `ocaf_sovereign_master` w Rust. Fragmenty Rust dotyczące OpenTelemetry, guest eBPF, RQGM, Titans, L402 i kosztów **nie oznaczają realnego uruchomienia tych adapterów**. Kod przykładowy emituje logi, lecz nie łączy się sam w sobie z OTel collector, nie ładuje sondy Aya do jądra gościa i nie potwierdza rozliczenia faktury w Lightning. Twierdzenia o 7–10 µs, EDP 0.000872x i innych wynikach z PDF są **tezami źródła**, a nie pomiarami OCAF.

Nie zastępujemy już działającego `Cargo.toml` nowym `[package]` z PDF. Etap 1.90 przewiduje **stopniową zgodną wstecznie migrację do workspace**, a nie utratę dotychczasowego runtime. P0 nadal obejmuje rzeczywistą integrację broker→WASI wewnątrz Firecracker i replikację PostgreSQL TCP między niezależnymi hostami.

## Osiem metodologicznych bloków weryfikacji

| Blok | Etapy | Badany komponent | Strony PDF |
|---|---|---|---|
| 1 | `1.55–1.59` | Transport Zero-Copy i konwergencja stanu | 1, 2 |
| 2 | `1.60–1.64` | Obserwowalność OTel w środowisku async | 1, 2, 3 |
| 3 | `1.65–1.69` | CoALA, UALA i pamięć Titans | 1, 3, 9, 10 |
| 4 | `1.70–1.74` | RQGM, koewolucja egzaminatora i typowane AST | 1, 3, 4, 9, 10 |
| 5 | `1.75–1.79` | Loihi 2 SDNN i ML-KEM | 1, 4 |
| 6 | `1.80–1.84` | Aya eBPF w gościu Firecracker i detekcja syscall | 1, 2, 4, 5, 7, 8 |
| 7 | `1.85–1.89` | L402, cost exhaustion i disposable apps | 2, 5, 11, 12 |
| 8 | `1.90–1.94` | Cargo Workspace i unifikacja rojów | 2, 5, 6, 12, 13 |

## Nowe etapy główne: 40

| ID | Owner/Priorytet | Konkretny rezultat | Pozytywny test akceptacji | Test negatywny | Zależności | Strony PDF | Status |
|---|---|---|---|---|---|---|---|
| **1.55** | transport / P1 | **Transport benchmark SHM/TCP:** Matched real Zenoh SHM and TCP payload corpus | Raw p50/p95/p99 latency, CPU and memory for 64B, 4KiB, 1MiB messages | Reject synthetic clock and unequal hardware | 1.15, 1.20 | 1, 2 | **PLANNED** |
| **1.56** | transport / P1 | **SHM tenancy and quota:** Bound shared allocations to UID and tenant capabilities | 8 publishers stress allocation reclaim, crash and no cross tenant map | Reject foreign handle and unbounded pool growth | 1.15, 1.16, 1.55 | 1, 2 | **PLANNED** |
| **1.57** | transport / P1 | **Dual CRDT convergence corpus:** Bridge Automerge JSON with Yrs text using typed adapters | 10000 randomized concurrent operations converge across restarted replicas | Reject stale tombstone revival and schema coercion | 1.18, 1.19 | 1, 2 | **PLANNED** |
| **1.58** | transport / P1 | **Edge Zenoh interoperability:** Pin zenoh-pico/no_std compatibility with cloud peers | Real device or labeled emulator demonstrates suspend reconnect and queued delivery | Reject unsupported MCU claimed as measured hardware | 1.17, 1.55 | 1, 2 | **PLANNED** |
| **1.59** | transport / P1 | **Transport and CRDT release matrix:** Publish reproducible versioned interoperability cases | x86_64 and optional arm64 run hash-equal state after network faults | Reject incompatible wire frame and malformed CRDT update | 1.55, 1.56, 1.57, 1.58 | 1, 2 | **PLANNED** |
| **1.60** | observability / P1 | **OTel resource precedence:** Make service name and deployment environment operator configurable | Real collector records OTEL_SERVICE_NAME and version from process env | Reject hard-coded resource override | 1.20, 1.21 | 1, 2, 3 | **PLANNED** |
| **1.61** | observability / P1 | **Asynchronous span lineage:** Carry traceparent across Tokio tasks broker and guest boundaries | 1000 fan-out tasks preserve trace_id and parent span identity | Reject orphan span or swapped tenant trace context | 1.20, 0.39 | 1, 2, 3 | **PLANNED** |
| **1.62** | observability / P1 | **Batch exporter bounded outage:** Constrain OTel batch queue and collector retry | 10min collector failure yields bounded RSS, counted dropped spans and live workload | Reject unbounded memory or hidden trace loss | 1.20, 1.21 | 1, 2, 3 | **PLANNED** |
| **1.63** | observability / P1 | **Telemetry orderly shutdown:** Flush Tracer then Meter then Logger on process exit | Collector receives final span, metric and log within bounded deadline | Reject missing flush receipt and duplicated shutdown | 1.20, 1.21 | 1, 2, 3 | **PLANNED** |
| **1.64** | observability / P1 | **Telemetry controlled overhead:** Compare traces at head sampling ratios zero point one and one | Measured throughput and latency deltas vs matched no telemetry baseline | Reject unproven zero cost claim | 1.60, 1.61, 1.62, 1.63 | 1, 2, 3 | **PLANNED** |
| **1.65** | memory / P1 | **CoALA typed memory API:** Unify four separate memory stores with per tenant capabilities | Restart and replay an authorized agent decision on identical memory snapshot | Reject memory type confusion and foreign tenant read | 1.22, 1.23, 1.24, 1.25 | 1, 3, 9, 10 | **PLANNED** |
| **1.66** | memory / P1 | **UALA decision trace provenance:** Bind measured uncertainty and Titans gradients to action receipt | External action logs calibrated uncertainty and selected memory revision | Reject NaN gradient and missing confidence | 1.26, 1.27, 1.65 | 1, 3, 9, 10 | **PLANNED** |
| **1.67** | memory / P1 | **Titans stability evaluation:** Bound momentum, gradient, forgetting and checkpoint format | Seeded replay tracks heldout loss and numerical stability | Reject nonfinite update, dimension mismatch and catastrophic regression | 1.27, 1.66 | 1, 3, 9, 10 | **PLANNED** |
| **1.68** | memory / P1 | **Cognitive cold restore:** Reconstruct CoALA and optional Titans from authorized checkpoint | Fresh process reproduces approved memory and provenance graph | Reject recovery of revoked personal records | 1.28, 1.65, 1.67 | 1, 3, 9, 10 | **PLANNED** |
| **1.69** | memory / P0 | **Cognitive tool safety benchmark:** Evaluate calibrated UALA gates against actual sandbox actions | Publish FPR/FNR by risk threshold and baseline | Reject high uncertainty destructive action irrespective of reward | 1.26, 1.66, 1.68 | 1, 3, 9, 10 | **PLANNED** |
| **1.70** | evaluation / P1 | **RQGM immutable examiner epoch:** Pin judge and benchmark corpus digest for whole evaluation epoch | Independent workers attest one immutable examiner version | Reject changed judge during candidate batch | 1.29, 1.30 | 1, 3, 4, 9, 10 | **PLANNED** |
| **1.71** | evaluation / P1 | **RQGM score ledger preservation:** Retain immutable raw evidence and selectively expire superseded scores | Replay judge transition preserves unrelated source evidence | Reject deletion of external judge observations | 1.31, 1.70 | 1, 3, 4, 9, 10 | **PLANNED** |
| **1.72** | evaluation / P1 | **Type sitter admission of mutations:** Parse Rust mutation with pinned tree sitter schema and layer rules | Real compiled candidate passes dependency DAG and AST gate | Reject indirect circular graph and reexport bypass | 1.33, 1.34, 1.35 | 1, 3, 4, 9, 10 | **PLANNED** |
| **1.73** | evaluation / P1 | **Examiner adversarial evaluation:** Generate protected counterexamples for grading reward hacking | Heldout mutation corpus reports detection and human baseline | Reject contaminated evaluation set and prompt injected candidate | 1.32, 1.70, 1.72 | 1, 3, 4, 9, 10 | **PLANNED** |
| **1.74** | evaluation / P1 | **Mutation deployment rollback:** Stage accepted candidate behind sandboxed regression and rollback | Rebuild full regression with signed commit provenance before promotion | Reject security regression and self approved risky patch | 1.71, 1.72, 1.73 | 1, 3, 4, 9, 10 | **PLANNED** |
| **1.75** | crypto / P1 | **SDNN arithmetic property corpus:** Implement saturating signed spikes and accumulator boundary tests | 100000 seeded CPU traces yield reproducible exact deltas | Reject overflow, zero threshold and invalid channel index | 1.36, 1.37 | 1, 4 | **PLANNED** |
| **1.76** | crypto / P1 | **Loihi optional hardware backend:** Negotiate SDK capabilities and fallback to CPU when unavailable | Device inventory records real Loihi chip for hardware comparisons | Reject emulator results labeled real hardware | 1.38, 1.75 | 1, 4 | **PLANNED** |
| **1.77** | crypto / P1 | **Energy delay reproducibility:** Run same model and I/O boundary across available Loihi, Jetson, CPU | Export measured joules, frame rate and latency with raw repeats | Reject document headline ratio as OCAF measurement | 1.39, 1.76 | 1, 4 | **PLANNED** |
| **1.78** | crypto / P1 | **ML KEM provenance and vectors:** Pin licensed mlkem-native release and supported CPU variants | Official KAT and negative ciphertext decapsulation run against actual backend | Reject changed ciphertext and unsupported accelerator | 1.40 | 1, 4 | **PLANNED** |
| **1.79** | crypto / P1 | **Authenticated hybrid PQ handshake:** Bind classical certificate and ML KEM key to same transport transcript | Two endpoints complete real handshake and key rotation | Reject downgrade replay and identity mismatch | 1.41, 1.78 | 1, 4 | **PLANNED** |
| **1.80** | security / P0 | **In guest Aya BPF verifier gate:** Load actual Aya eBPF ELF into independent Firecracker guest kernel | Guest BPF ID and verified kernel image digest in KVM evidence | Reject host only TC classifier as guest result | 1.42, 0.39.22 | 1, 2, 4, 5, 7, 8 | **PLANNED** |
| **1.81** | security / P1 | **One second syscall window:** Track guest openat in bounded per PID monotonic windows | Actual guest syscall event counts correlate with guest PID and tenant | Reject PID reuse clock shift and unbounded BPF maps | 1.42, 1.43, 1.80 | 1, 2, 4, 5, 7, 8 | **PLANNED** |
| **1.82** | security / P1 | **Kernel anomaly false positive audit:** Calibrate entropy and ransomware suffix features with real corpus | Export FNR and FPR before any process quarantine | Reject single suspicious filename as automatic killing proof | 1.44, 1.81 | 1, 2, 4, 5, 7, 8 | **PLANNED** |
| **1.83** | security / P1 | **Guest host event correlation:** Bind guest syscall to host packet via signed VM and workload nonce | Real guest and host traces match workload and boot ID | Reject forged nonce or foreign tenant event | 1.43, 1.45, 1.80 | 1, 2, 4, 5, 7, 8 | **PLANNED** |
| **1.84** | security / P0 | **Guest BPF fail safe controls:** Lease authorize guest quarantine with revocation and map cleanup | Probe restart and VM crash preserve deny safety with reversible enforcement | Reject missing guest probe falsely reporting security PASS | 1.80, 1.82, 1.83 | 1, 2, 4, 5, 7, 8 | **PLANNED** |
| **1.85** | governance / P0 | **L402 settlement provenance:** Distinguish preimage verification from actual invoice settlement state | Authorized LN node returns paid invoice reference and amount | Reject self declared payment by HMAC alone | 1.49, 1.53 | 2, 5, 11, 12 | **PLANNED** |
| **1.86** | governance / P1 | **L402 caveat semantics:** Enforce tenant, scope, time, calls and spend on Macaroon attenuation | Interoperability test confirms bounded nested caveats across restart | Reject unknown caveat and privilege escalation | 1.50, 1.51, 1.54 | 2, 5, 11, 12 | **PLANNED** |
| **1.87** | governance / P0 | **Atomic token budget escrow:** Reserve, commit and refund per tenant tool token budget | Concurrent 50 agent calls obey caps after simulated leader failover | Reject double debit overflow and replayed payment | 1.47, 1.54 | 2, 5, 11, 12 | **PLANNED** |
| **1.88** | governance / P1 | **Disposable app lifecycle:** Track application creator, TTL, endpoint and secret revocation | Real demo app expiry revokes deploy credentials and cleans resources | Reject lingering service access after purge | 1.48, 1.46 | 2, 5, 11, 12 | **PLANNED** |
| **1.89** | governance / P1 | **Energy aware scheduler fallback:** Use trusted energy budget and opt in for grid aware compute | Actual meter readings govern optional batch work with offline fallback | Reject fabricated energy surplus or implicit paid execution | 1.48, 1.87 | 2, 5, 11, 12 | **PLANNED** |
| **1.90** | architecture / P0 | **Cargo Workspace migration boundary:** Preserve existing package while staging six functional workspace crates | Cargo metadata and cargo test workspace complete on main with old API intact | Reject cyclic crate graph and removed public CLI | 0.42, 1.59, 1.64 | 2, 5, 6, 12, 13 | **PLANNED** |
| **1.91** | architecture / P0 | **Six crate typed interfaces:** Define transport-crdt telemetry-ebpf cognition-rqgm economy-gov domain-swarms master-node boundaries | Six real crates compile with schema contracts and no unauthorized cycles | Reject direct dependency on privileged runtime from untrusted plugin | 1.90, 1.65, 1.74, 1.84, 1.86 | 2, 5, 6, 12, 13 | **PLANNED** |
| **1.92** | architecture / P1 | **Core domain swarms integration:** Bind OmniDev OmniUX OmniE2E OmniForge OmniWeb OmniMath and OmniSec to capability registry | Seven swarms execute real authorized fixture tasks with trace and audit | Reject fabricated tool completion and missing sandbox | 1.91, 0.43, 0.56 | 2, 5, 6, 12, 13 | **PLANNED** |
| **1.93** | architecture / P1 | **Extended domain adapters:** Integrate OmniNous OmniHoncho OmniZeroClick OmniDiT OmniLex and OmniHFT as opt in adapters | Real backend smoke or explicit DISABLED gate for all six systems | Reject absent credentials reported as live deployment | 1.91, 1.92, 1.00, 1.02 | 2, 5, 6, 12, 13 | **PLANNED** |
| **1.94** | architecture / P0 | **Sovereign master integrated workflow:** Compose broker Firecracker PostgreSQL CRDT telemetry cognition action gates and swarms | One authorized request has verified identity trace durable result and recovery | Reject network partition stale epoch or missing evidence as success | 1.90, 1.91, 1.92, 1.93, 0.39, 0.40, 0.41 | 2, 5, 6, 12, 13 | **PLANNED** |

## Nowe podetapy wykonawcze: 16

| ID | Owner/Priorytet | Konkretny rezultat | Pozytywny test akceptacji | Test negatywny | Zależności | Strony PDF | Status |
|---|---|---|---|---|---|---|---|
| **0.39.26** | runtime / P0 | **WASI guest provenance:** Bind live WASIp1 execution receipt to broker lease and kernel identity | Two real guest nonces show distinct Wasmtime child exit and hashes | Reject host side WASI pretending to be guest execution | 0.39.20, 1.94 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.39.27** | runtime / P0 | **In guest eBPF loading:** Build and actually insert Aya syscall probe inside Firecracker kernel | Authenticated guest BPF program ID and openat events collected | Reject host Aya proof as guest program ID | 1.80, 1.81 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.39.28** | runtime / P0 | **Guest process quarantine:** Enforce authorized PID isolation after corroborated syscall anomaly | Guest child is stopped while second tenant VM remains alive | Reject entropy only irreversible SIGKILL | 1.82, 1.84 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.39.29** | runtime / P0 | **VM trace propagation:** Carry broker traceparent through Firecracker boot and guest receipt | Collector correlates two independent VM traces under correct tenant | Reject reused cross tenant serial receipt | 1.61, 1.83, 0.39.26 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.40.11** | database / P0 | **Live WAL vs artifact recovery distinction:** Require direct two kernel TCP stream to count as distributed HA | Two separate Linux boot IDs continuously exchange actual PostgreSQL WAL | Reject artifact mediated cold restore counted as TCP streaming | 0.40.3, 1.94 | 1, 5, 6 | **PLANNED** |
| **0.40.12** | database / P0 | **Postgres quorum witness:** Require third witness fencing token before standby promotion | Network partition and old primary return never yield two writable nodes | Reject expired epoch or dual primary overlap | 0.40.8, 0.40.11 | 1, 5, 6 | **PLANNED** |
| **0.40.13** | database / P0 | **Distributed escrow durability:** Persist authorized billing and job reservations on fenced transactional HA store | Failover preserves committed ledger and refuses duplicate settlements | Reject stale leader ledger modifications | 1.87, 0.40.12 | 1, 5, 6 | **PLANNED** |
| **0.40.14** | database / P0 | **WAL failover trace continuity:** Correlate LSN lag and timeline across old and new leader | Real promotion trace shows exact WAL catchup and reconnect | Reject partial trace claiming RPO zero | 0.40.12, 1.61 | 1, 5, 6 | **PLANNED** |
| **0.41.11** | reliability / P0 | **Eight step method inventory:** Document per step owner test threat model and source-page traceability | Automation detects missing step for all eight research phases | Reject synthetic empirical PDF benchmarks as CI evidence | 1.55, 1.94 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.41.12** | reliability / P0 | **Integrated 60 minute soak:** Stress live broker VM PG WAL and swarms concurrently for at least 60min | Measured duration and p95 memory LSN exits and errors archived | Reject 30s smoke called one hour test | 1.94, 0.40.12 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.41.13** | reliability / P0 | **Memory and trace chaos:** Fault inject CRDT divergence delayed trace export and Titans restore | No cross tenant leak unbounded queue or revoked snapshot recovery | Reject memory accuracy score as chaos validation | 1.59, 1.62, 1.68 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.41.14** | reliability / P0 | **Settlement disaster reconciliation:** Test escrow failover L402 expiry and delayed authorization responses | Post restore ledger matches genuine events without double charge | Reject fake settlement and spending after revoke | 1.85, 1.87, 1.89 | 1, 2, 5, 12, 13 | **PLANNED** |
| **0.42.1** | architecture / P0 | **Cargo backwards compatibility:** Stage Rust workspace crates without rewriting existing root runtime binary | All existing cargo tests and CLI entrypoints pass on main | Reject breaking API without migration and tests | 1.90, 1.91 | 5, 6, 13 | **PLANNED** |
| **0.42.2** | architecture / P0 | **Workspace licensing and SBOM:** Record actual upstream license resolved hash and dependency vulnerabilities | Pinned SBOM cargo deny and audit cover all crates | Reject unlicensed code or unpinned binary | 1.91, 1.93 | 5, 6, 13 | **PLANNED** |
| **0.42.3** | architecture / P0 | **Release evidence scope matrix:** Separate real host, guest, Loihi, LN and test lab proof per feature | Production gate demands real evidence IDs and scopes | Reject prototype PDF code counted as production PASS | 1.94, 0.41.11 | 5, 6, 13 | **PLANNED** |
| **0.42.4** | architecture / P0 | **Reproducible rollback:** Sign manifests and test rollback of workspace update on fresh runner | Artifact SHA and prior version behavior recover after rollback | Reject revoked secrets or unsigned image restored | 0.42.1, 0.42.2, 0.42.3 | 5, 6, 13 | **PLANNED** |

## Docelowe granice Cargo Workspace — sześć nazw z PDF

- `ocaf-transport-crdt` — planowana granica crate, **niepotwierdzona jako zaimplementowana**.
- `ocaf-telemetry-ebpf` — planowana granica crate, **niepotwierdzona jako zaimplementowana**.
- `ocaf-cognition-rqgm` — planowana granica crate, **niepotwierdzona jako zaimplementowana**.
- `ocaf-economy-gov` — planowana granica crate, **niepotwierdzona jako zaimplementowana**.
- `ocaf-domain-swarms` — planowana granica crate, **niepotwierdzona jako zaimplementowana**.
- `ocaf-master-node` — planowana granica crate, **niepotwierdzona jako zaimplementowana**.

Wszystkie funkcje nowych etapów wymagają: prawdziwego repozytoryjnego kodu, wersji i licencji dependencies, wykonanych testów pozytywnych i negatywnych, CI run ID, zapisanych artefaktów SHA-256 oraz dowodu rzeczywistego backendu. Dopóki nie zostaną udokumentowane, status **PLANNED**.

Pomiary z dokumentu należy powtórzyć na rzeczywistej infrastrukturze: nie akceptować fałszywych dowodów sprzętu Loihi, niewykonanej sondy guest eBPF ani opłacenia L402 potwierdzonego wyłącznie przez SHA-256 preimage. Audyt lub osobne podpisy nie są blokadą *developmentu*, lecz żaden wpis w roadmapie sam nie jest akceptacją produkcyjną.

**Manifest maszynowy:** `docs/roadmap/sovereign_workspace_v9.json`. Każdy z 56 identyfikatorów w tym pliku ma dokładnie jeden wiersz w tabeli i jawne testy ujemne.
