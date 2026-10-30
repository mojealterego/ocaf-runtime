# Argon PDF 1.15–1.54 — uzgodnienie z kanoniczną Roadmapą OCAF (2026-10-30)

## Źródło i zasada numeracji

Źródło: Google Drive `Wdrożenie Roadmapy Projektu OCAF-Runtime.PDF`, ID `1ezPUBRwroogi_vJg9oYc7OjlcYjPO5z3`. Wierna transkrypcja już znajduje się w `docs/sources/drive-ocaf/roadmap-115-154.txt`. PDF nazywa swoje 40 wymagań 1.15–1.54, lecz **nie są to etapy 1.15–1.54 kanonicznej Roadmapy v11**. Kanoniczny etap 1.15 oznacza Zenoh SHM, nie LabelPath; kanoniczny 1.54 oznacza L402 gateway, nie ClaimLedgerVerifier. Nie wolno renumerować ani zastępować etapów v8. Poniższe `PDF-1.xx` to identyfikatory wymagań dokumentu, nie numery nowych etapów.

## Macierz kompletności (40/40)

Status `PARTIAL` oznacza wykryty, ograniczony kod w repozytorium, nie pełne kryteria odbioru. `PREFLIGHT` oznacza samodzielną bibliotekę walidacji, nie integrację ani autoryzację efektów. `OPEN` oznacza brak potwierdzenia pełnej implementacji po przeglądzie źródeł, nie dowód nieistnienia kodu w innych repozytoriach.

| ID źródłowe | Wymaganie Argona | Status | Istniejący kod / wymagany dowód |
|---|---|---|---|
| PDF-1.15 | Hierarchiczne LabelPath | PARTIAL | `src/document_state.rs`; testy ścieżek i rodziców |
| PDF-1.16 | AttributeMap z unikalnym GUID | PARTIAL | `src/document_state.rs`; brak pełnych sterowników atrybutów |
| PDF-1.17 | Forget/Remember | PARTIAL | `src/document_state.rs`; stan lokalny |
| PDF-1.18 | Atomowe transakcje i Undo/Redo | PARTIAL | `src/document_state.rs`; ograniczona historia |
| PDF-1.19 | Wersjonowany, trwały dziennik SHA-256 | PARTIAL | `src/document_store.rs`; brak zewnętrznego zakotwiczenia Head |
| PDF-1.20 | Deterministyczny replay bez efektów | PARTIAL | `src/document_store.rs`; brak pełnego chaos/DR |
| PDF-1.21 | ExternalLinkSupervisor | PARTIAL | `resolve_link` w `src/document_store.rs`; brak trwałego rejestru i topologii |
| PDF-1.22 | StorageRetrievalPluginDriver | PARTIAL | SQLite DocumentDriver; brak pięciu sterowników i migracji |
| PDF-1.23 | AgentContractSpec | PREFLIGHT | `src/governance_contract.rs`; brak wiązania z OMAt |
| PDF-1.24 | ExactAllowlistPolicyGate | PREFLIGHT | `src/governance_contract.rs`; osobna bramka, nie zamiennik istniejącego Auth |
| PDF-1.25 | RiskCapEnforcer | PREFLIGHT | `src/governance_contract.rs`; procurement > MEDIUM odrzucane |
| PDF-1.26 | ApprovalBindingEngine | PREFLIGHT | HMAC verifier; wymagane zaufane IdP, wydawca i durable nonce consumption |
| PDF-1.27 | IdempotencyGuard | OPEN | Wymagany trwały atomowy klucz (tenant, akcja, nonce, hash) i recovery |
| PDF-1.28 | DecisionToAuditWiring | OPEN | Wymagany trwały zapis ALLOW/DENY **przed** handlerem i fail-closed |
| PDF-1.29 | TenantWorkspaceIsolator | OPEN | Wymagane mapowanie autorytatywnej tożsamości, trwałość i izolacja |
| PDF-1.30 | CostAndProviderAccessGate | PREFLIGHT | `src/governance_contract.rs`; wymagana integracja z rzeczywistym billingiem |
| PDF-1.31 | ContractObligationRecord | OPEN | Typowana domena i walidacja kontraktów |
| PDF-1.32 | StructuredClauseExtractor | OPEN | Rzeczywisty parser, ślad źródłowy i testy klauzul |
| PDF-1.33 | ObligationDeadlineMonitor | OPEN | Terminy, strefy czasowe, kalendarze, testy brzegowe |
| PDF-1.34 | ObligationActionVerification | OPEN | Sprawdzenie dowodów i stanu działania |
| PDF-1.35 | Contract Obligations policy | OPEN | Egzekwowanie LOW/MEDIUM i human approval |
| PDF-1.36 | Contract Obligations tool exposure | OPEN | Schemat narzędzia, walidacja I/O |
| PDF-1.37 | Contract Obligations governed runtime | OPEN | Rzeczywiste spięcie z istniejącym OMAt |
| PDF-1.38 | Contract Obligations safety E2E | OPEN | Terminy, dowody, zgody i cross-tenant negative |
| PDF-1.39 | ComplianceEvidenceArtifact | OPEN | Model dowodu, źródło, ważność, identyfikator |
| PDF-1.40 | GovAssure / NCSC CAF / SOC 2 validator | OPEN | Wersjonowane normy i testy kompletności |
| PDF-1.41 | Compliance Evidence governed runtime | OPEN | Rejestracja, audyt i polityka wykonania |
| PDF-1.42 | Compliance Evidence safety tests | OPEN | Tamper, expiry, cross-tenant, odmowy |
| PDF-1.43 | ProcurementScout foundation | OPEN | Typowane zapytania, dostawcy, dane źródłowe |
| PDF-1.44 | Sanctions / supplier anomaly screening | OPEN | Aktualne źródła list, dopasowanie i false positives |
| PDF-1.45 | Procurement governance | OPEN | Risk cap, zgoda, segregacja obowiązków |
| PDF-1.46 | Procurement safety tests | OPEN | Brak płatności/zakupów bez zgody, retry |
| PDF-1.47 | InProcessLexicalSelector | OPEN | BM25-lite, korpus, testy relevance |
| PDF-1.48 | MossRetrievalAdapter | OPEN | Zewnętrzny interfejs dopiero po zweryfikowaniu dostawcy; local fallback |
| PDF-1.49 | ZenohTransportBridge | OPEN | Istnieje transport Zenoh, brak E2E tego konkretnego mostu i benchmarku |
| PDF-1.50 | McpRegistryIntegrityVerifier | OPEN | Nie potwierdzono deklarowanych 210 skilli / 420 sum |
| PDF-1.51 | DualTrackCollaborationAnalyzer | OPEN | Historia zmian obiektu i atrybucja aktorów |
| PDF-1.52 | RetrievalQualityEvaluator | OPEN | Ground-truth, Recall@k i MRR |
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
