# OCAF OM Engine — architektura inżynieryjno-produkcyjna

**Wersja kontraktu:** OM Engine v0.1 / 2026-11-03  
**Repozytorium docelowe:** `mojealterego/ocaf-runtime`, wyłącznie `main`  
**Zakres:** trwała orkiestracja OmniDev (etap 0.56) i neutralny kontrakt pamięci dla przyszłej integracji Digital-Life.  
**Status:** przyrost implementacyjny, **nie** certyfikacja produkcyjna, nie ASI, nie uniwersalny wykonawca narzędzi.

## 1. Inwentaryzacja źródeł (stan przy analizie)

- OCAF `main`: `be11b8d0ac400d3781d5fbc3a411d472e80853c3` (początkowy SHA analizy).
- OCAF Rust 2021 / toolchain 1.99; `src/omnidev_workflow.rs` już implementuje ograniczony scheduler DAG, role, lease/epoch, rezerwacje kosztu, timeout reconciliation i hash audytowy; `src/agent_fleet.rs` oddziela manifesty agentów od gotowości; `src/builder_core.rs` zawiera typed graph IR; `src/stage044_omat_authz.rs` obsługuje bramkę OMAt/OAuth; `src/model_router.rs` selekcję profili, a `src/compound_reasoning.rs` ograniczoną deliberację.
- Digital-Life `main` na etapie analizy zawiera `src/imandrax_dgm/omni_temporal_memory.py` (SQLite, cztery osie czasu, tenant scope), `delta_crdt.py` (delta CRDT) i `model_contracts.py` (typed offline model inference). Moduły te nie stanowią jeszcze potwierdzonego połączenia OCAF–Digital-Life.
- Kanoniczna roadmapa OCAF: 334 etapy główne, 64 techniczne i 10 produktów Builder, `production_approved=false`. OM Engine jest przyrostem istniejącego etapu **0.56**; nie tworzy duplikatu historycznego etapu ani nie podwyższa statusów Builderów.

## 2. Cel architektoniczny

OM Engine ma być **warstwą wykonawczą sterowania i koordynacji** agentów, a nie monolitem, który zastępuje istniejący runtime. W przyszłości system ma spinać planowanie, rozumowanie, wiedzę, pamięć, wykonanie, weryfikację, krytykę, bezpieczeństwo i telemetrię. Granice uprawnień pozostają w OCAF. Digital-Life dostarcza w przyszłości wybrane funkcje kognitywne przez wersjonowane adaptery, nie przez współdzielony dostęp do tabel SQLite.

```text
Agents Builder (Code/NoCode, lokalnie/chmura)
          | typed graph IR / manifest / verified subject
          v
OM Engine API (zaufany ingress, jeszcze niezaimplementowany)
          |  tenant + role + bounded DAG
          v
OM Engine Durable Coordinator [NOWY]
   | SQLite IMMEDIATE journal, replay, SHA-256 chain
   | invokes existing OmniDevWorkflow (no duplicate scheduler)
   +--> OCAF policy/OMAs/OMAt/OSTc (separate, no bypass)
   +--> OCAF WASI/Firecracker executor (separate, no direct call here)
   +--> OCAF model router / compound reasoning (separate)
   +--> Knowledge Envelope v1 (INERT DATA ONLY)
                     |
                     v
             Future Digital-Life adapter
             OTCM / CRDT / Knowledge / NARS / JEPA / SAE
```

## 3. Struktura plików

```text
ocaf-runtime/
├── src/
│   ├── lib.rs                    # re-eksport `pub mod om_engine;`
│   ├── om_engine.rs              # trwały koordynator, event journal, replay, bridge DTO
│   ├── omnidev_workflow.rs       # istniejący scheduler — bez duplikacji
│   ├── builder_core.rs           # istniejący typed graph IR
│   ├── stage044_omat_authz.rs    # istniejąca bramka uprawnień narzędzi
│   └── model_router.rs           # istniejący routing modeli
├── tests/
│   └── om_engine.rs              # E2E SQLite, tenant isolation, replay, fencing, corruption
├── docs/
│   └── om-engine/
│       └── ARCHITECTURE.md       # niniejszy dokument
└── .github/workflows/
    └── om-engine.yml             # dedykowana bramka kompilacji/testów
```

