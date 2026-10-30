# Argon PDF 1.15–1.54 — uzgodnienie z kanoniczną Roadmapą OCAF (2026-10-30)

## Źródło i zasada numeracji

Źródło: Google Drive `Wdrożenie Roadmapy Projektu OCAF-Runtime.PDF`, ID `1ezPUBRwroogi_vJg9oYc7OjlcYjPO5z3`. Wierna transkrypcja już znajduje się w `docs/sources/drive-ocaf/roadmap-115-154.txt`. PDF nazywa swoje 40 wymagań 1.15–1.54, lecz **nie są to etapy 1.15–1.54 kanonicznej Roadmapy v11**. Kanoniczny etap 1.15 oznacza Zenoh SHM, nie LabelPath; kanoniczny 1.54 oznacza L402 gateway, nie ClaimLedgerVerifier. Nie wolno renumerować ani zastępować etapów v8. Poniższe `PDF-1.xx` to identyfikatory wymagań dokumentu, nie numery nowych etapów.

## Macierz kompletności (40/40)

Status `PARTIAL` oznacza wykryty, ograniczony kod w repozytorium, nie pełne kryteria odbioru. `PREFLIGHT` oznacza samodzielną bibliotekę walidacji, nie integrację ani autoryzację efektów. `OPEN` oznacza brak potwierdzenia pełnej implementacji po przeglądzie źródeł, nie dowód nieistnienia kodu w innych repozytoriach.

