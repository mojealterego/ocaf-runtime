# OCAF Runtime — reconciliacja źródłowego zakresu 1.55–2.06 (2026-11-14)

**Stan:** `SOURCE_RECONCILED / CROSSWALK_TESTED_LOCAL / RUST_SOURCE_ADDED_UNVERIFIED`  
**Repozytorium:** `mojealterego/ocaf-runtime` / `main`  
**Źródło:** „Architektura Wykonawcza OCAF-Runtime: Mapa Drogowa (1.55–2.06) oraz Kompletna Implementacja Systemowa” (tekst przekazany przez użytkownika).  
**Kanoniczne źródło prawdy:** `docs/roadmap/canonical_v11_inventory.json`, 334 główne i 64 techniczne identyfikatory, bez przenumerowania; `production_approved=false`.

## Konflikt semantyczny identyfikatorów

Numery od `1.55` do `2.06` w tekście są identyfikatorami **lokalnymi źródła** i często opisują zupełnie inne zagadnienia niż numery o tej samej postaci w kanonicznym OCAF. Dla jednoznaczności oznaczamy je `SOURCE-1.55`, ..., `SOURCE-2.06` w osobnym manifestcie. Nie dodajemy nowych etapów, nie zastępujemy nazw ani nie podnosimy statusów.

## Pełna mapa 20 punktów źródłowych

| ID źródłowe | Moduł według specyfikacji | Istniejący kanoniczny etap | Stan źródłowy | Istniejące implementacje |
|---|---|---|---|---|
| `SOURCE-1.55` | OCAFLabelTree & GUIDAttribute | `1.55` — Transport benchmark SHM/TCP | `EXISTING_PARTIAL` | `src/document_state.rs` |
| `SOURCE-1.56` | TransactionAndUndoManager | `1.56` — SHM tenancy and quota | `EXISTING_PARTIAL` | `src/document_state.rs`, `src/document_store.rs` |
| `SOURCE-1.57` | FunctionMechanismDAG | `1.57` — Dual CRDT convergence corpus | `EXISTING_PARTIAL` | `src/document_function_dag.rs` |
| `SOURCE-1.58` | GovernedPolicyGate | `1.58` — Edge Zenoh interoperability | `EXISTING_PARTIAL` | `src/governance_contract.rs` |
| `SOURCE-1.59` | DurableReplayLedger | `1.59` — Transport and CRDT release matrix | `EXISTING_PARTIAL` | `src/governance_journal.rs`, `src/document_store.rs` |
| `SOURCE-1.60` | PersistencePluginEngine | `1.60` — OTel resource precedence | `EXISTING_PARTIAL` | `src/document_store.rs` |
| `SOURCE-1.65` | HybridRetrievalSubsystem | `1.65` — CoALA typed memory API | `EXISTING_PARTIAL` | `src/retrieval_quality.rs` |
| `SOURCE-1.70` | CompoundReasoningEngine | `1.70` — RQGM immutable examiner epoch | `EXISTING_PARTIAL` | `src/compound_reasoning.rs` |
| `SOURCE-1.75` | AdaptiveModelRouter | `1.75` — SDNN arithmetic property corpus | `EXISTING_PARTIAL` | `src/model_router.rs` |
| `SOURCE-1.80` | AgenticPromptCompiler | `1.80` — In guest Aya BPF verifier gate | `EXISTING_PARTIAL` | `src/agentic_prompt_dsl.rs` |
| `SOURCE-1.85` | GEMExecutiveOODALoop | `1.85` — L402 settlement provenance | `EXISTING_PARTIAL` | `src/om_engine.rs` |
| `SOURCE-1.90` | DomainGovernanceSuite | `1.90` — Cargo Workspace migration boundary | `EXISTING_PARTIAL` | `src/contract_obligations.rs`, `src/compliance_evidence.rs`, `src/procurement_scout.rs` |
| `SOURCE-1.95` | EpistemicClaimLedger | `1.95` — Hyperon/MeTTa neuro-symbolic research adapter | `EXISTING_PARTIAL` | `src/claim_ledger_verifier.rs`, `src/mcp_registry_integrity.rs` |
| `SOURCE-2.00` | OCAFUnifiedNexusKernel | `2.00` — Taskweaving dependency graph and multi-agent rollback | `EXISTING_PARTIAL` | `src/document_state.rs`, `src/governance_contract.rs`, `src/governance_journal.rs`, `src/om_engine.rs` |
| `SOURCE-2.01` | ZenohZeroLatencyTransport | `2.01` — AI Scientist reproducible research lifecycle | `EXISTING_PARTIAL` | `src/transport_policy.rs`, `crates/ocaf-transport-crdt/src/lib.rs` |
| `SOURCE-2.02` | CollaborativeObjectTracker | `2.02` — OpenHands agent server and legacy repository adapter | `EXISTING_PARTIAL` | `src/collaboration_tracker.rs` |
| `SOURCE-2.03` | VoiceNarrativeStateFirewall | `2.03` — Theory-of-Mind evaluation and human approval gates | `EXISTING_PARTIAL` | `src/voice_narrative_firewall.rs` |
| `SOURCE-2.04` | RustHotPathKernel | `2.04` — EideticEngine metacognitive memory-control evaluation | `EXISTING_PARTIAL` | `src/task_cas.rs`, `src/governance_contract.rs` |
| `SOURCE-2.05` | ReproducibleBenchHarness | `2.05` — Agent-Zero and Bonfire self-programming research sandbox | `OPEN` | Brak istniejącego modułu |
| `SOURCE-2.06` | ReleaseReadinessGuardian | `2.06` — NuNet PRIMUS and GodMode experimental orchestration interfaces | `EXISTING_PARTIAL` | `scripts/release_readiness.py` |

