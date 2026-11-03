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
