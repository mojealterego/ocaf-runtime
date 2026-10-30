# Argon 1.55–2.06 — rejestr kolizji

Dokument Drive `1gyLwD4LMaUarwUCpl6I4_TNxAr2F-P6A` opisuje inne funkcje niż aktualne kanoniczne etapy 1.55–2.06. Nie zastępować istniejącej numeracji ani uznawać przykładowego kodu za wdrożenie.

## Mapowanie wymagania → istniejący kod / luka

| Punkt źródłowy | Wymaganie | Stan po weryfikacji kodu |
|---|---|---|
| 1.55 | Label/Tag/GUID | PARTIAL: `src/document_state.rs`; brak pełnej semantyki Open CASCADE |
| 1.56 | Undo/Redo | IMPLEMENTED_SCOPE: `src/document_state.rs`; snapshoty, atomowe transakcje |
| 1.57 | DAG funkcji dokumentu | GAP: `src/omnidev_workflow.rs` to inny DAG |
| 1.58 | PolicyGate | PARTIAL: `src/governance_contract.rs`, `src/governance_journal.rs` |
| 1.59 | ReplayLedger | PARTIAL: `src/document_store.rs`, `src/governance_journal.rs` |
| 1.60 | Persystencja i migracje | PARTIAL: SQLite driver; brak wieloformatowych migracji |
| 1.65 | BM25/Moss/Recall@k/MRR | PARTIAL: `src/retrieval_quality.rs`; brak Moss E2E |
| 1.70 | Compound reasoning | GAP: brak kompletnego backendu i kontrolowanego fan-out |
| 1.75 | Adaptive Model Router | GAP: brak kompletnego modelowego routera |
| 1.80 | Prompt DSL | GAP: brak kompilatora i walidacji DSL |
| 1.85 | GEM/OODA/H-MoE | RESEARCH: brak sprawdzonej integracji |
| 1.90 | Kontrakty/compliance/procurement | PARTIAL: `src/contract_obligations.rs`, `src/compliance_evidence.rs` |
| 1.95 | ClaimLedger/SHA256SUMS | PARTIAL: `src/claim_ledger_verifier.rs`, `src/mcp_registry_integrity.rs` |
| 2.00 | Unified Kernel | PARTIAL: istniejący Rust runtime, nie tworzyć drugiego rdzenia |
| 2.01 | Zenoh pub/sub | PARTIAL: `src/transport/`; brak E2E delt dokumentu |
| 2.02 | Collaborative Object Tracker | GAP: brak potwierdzonego modułu |
| 2.03 | Voice Narrative Firewall | IMPLEMENTED_SCOPE_CI_PENDING: `src/voice_narrative_firewall.rs`, tenant binding i kontrola zgody mikrofonu; realny audio/game engine E2E OPEN |
| 2.04 | Rust Hot Path | PARTIAL: Rust obecny; brak zweryfikowanego benchmarku |
| 2.05 | Benchmark Harness | PARTIAL: skrypty i CI, brak jednolitego korpusu |
| 2.06 | Release Guardian | PARTIAL: `scripts/release_readiness.py`, brak pełnego E2E |

**Ograniczenie źródła:** PDF zawiera przykładowy TypeScript z artefaktami `[span_...]`; nie jest dowodem kompilacji, pomiaru wydajności, uruchomienia Moss ani realnego Zenoh. Nie kopiować TypeScript jako drugiego jądra Rust. Statusy dotyczą wymagań, nie ukończenia kanonicznych etapów.

## Bramki odbioru brakujących funkcji

- **DAG dokumentu (źr. 1.57):** topologiczne sortowanie, wykrywanie cykli i duplikatów GUID, dirty propagation, typowane wejścia, overflow, atomowy commit/undo, SQLite restart/replay, odrzucenie stale head i testy dwóch tenantów. Nie zastępować go DAG orkiestracji agentów.
- **Compound reasoning / routing / DSL (źr. 1.70–1.80):** zaufana allowlista modeli, prywatność per tenant, jawny koszt i p95, bounded fan-out, cancellation, retry/backpressure, schemat wyników, test prompt injection i test rzeczywistego adaptera modelu. Self-consistency nie jest dowodem prawdziwości.
- **GEM i pakiety domenowe (źr. 1.85–1.90):** wersjonowane kontrakty i policy gate, jawne decyzje człowieka dla działań wysokiego ryzyka, negatywne testy zakupów i braku dowodów, brak deklaracji AGI.
- **Zenoh / kolaboracja (źr. 2.01–2.02):** transport między realnymi procesami, trwała identyfikacja obiektu/aktora, monotoniczna wersja i fencing, izolacja tenantów, test reorder/duplicate/reconnect i pomiar na rzeczywistym sprzęcie.
- **Voice firewall (źr. 2.03):** narracja bez możliwości zmiany RNG/ekonomii, jawna zgoda na mikrofon, walidacja zdarzeń, odmowa nieautoryzowanych mutacji, test prompt/tool-output injection.
- **Benchmark / release (źr. 2.04–2.06):** pin SHA/CPU/OS/Rust, korpus warm/cold, raw p50/p95/p99, CI pozytywne i negatywne, SBOM, rollback, niezależna kontrola danych, jawny wynik GO/NO-GO; brak automatycznego oznaczenia produkcyjnego PASS.