**Planowane odrębne moduły po bramkach:** `adapters/omas`, `adapters/omat`, `adapters/digital_life`, `checkpoint`, `policy`, `telemetry`, `planner`, `critic`, `rag`, `graph`, `sandbox`. Nie oznacza to ich implementacji.

## 4. Kontrakty runtime i algorytm

### 4.1 Rejestracja workflow

`OmEngine::register(trusted_tenant, WorkflowContract)` waliduje kontrakt w istniejącym `OmniDevWorkflow::new`, odmawia obcego tenanta, zapisuje kanoniczną reprezentację JSON wraz z SHA-256. Identyczny ponowny zapis jest idempotentny; modyfikacja kontraktu o tym samym identyfikatorze jest odrzucana.

### 4.2 Zdarzenia i checkpointy

`claim`, `complete` i `reconcile` wykonują `BEGIN IMMEDIATE`, odtwarzają scheduler z zatwierdzonych zdarzeń, weryfikują sekwencję, prev-hash i hash zdarzenia, wywołują istniejący kontrakt, dopisują zdarzenie i atomowo zatwierdzają transakcję. **Nie utrzymują niezapisanego stanu w RAM**. To pozwala odzyskać lease i fencing po restarcie procesu oraz zapobiega rozdźwiękowi RAM/SQLite po błędzie commit.

`claim` wymaga unikalnego, rosnącego epoch z zaufanego koordynatora. `complete` wymaga zgodnego attempt, epoch, roli, terminu i dowodu. Powtórne identyczne potwierdzenie zwraca `Replay` bez nowego zdarzenia. Nieznany wynik po timeout pozostaje nierozstrzygnięty do decyzji Verifier; `UnknownOutcome` zatrzymuje krok.

`MAX_EVENTS=4096` na workflow ogranicza replay i rozmiar bazy; po osiągnięciu limitu nowe efekty są odmawiane. Wydajności ani RPO/RTO nie oszacowano bez benchmarków. Konieczna jest przyszła polityka snapshot/compaction z podpisanym checkpointem.

### 4.3 SQLite

Tabela `om_workflows`: klucz `(tenant,workflow_id)`, kontrakt JSON, SHA-256. Tabela `om_events`: `(tenant,workflow_id,seq)`, JSON zdarzenia, poprzedni i bieżący SHA-256. WAL, `synchronous=FULL`, foreign keys, busy timeout 5 s. Jedno połączenie chronione `Mutex`, a blokada `BEGIN IMMEDIATE` serializuje również współbieżne procesy na tej samej bazie. **Nie zapewnia to PostgreSQL HA ani rozproszonego quorum.**

### 4.4 Memory Envelope v1

```json
{
  "schema_version": 1,
  "source_engine": "ocaf.om_engine",
  "tenant_id": "tenant_a",
  "workflow_id": "research_001",
  "namespace": "reasoning",
  "record_id": "fact_1",
  "payload": {"source": "MSC2020", "value": "data only"},
  "payload_sha256": "SHA256(serde_json::to_vec(payload)) in lowercase hex"
}
```

`MemoryEnvelope::verify(trusted_tenant)` sprawdza wersję, tenant, format, limit 32 KiB i integralność JSON. Przyjmowane identyfikatory źródła: `ocaf.om_engine` i `digital-life.noevaris`. **Pole `source_engine` jest deklaracją danych, nie tożsamością kryptograficzną.** Weryfikacja nie nadaje prawa do importu, wykonywania promptów, uruchamiania narzędzi ani dostępu do cudzej pamięci.

### 4.5 Docelowy adapter Digital-Life

1. Zewnętrzny ingress uwierzytelnia OCAF subject przez TLS-bound OAuth/OMAs i przypisuje tenant (nigdy z payloadu).
2. Adapter wykonuje mapowanie `tenant/workflow/namespace` na politykę OTCM i sprawdza ACL, limit retencji, klasyfikację danych, pochodzenie, podpis i replay.
3. `MemoryEnvelope v1` przenosi tylko inertny JSON. Cztery czasy OTCM (`valid`, `observed`, `recorded`, `consolidated`) i CRDT dot/clock wymagają osobnych, wersjonowanych rozszerzeń schematu — nie należy dopowiadać ich wartości.
4. Bramka waliduje kompatybilność wersji, deduplikację i idempotency key, a dopiero potem zapisuje do API Digital-Life. **Brak bezpośredniego dostępu do bazy Digital-Life**.
5. W przypadku błędu upstream: deny, retry wyłącznie dla operacji bezpiecznie idempotentnych, DLQ i telemetryka; wynik nieznany wymaga reconciliation.