| ID źródłowe | Wymaganie Argona | Status | Istniejący kod / wymagany dowód |
|---|---|---|---|
| PDF-1.15 | Hierarchiczne LabelPath | TESTED_LOCAL_PARTIAL | `src/document_state.rs`; testy ścieżek i rodziców |
| PDF-1.16 | AttributeMap z unikalnym GUID | TESTED_LOCAL_PARTIAL | `src/document_state.rs`; brak pełnych sterowników atrybutów |
| PDF-1.17 | Forget/Remember | TESTED_LOCAL_PARTIAL | `src/document_state.rs`; stan lokalny |
| PDF-1.18 | Atomowe transakcje i Undo/Redo | TESTED_LOCAL_PARTIAL | `src/document_state.rs`; ograniczona historia |
| PDF-1.19 | Wersjonowany, trwały dziennik SHA-256 | TESTED_LOCAL_PARTIAL | `src/document_store.rs`; brak zewnętrznego zakotwiczenia Head |
| PDF-1.20 | Deterministyczny replay bez efektów | TESTED_LOCAL_PARTIAL | `src/document_store.rs`; brak pełnego chaos/DR |
| PDF-1.21 | ExternalLinkSupervisor | TESTED_LOCAL_PARTIAL | `resolve_link` w `src/document_store.rs`; brak trwałego rejestru i topologii |
| PDF-1.22 | StorageRetrievalPluginDriver | TESTED_LOCAL_PARTIAL | SQLite DocumentDriver; brak pięciu sterowników i migracji |
| PDF-1.23 | AgentContractSpec | TESTED_PREFLIGHT_ONLY | `src/governance_contract.rs`; brak wiązania z OMAt |
| PDF-1.24 | ExactAllowlistPolicyGate | TESTED_PREFLIGHT_ONLY | `src/governance_contract.rs`; osobna bramka, nie zamiennik istniejącego Auth |
| PDF-1.25 | RiskCapEnforcer | TESTED_PREFLIGHT_ONLY | `src/governance_contract.rs`; procurement > MEDIUM odrzucane |
| PDF-1.26 | ApprovalBindingEngine | TESTED_PREFLIGHT_ONLY | HMAC verifier; wymagane zaufane IdP, wydawca i durable nonce consumption |
| PDF-1.27 | IdempotencyGuard | TESTED_LOCAL_PARTIAL | `src/governance_journal.rs` — implementacja lokalna, testy CI weryfikowane osobno; pełny E2E otwarty. |
| PDF-1.28 | DecisionToAuditWiring | TESTED_LOCAL_PARTIAL | `src/governance_journal.rs` — implementacja lokalna, testy CI weryfikowane osobno; pełny E2E otwarty. |
| PDF-1.29 | TenantWorkspaceIsolator | OPEN | Wymagane mapowanie autorytatywnej tożsamości, trwałość i izolacja |
| PDF-1.30 | CostAndProviderAccessGate | TESTED_PREFLIGHT_ONLY | `src/governance_contract.rs`; wymagana integracja z rzeczywistym billingiem |
| PDF-1.31 | ContractObligationRecord | TESTED_LOCAL_PARTIAL | `src/contract_obligations.rs` — Model zobowiązania SHA-256; trwałość i OMAt otwarte |
| PDF-1.32 | StructuredClauseExtractor | TESTED_LOCAL_PARTIAL | `src/contract_obligations.rs` — Ekstraktor wąskiej gramatyki (shall/must/will, within N days) |
| PDF-1.33 | ObligationDeadlineMonitor | TESTED_LOCAL_PARTIAL | `src/contract_obligations.rs` — Lokalna ewaluacja terminów; scheduler/alerty E2E otwarte |
| PDF-1.34 | ObligationActionVerification | OPEN | Sprawdzenie dowodów i stanu działania |
| PDF-1.35 | Contract Obligations policy | OPEN | Egzekwowanie LOW/MEDIUM i human approval |
| PDF-1.36 | Contract Obligations tool exposure | OPEN | Schemat narzędzia, walidacja I/O |
| PDF-1.37 | Contract Obligations governed runtime | OPEN | Rzeczywiste spięcie z istniejącym OMAt |
| PDF-1.38 | Contract Obligations safety E2E | OPEN | Terminy, dowody, zgody i cross-tenant negative |
| PDF-1.39 | ComplianceEvidenceArtifact | TESTED_LOCAL_PARTIAL | `src/compliance_evidence.rs` — Modele dowodowe i integralność SHA-256; brak niezależnego trust anchor |
| PDF-1.40 | GovAssure / NCSC CAF / SOC 2 validator | TESTED_LOCAL_PARTIAL | `src/compliance_evidence.rs` — NCSC CAF A-D oraz okres ważności; brak certyfikacji GovAssure |
| PDF-1.41 | Compliance Evidence governed runtime | OPEN | Rejestracja, audyt i polityka wykonania |
| PDF-1.42 | Compliance Evidence safety tests | TESTED_LOCAL_PARTIAL | `src/compliance_evidence.rs` — 9 testów w tym tenant, expiry, tampering; brak safety E2E |
| PDF-1.43 | ProcurementScout foundation | OPEN | Typowane zapytania, dostawcy, dane źródłowe |
| PDF-1.44 | Sanctions / supplier anomaly screening | OPEN | Aktualne źródła list, dopasowanie i false positives |
| PDF-1.45 | Procurement governance | OPEN | Risk cap, zgoda, segregacja obowiązków |
| PDF-1.46 | Procurement safety tests | OPEN | Brak płatności/zakupów bez zgody, retry |
| PDF-1.47 | InProcessLexicalSelector | TESTED_LOCAL_COMPONENT | `src/retrieval_quality.rs` — implementacja lokalna, testy CI weryfikowane osobno; pełny E2E otwarty. |
| PDF-1.48 | MossRetrievalAdapter | OPEN | Zewnętrzny interfejs dopiero po zweryfikowaniu dostawcy; local fallback |
| PDF-1.49 | ZenohTransportBridge | OPEN | Istnieje transport Zenoh, brak E2E tego konkretnego mostu i benchmarku |
| PDF-1.50 | McpRegistryIntegrityVerifier | TESTED_LOCAL_PARTIAL | `src/mcp_registry_integrity.rs` — pinned SHA256SUMS verifier (fixture-only); brak realnych 420 hashów i MCP E2E |
| PDF-1.51 | DualTrackCollaborationAnalyzer | OPEN | Historia zmian obiektu i atrybucja aktorów |
| PDF-1.52 | RetrievalQualityEvaluator | TESTED_LOCAL_COMPONENT | `src/retrieval_quality.rs` — implementacja lokalna, testy CI weryfikowane osobno; pełny E2E otwarty. |
| PDF-1.53 | LatencyBenchmarkHarness | OPEN | Realne p50/p95/p99, hardware i surowe próbki |
| PDF-1.54 | ClaimLedgerVerifier | OPEN | Dowody commit/test/artefakt; oddzielić hipotezy i implementacje |

## Bezpieczeństwo i spójność architektury