**Stan wdrożenia po audycie:** brak nowej zweryfikowanej implementacji z tego PDF. Wymagania są zmapowane, ale nie należy podnosić statusu kanonicznej Roadmapy bez kodu i dowodów CI.

## Identyfikatory kanoniczne i status wdrożenia (aktualizacja 30.10.2026)

**Reguła:** wskazane etapy aktualnej ROADMAPY mają związek semantyczny, ale nie są numerami PDF Argona. `IMPLEMENTED_SCOPE` oznacza wyłącznie konkretne funkcje w kodzie; `CI_PENDING` oznacza brak zielonego dowodu dla zmiany. Istniejące etapy zachowują status niezależny od tego rejestru.

| Punkt Argona | Powiązania z aktualną Roadmapą | Status tego wymagania |
|---|---|---|
| 1.55 | 0.67, 1.90; nowy zakres modelu dokumentu | PARTIAL |
| 1.56 | 0.12, 0.67; nowy zakres transakcyjny | IMPLEMENTED_SCOPE, wcześniejsze testy; pełna kompatybilność OCAF OPEN |
| 1.57 | 0.67, 1.94, 227; **odrębny DAG atrybutów** | IMPLEMENTED_SCOPE_CI_PENDING: `src/document_function_dag.rs`, `tests/document_function_dag.rs`; nie mylić z DAG agentów |
| 1.58 | 0.43, 0.64, 1.51 | PARTIAL — governance preflight nie zastępuje authz wykonawcy |
| 1.59 | 0.12, 0.19, 0.42 | PARTIAL — trwały replay i hash chain, potrzebny niezależny pin |
| 1.60 | 0.67, 1.90 | PARTIAL — SQLite storage, migracje i pluginy OPEN |
| 1.65 | 0.67, 0.95, 322 | PARTIAL — BM25/metrics lokalnie, Moss OPEN |
| 1.70 | 0.95, 1.69, 1.94 | OPEN — realny LLM fan-out/cache/consensus |
| 1.75 | 224, 1.94; router ogólny odrębny od audio 1.05/307 | IMPLEMENTED_SCOPE_CI_PENDING: `src/model_router.rs`; rzeczywisty provider E2E OPEN |
| 1.80 | 1.06, 227, 309 | OPEN — DSL, typowane koperty i walidacja |
| 1.85 | 0.67, 1.95 | RESEARCH_OPEN — H-MoE/OODA/GEM bez AGI claims |
| 1.90 | 0.64, 241, 280 | PARTIAL — bounded compliance/contract; realne działania OPEN |
| 1.95 | 0.42, 0.43, 280, 333 | PARTIAL — ClaimLedger/MCP integrity, niezależne receipt E2E OPEN |
| 2.00 | 1.94, 334 | PARTIAL — istniejący jedyny runtime, bez drugiego jądra |
| 2.01 | 0.46, 1.55–1.59 | PARTIAL — Zenoh w runtime, delty dokumentu E2E OPEN |
| 2.02 | 0.67, 1.65, 322 | OPEN — collaborative object tracker i provenance |
| 2.03 | 0.99, 1.11, 276 | IMPLEMENTED_SCOPE_CI_PENDING: `src/voice_narrative_firewall.rs`; rzeczywisty game/audio E2E OPEN |
| 2.04 | 1.90, 264 | PARTIAL — Rust hot path istnieje, brak pomiarów efektu |
| 2.05 | 0.42, 1.64, 264 | PARTIAL — brak wspólnego pełnego benchmark harness |
| 2.06 | 0.42, 1.94, 333 | PARTIAL — release precheck, brak kompletnego GO/NO-GO E2E |

**Zmiany kodu do weryfikacji CI:** `src/document_state.rs` (Update), `src/document_store.rs` (replay Update), `src/document_function_dag.rs`, `tests/document_function_dag.rs`, `src/model_router.rs`, `src/lib.rs`. Nowych liczb etapów nie dopisano i nie zmieniono historycznych statusów. Pełna integracja wymaga dalszej implementacji.