## 5. Model zagrożeń

| Zagrożenie | Obecna kontrola | Pozostała luka |
|---|---|---|
| Cross-tenant access | Klucze tenant/workflow, trusted tenant w API, testy negatywne | Weryfikacja tożsamości ingress poza modułem |
| Replay i double spend | Idempotentny `complete`, epoch/attempt/lease, SQLite transaction | `claim` po utracie odpowiedzi wymaga odczytu statusu i reconciliation |
| Crash/commit divergence | Replay z transakcyjnego dziennika | Backup i cross-host disaster recovery |
| Manipulacja dziennikiem | Hash-chain i SHA kontraktu, fail-closed | Brak MAC/podpisu i niezależnego anchoringu; DBA może przepisać cały łańcuch |
| Rollback bazy | Weryfikacja lokalnego łańcucha | Wymagany zewnętrzny high-water mark / podpisany checkpoint |
| Prompt/tool injection | Pamięć jako inertny JSON, brak wykonawcy | Przyszły parser/tool gateway wymaga oddzielnych policy gates |
| Resource exhaustion | 4096 eventów, 32 KiB payload, 128 KiB kontrakt, bounded scheduler | Brak load testów i quota globalnej liczby workflow |
| Symlink state file | Odrzucenie istniejącego symlinku pliku DB | Symlinki katalogów nadrzędnych, filesystem hardening |
| TOCTOU / fencing | SQLite serializacja + epoch w schedulerze | Wielohostowe fencing i rozproszona kontrola lease |

## 6. Testy i dowody wymagane do odbioru