Status `EXISTING_PARTIAL` oznacza jedynie odnalezienie elementów mechanizmu w plikach źródłowych, **nie oznacza pełnego wdrożenia ani PASS**. Pełne ograniczenia każdego wpisu zawiera `docs/roadmap/ocaf_source_155_206_manifest.json`. Wpis `OPEN` nie jest obejściem brakującej funkcjonalności.

## Weryfikacja wykonana w ramach przyrostu

1. Dodano `scripts/verify_source_155_206.py` — stdlib Python, strict JSON bez duplikatów, twarda lista 20 identyfikatorów, przypięcie nazw aktualnych etapów do kanonicznego manifestu, blokada `production_approved`, sprawdzanie istnienia plików, odmowa symlinków i `..`, odmowa fałszywych statusów CI. **Read-only**.
2. Dodano `scripts/tests/test_verify_source_155_206.py` z 14 przypadkami pozytywnymi/negatywnymi. Testy jednostkowe były lokalnie uruchomione na syntetycznym inwentarzu z nazwami etapów zweryfikowanymi w GitHub: **14/14 PASS**; nie stanowi to uruchomienia repozytoryjnego Rust/Cargo. Osobny lokalny przebieg sprawdził uzupełniony manifest z syntetycznymi kopiami ścieżek, nie na pełnym Checkout GitHub.
3. Dopisano odczytowy `CollaborationTracker::analyze_object` do istniejącego `src/collaboration_tracker.rs`: ważony wkład aktorów rozdzielony jako 10 000 punktów bazowych (largest-remainder, stabilne rozstrzyganie remisów), znormalizowany wskaźnik Gini-Simpson i chronologiczne zmiany fokusu **w ramach tenanta i obiektu**. Zapisano cztery nowe testy Rust; kompilacja i uruchomienie **UNVERIFIED**. Wynik analityczny nie daje prawa własności ani uprawnień.
4. Dodano jawny krok walidatora do istniejącego `.github/workflows/rust.yml`, bez tworzenia osobnych workflowów lub uruchamiania płatnego runnera.

## Ujawnione pułapki kodu referencyjnego (niekopiowane)