- Dokument miesza **Open CASCADE Application Framework (OCAF)** z **OmniContext Agent Framework**. Drzewo LabelPath jest adaptowanym modelem danych, a nie dowodem integracji biblioteki Open CASCADE ani wspólnej tożsamości obu projektów.
- PDF zawiera zniekształcenia ekstrakcji (`[span_*]`, uszkodzone template strings), a niektóre przykłady TS opierają idempotencję na pamięci procesu. Nie kopiować ich bez testów i korekty semantyki.
- Wydanie HMAC zatwierdzenia musi nastąpić **poza agentem** przez uwierzytelniony system operatora. Sam MAC nie daje jednokrotności. Weryfikacja podpisu nie jest decyzją o wykonaniu.
- Existing `src/omas_policy.rs`, `src/stage044_omat_authz.rs`, `src/sasos_capability.rs`, `src/document_store.rs` mają własne bramki. Nowe kontrakty są dodatkową warstwą; nie wolno ich używać do obejścia mTLS, OAuth, replay, tenant fencing, OSTc ani audytu.
- Odczyt PDF nie dowodzi stanu obcych repozytoriów, dostawcy Moss, benchmarków sub-ms ani realnego użycia 210 skilli. Każdy taki dowód wymaga oddzielnego badania.
- Pliki binarne, instalatory i podpisy release są **odłożone na koniec**, bez zmiany statusów kanonicznych etapów. Nowe wymagania wymagające osobnych etapów należy numerować od kolejnego wolnego ID (po 334) dopiero po deduplikacji z kanoniczną macierzą.

## Weryfikacja

`src/governance_contract.rs` ma testy jednostkowe: exact allowlist/prohibition, cross-tenant/actor, risk cap, human approval role/HMAC/expiry/payload/request binding, provider cost gate, invalid configuration. Uruchomienie: `cargo test --locked --no-default-features --lib governance_contract::tests`. Dopóki CI dla konkretnego SHA nie zakończy się sukcesem, status `PREFLIGHT` nie oznacza `TESTED` ani `INTEGRATED`. Pełna bramka `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --no-default-features -- -D warnings`, `cargo test --locked --no-default-features`.

Następny techniczny zakres: durable audit + idempotency 1.27–1.29, następnie integracja preflight z istniejącym OMAt bez ścieżki bypass, po czym domeny 1.31–1.46 i retrieval 1.47–1.54.

## Uzgodnienie wszystkich 40 identyfikatorów źródłowych (2026-10-30)

[Manifest 40/40](../roadmap/argon_pdf_115_154_manifest.json) wiąże każdy `PDF-1.xx` z kandydatami kanonicznych etapów bez nadpisywania Apex 1.15–1.54. Mapowanie SEMANTIC_PROPOSED nie daje ukończenia etapu; UNMAPPED wymaga decyzji semantycznej.

Źródła domen 1.31–1.46 mają analogi w `mojealterego/Nowe-projekty` (agent-contract-obligations, agent-compliance-evidence, agent-procurement-scout), lecz nie potwierdzono ich integracji z Rust OMAt. Zewnętrzny `The-Zero-Latency-Builder-Sprint/docs/MOSS_ADAPTER_GATE.md` oznacza adapter Moss jako zablokowany do weryfikacji API. Nowe moduły `governance_journal.rs` i `retrieval_quality.rs` nie stanowią dowodu produkcyjnego E2E. `production_approved=false`.

## Dowody CI po korekcie Clippy i ograniczony zakres ukończenia

[GitHub Actions 40989435834 — PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/40989435834), SHA `3b3fe1329390f52dbde09a6babd9b637395163ac`: walidacja 40/40, rustfmt, testy SQLite journal, preflight/HMAC, BM25, Recall@k/MRR i dokument/replay, Clippy core. **PDF-1.52: ZROBIONE — testowany lokalnie komponent obliczania Recall@k/MRR.** Nie oznacza całej funkcji retrieval, kryteriów E2E ani kanonicznych etapów 1.65/1.99 jako ukończonych. Pozostałe testowane komponenty są PARTIAL lub PREFLIGHT, nie COMPLETE.

## Zakończenie lokalnych komponentów retrieval — odczyt CI 40990353117