- `cargo test --locked --no-default-features --test om_engine`
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --no-default-features -- -D warnings`
- `cargo test --locked --no-default-features` (regresja runtime)
- GitHub Actions na **konkretnym SHA** i odczyt statusu końcowego.
- Niezależne: test restartu procesu z utratą odpowiedzi, crash injection w SQLite, wieloprocesowa konkurencja, test na prawdziwym gateway OAuth/mTLS, audyt uprawnień i operator acceptance.

**Zakres testów dodanych w tym przyroście:** restart/replay, idempotencja receipt, izolacja tenantów, kontrakt immutable, nierozstrzygnięty timeout, kontrola roli Verifier, wykrywanie modyfikacji SQLite, integralność MemoryEnvelope, limit payloadu.

## 7. Kryteria wdrożenia

`IMPLEMENTED` = kod i testy obecne; `TESTED_LOCAL` = uruchomione testy; `CI_VERIFIED` = zielony przebieg przypięty do SHA; `INTEGRATED` = prawdziwe połączenie OMAs/OMAt/Builder/Digital-Life; `DEPLOYED` = rzeczywiste środowisko i health checks; `PRODUCTION_APPROVED` = formalny odbiór. Stan jednego poziomu nie implikuje kolejnego. Nie należy deklarować produkcyjnej autonomii, ASI, formalnej weryfikacji, Firecracker/KVM ani pełnej zgodności Digital-Life na podstawie tego przyrostu.

## 8. Zweryfikowany przyrost CI (2026-11-03)

- Kod przypięty do SHA `c7e4aa5dc4158fd825dc949a35161cb1b5231b19`.
- [OM Engine CI #41578989418](https://github.com/mojealterego/ocaf-runtime/actions/runs/41578989418): `completed/success`.
- `cargo test --locked --no-default-features --test om_engine`: **5/5 PASS**.
- `cargo fmt --all -- --check`: **PASS**.
- `cargo clippy --locked --no-default-features --all-targets -- -D warnings`: **PASS**.
- Status: `TESTED_CI_PARTIAL`, nie `INTEGRATED`, `DEPLOYED`, `PRODUCTION_APPROVED` ani `EXTERNALLY_AUDITED`.


## Checkpoint zewnętrzny i wykrywanie rollbacku (przyrost 2026-11-03)

`OmEngine::audit_head(tenant, workflow_id)` zwraca teraz serializowalny `AuditHead` zawierający identyfikator tenanta, workflow i oba łańcuchy SHA-256 (journal i scheduler). Zaufany operator może zapisać ten punkt kontrolny w niezależnej domenie zaufania, np. podpisanym rejestrze dowodowym, i porównać go później z lokalnym stanem przez `verify_audit_anchor(...)`. Porównanie jest dokładne: wykrywa zarówno zmianę, jak i **ucięcie końcowych zdarzeń**, nawet jeśli pozostały prefiks łańcucha jest poprawny.

Kontrakt **nie jest podpisem kryptograficznym ani automatycznym atestowaniem**. OCAF nie przechowuje tu zaufanego klucza, nie dostarcza zewnętrznego storage i nie wymusza atestacji przy `claim`/`complete`. Samo `verify_audit_anchor` jest tylko odczytem — między weryfikacją a nowym zapisem możliwa jest zmiana stanu (**TOCTOU**). W przyszłym integratorze Digital-Life wymagane będzie uwierzytelnienie tożsamości peera, wersjonowany kontrakt i transakcyjna kontrola wymaganego high-water mark podczas mutacji. Nie wolno przyjmować pola `AuditHead` dostarczonego przez model jako zaufanego dowodu.


**Dowód przyrostu high-water mark:** [GitHub Actions OM Engine #41586244060](https://github.com/mojealterego/ocaf-runtime/actions/runs/41586244060), SHA kodu `c62e0f013cb355bb02dca6ac90b16e71e3e56088`, `completed/success`: testy Rust 6/6, rustfmt, Clippy PASS. Nie jest to certyfikacja ani produkcyjny connector Digital-Life.

## 2026-11-08 — OM Architect / DGM–RSI / AlphaEvolve: kontrakt rozszerzenia (SPECIFIED, NOT IMPLEMENTED)

**Provenance:** materiały dostarczone przez operatora: *AlphaEvolve: A coding agent for scientific and algorithmic discovery* (Google DeepMind, 2025), *Darwin Gödel Machine: Open-Ended Evolution of Self-Improving Agents* (arXiv:2505.22954v3, ICLR 2026), `DGM_RSI_OMEGA_PRODUCTION_BLUEPRINT_v1.0.md`, `DGM_RSI_OMEGA_SECURITY_VALIDATION_AND_RELEASE_v1.0.md` oraz dwie wersje *Architektura Autonomicznego Systemu DGM*. Dokumenty są **materiałem wejściowym**, nie dowodem wykonania. Dwa dokumenty DGM–RSI Ω pierwotnie opisują **oddzielny projekt**, dlatego ich wymagania nie stają się automatycznie istniejącą implementacją OCAF.

### Decyzja architektoniczna i kolejność zależności

1. **OM Evolution Core (przed integracją Builder):** wielopokoleniowe archiwum kandydatów DGM, selekcja jakości/różnorodności, lineage CAS, propozycje modyfikacji agentów i harnessu (RSI), automatyczne mierzalne ewaluatory w stylu AlphaEvolve, deterministyczne seed, baseline i holdout. AlphaEvolve oznacza tu **inspirowany publikacją wzorzec**, nie wykorzystanie zamkniętego produktu Google.
2. **OM Architect:** kontrolowany planner, fabryka agentów z opisów tekstowych i głosowych, resolver katalogu, router zadań, delegacja do agentów/rojów, weryfikator oraz interfejs do OMAt/OMAs. Mowa wymaga osobnych rzeczywistych adapterów STT/TTS i zgód mikrofonu.
3. **Market Intelligence:** cotygodniowy job pobierający dane o popycie, konkurencji, niszach i rentowności. Dane muszą mieć źródło, datę, metodę, koszty i przedziały niepewności. Prognoza rentowności nie jest przychodem. Job jest zewnętrznie schedulowany, z ograniczeniami egress i budżetu; sam wpis w dokumencie nie uruchamia go.
4. **Agents Builder:** siedem aplikacji docelowych Windows, Android, macOS, iOS, Linux, KaiOS, HarmonyOS z Code/NoCode, przełącznikiem Cloud (gdzie wspierany) oraz nowym Architect Chat/Voice. Istniejące historyczne **10 rekordów v11** pozostaje nienaruszone; rozszerzenie platform wymaga odrębnej, addytywnej matrycy odbioru. KaiOS i HarmonyOS wymagają jawnej analizy ograniczeń natywnych API i realnych artefaktów.
5. **Poligon:** benchmarki agentów i rojów, adversarial testing, fault injection, replay/recovery, podpisane dowody i rozdzielony admission/release.

### Granice uprawnień i samoczynne aktualizacje

Operator wymaga, aby Architekt sam aktualizował moduły aplikacji bez każdorazowej interakcji. Realizuje to **wcześniej zdefiniowana, wersjonowana polityka delegowanej autoryzacji**, nie możliwość nadawania sobie uprawnień. Policy kernel, podpisy, tożsamość release authority, mechanizm audytu, uprawnienia, sekret store i niezależny evaluator są poza modyfikowalną powierzchnią DGM. Brak potwierdzonej izolacji => fail closed. Kandidat nie otrzymuje host filesystem, produkcyjnych tokenów, bezpośredniego internetu ani możliwości edycji własnych bramek. Internet zapewnia oddzielny allowlisted research gateway, a nie sandbox wykonujący kod kandydata.

Dopuszczalne bez interakcji: pobranie publicznych danych, sporządzenie propozycji, mutacja w odizolowanym workspace, ewaluacja, zapis do append-only archive, budowa/testy agentów i publikacja do **STAGED**. Automatyczne aktywowanie zmian jest dopuszczalne wyłącznie dla wcześniej zatwierdzonego zakresu reversible, przy ważnym delegowanym grancie, niezależnych testach, podpisie release authority, ograniczeniu blast radius, health gates i działającym rollback. Zmiany uprawnień, polityk bezpieczeństwa, sekretów, zewnętrznych płatności, destrukcyjnych migracji i nieodwracalnych skutków nie należą do tej delegacji.

### Kontrakt kandydata i cykl ewolucji

- Wejście: `tenant_id` z zaufanego ingress, `candidate_id`, `parent_digest`, `source_tree_sha256`, `policy_sha256`, `evaluation_bundle_sha256`, `seed`, `max_generations`, `max_cost_minor`, `deadline`, `allowed_paths`, `allowed_tools`, `risk_class`.
- Przejścia: `PROPOSED -> ADMITTED -> SANDBOXED -> EVALUATED -> ARCHIVED -> STAGED`; `QUARANTINED` po naruszeniu. `ACTIVE` wymaga osobnej decyzji release authority, nawet przy delegowanym trybie auto-update.
- Ocena: correctness, regression, security, latency, resource/cost, novelty, quality-diversity i holdout; scorecards podpisane przez **niezależny** evaluator. Odrzucone mutacje nie mogą nadpisywać archiwum. Ogranicz wielokrotne próby holdout i przeciek benchmarku.
- Niezmienniki: `candidate.mutable_paths ∩ {policy,evaluator,release,signer,secrets}=∅`; brak zwiększenia uprawnień i egress; dowody związane z pełnym tree digest; idempotent release; fenced epoch; rollback do znanego digest; izolacja tenantów; koszty i deadline twarde, bez nieograniczonej rekurencji.
- Negatywne testy obowiązkowe: prompt injection w danych rynku, symlink/path traversal, SSRF/cloud metadata, nieautoryzowany egress, fałszywe wyniki benchmarku, modyfikacja scorecard, próba zmiany policy, wyciek sekretu, cross-tenant, replay podpisu, wyczerpanie budżetu, fork bomb, utrata lease, awaria w połowie rollout, nieudany rollback i brak cleanup.
- Minimalny dowód: commit SHA, source tree digest, pełny fixture digest, policy digest, środowisko, polecenia i logi testów, liczby prób, wyniki negatywne, ocena niezależnego weryfikatora, release receipt i status operatora.

### Kryteria odbioru i aktualny stan

**SPECIFIED / NOT IMPLEMENTED** dla DGM–RSI–AlphaEvolve w OM Engine, Architect Chat/Voice, Market Intelligence, samoczynnego release i integracji siedmiu Builderów. `src/om_engine.rs` pozostaje trwałym koordynatorem workflow, a nie implementacją RSI. Brak nowych buildów, CI ani benchmarków na podstawie tego dodatku. Wymagane są osobne cykle RED/GREEN i testy na konkretnym SHA przed zmianą statusu. Wszelkie historyczne etapy 0.50–0.56 i manifest v11 zachowują swoje znaczenie.