- **Poważna kolizja digestu autoryzacji w TS:** `JSON.stringify(payload, Object.keys(payload).sort())` używa tablicowej listy dozwolonych pól także na obiektach zagnieżdżonych. Dwa różne payloady `{outer:{amount_cents:100,vendor:"trusted"}}` i `{outer:{amount_cents:999999,vendor:"attacker"}}` dają identyczny łańcuch `{"outer":{}}` i identyczny SHA-256 `097111e9412e230f913f18883490d8d873bde2967f86474fed8f133714aa7123` — wynik z lokalnie uruchomionego Node.js 22. Niedopuszczalne do wiązania aprobaty. Niezbędna rekurencyjna, kanoniczna serializacja i ścisły schemat.
- **Podpis aprobaty:** `SHA256(message + ":" + secret)` oraz porównanie zwykłych łańcuchów nie zastępują HMAC z weryfikacją constant-time. Metoda `issueApprovalToken()` dostępna każdemu kodowi posiadającemu instancję PolicyGate nie weryfikuje niezależnej tożsamości aprobującego. W OCAF istnieje oddzielny kontrakt `ApprovalAuthority` bazujący na HMAC oraz bramki audytowe.
- **Trwałość:** `DurableReplayLedger` w TS przechowuje wpisy wyłącznie w `events: []` i `idempotencyIndex: Map`, co nie gwarantuje odzyskiwania po awarii ani write-ahead/outbox. Istniejące `src/governance_journal.rs` i `src/document_store.rs` mają kontrakty SQLite i jawne granice trwałości.
- **Kolejność zapisu:** `OCAFUnifiedNexusKernel.executeGovernedMutation` najpierw dopisuje do ledger, później mutuje stan. Błąd mutacji pozostawia wpis `allowed` bez rzeczywistego commita. Integracja musi atomowo związać decyzję, stan, audyt i idempotencję lub jawnie rozdzielić `PENDING/COMMITTED/FAILED`.
- **Integracja wyszukiwania:** przykładowy `HybridRetrievalEngine.indexRecord` nie usuwa dawnych tokenów przy nadpisywaniu `id`; adapter Moss przyjmuje dostarczone wyniki bez tenant-bound provenance i zaufanego indeksu.
- **Kompilacja TS:** przykład `CompoundReasoningEngine` odwołuje się do niezdefiniowanego w ciele funkcji `cacheKey`; nie jest gotowym artefaktem kompilacyjnym.
- **Kontrola wydań:** zewnętrzne założenie „11 historycznych branch refs” nie jest kontraktem lokalnego repozytorium OCAF. Samo zliczenie nazw nie potwierdza zachowania obiektów Git; sztuczny `gitCommitSha: "v2.06-verified-head"` nie jest dowodem pochodzenia.
- **Epistemika:** zgodność modeli, entropia wkładu aktorów ani pojedynczy hash nie dowodzą prawdy, certyfikacji lub legalnej współwłasności.
- **Zakupy i ekonomia:** wartości w USD wymagają typów całkowitoliczbowych, podpisanych approvals, prawdziwego autoryzowanego provider gate i niezależnego rozliczenia; bez tego nie wolno wykonywać zakupu.

## Minimalne bramki pozostałe do potwierdzenia

```bash
python3 -m unittest discover -s scripts/tests -p test_verify_source_155_206.py -v
python3 scripts/verify_source_155_206.py --repo .
cargo fmt --all -- --check
cargo test --locked --no-default-features --lib collaboration_tracker::tests
cargo clippy --locked --all-targets --no-default-features -- -D warnings
```

**Brak autoryzacji produkcyjnej**: nie przedstawiono podpisów operatora, wykonanych testów Rust na docelowym SHA, rzeczywistego potwierdzenia wydajności, integracji Moss, działania wielu hostów, pełnego E2E ani dowodu zatwierdzenia wydań. Inne repozytoria wymienione w źródłowym opracowaniu nie były w tym przyroście niezależnie audytowane.
