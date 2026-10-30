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
| 2.03 | Voice Narrative Firewall | GAP: brak potwierdzonego modułu |
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