[Run 40990353117](https://github.com/mojealterego/ocaf-runtime/actions/runs/40990353117) dla SHA `f618310d01543ce5475866f51263782f06c47810` zakończył się `success`: zweryfikowano formatowanie, Clippy, testy typowanego indeksu odwróconego BM25, izolacji tenantów, deterministycznych wyników, Recall@k/MRR i negatywnych przypadków. **PDF-1.47 i PDF-1.52: ZROBIONE tylko jako lokalne, testowane komponenty.** Brak potwierdzenia integracji z Zenoh/Moss, trwałego indeksu, rzeczywistego korpusu i wyników wydajnościowych. To nie jest odbiór całych etapów kanonicznych.

## Rozszerzenie implementacji PDF-1.31–33, PDF-1.39–40/42 — 2026-10-30

`src/contract_obligations.rs` zawiera deterministyczny model, parser ograniczony do klauzuli jawnie opatrzonej cytowaniem i `within N days`, SHA ID i monitor terminów. `src/compliance_evidence.rs` zawiera model dowodowy, SHA-256, expiry, izolację tenantów, NCSC CAF A–D i testy negatywne. Status **IMPLEMENTED_CI_PENDING**, brak prawdziwego scheduler/OMAt E2E, niezależnej atestacji, podpisu zgodności i produkcyjnego odbioru. Nie zamykać kanonicznych etapów.

### 2026-10-30 — CI PASS: Contract Obligations / Compliance Evidence

[Run 41010321126 — PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/41010321126), SHA `767230ec2a16ddbbb185c5df703cffe6256cf338`: weryfikacja Rustfmt, Clippy, testów kontraktów, dowodów SHA-256, NCSC CAF i pozostałych modułów Argona. PDF-1.31–1.33, PDF-1.39–1.40 i PDF-1.42 są **TESTED_LOCAL_PARTIAL**; brak produkcyjnej autoryzacji, rzeczywistej integracji z OMAt, zewnętrznego trust anchor, formalnych atestacji, schedulera i pełnych E2E. Kanoniczne etapy nie są oznaczone jako zakończone. `production_approved=false`.

### Dodatkowa weryfikacja PDF-1.50 — upstream MCP Registry

W rzeczywistym `mojealterego/Agent-God-Level-Omega/omega/SHA256SUMS.txt` odnaleziono 1100 linii SHA, w tym **420** wpisów w **210** katalogach `skills/*-mcp-registry`: dokładnie jeden `SKILL.md` i jeden `references/registry.md` na katalog. Dokument Argona błędnie wskazuje `reference.json` oraz ścieżkę z prefiksem `omega/` w samym SHA256SUMS (manifest ma ścieżki względne względem katalogu `omega`). To **kontrola kompletności nazw manifestu, NIE przeliczenie hashów 420 plików ani sprawdzenie live MCP**. PDF-1.50 pozostaje `OPEN` do czasu implementacji weryfikatora z zaufanym pinem, odczytu wszystkich plików, testów negatywnych i E2E. Żadnych binariów nie uruchamiano.

### 2026-10-30 — PDF-1.50: weryfikator Rust (testy CI w toku)

Dostarczono `src/mcp_registry_integrity.rs`: weryfikacja manifestu SHA256SUMS z niezależnie dostarczonym operacyjnym pinem SHA-256, dokładnie dwoma ścieżkami na skill, ograniczeniem rozmiaru, fail-closed przed błędnymi/duplikowanymi ścieżkami i różnicą zawartości plików. Testy negatywne pod `mcp_registry_integrity::tests`. **IMPLEMENTED_CI_PENDING**: do wyniku CI; nie pobrano 420 rzeczywistych plików, nie zakotwiczono operatorowego pinu, nie zintegrowano OMAt MCP E2E. Bez aktywacji binariów, bez zamknięcia etapu kanonicznego 0.43.

### PDF-1.50 — lokalne CI PASS, 2026-10-30

[Argon CI 41017007500 — PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/41017007500), commit `46522c5c506ddb634aa929260002c6e8494a7583`. Przeprowadzono testy weryfikacji integralności, odrzucenia nieprawidłowej pinu SHA-256, naruszeń SHA pliku, niepełnych/duplikowanych par i niebezpiecznych ścieżek. **TESTED_LOCAL_PARTIAL**: brak niezależnie operatorowo zakotwiczonego pinu prawdziwego upstream, brak ponownego przeliczenia wszystkich 420 realnych plików oraz uruchomienia MCP E2E. Kanoniczny etap 0.43 nadal niezamknięty.
