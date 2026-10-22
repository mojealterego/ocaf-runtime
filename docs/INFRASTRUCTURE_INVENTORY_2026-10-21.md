# Inwentaryzacja infrastruktury OCAF / OMAs / OMAt / OSTc

Data przeglądu: 2026-10-21. Raport obejmuje kod znajdujący się w tym
repozytorium, pliki Cargo i testy. Nazwa elementu w roadmapie albo przykład w
dokumentacji nie jest dowodem wdrożenia.

## Znaczenie statusów

- **IMPLEMENTED** — istnieje kod wykonywalny lub weryfikowalny kontrakt oraz
  testy dla dokładnie opisanego zakresu. Status nie rozszerza się na
  niewykonany backend zewnętrzny.
- **PARTIAL** — istnieje część kontraktu, adapter albo lokalny executor, ale
  brakuje integracji, procesu produkcyjnego, sprzętu lub dowodu E2E.
- **PLANNED** — wymaganie jest zapisane w roadmapie albo dokumentacji, lecz
  nie ma wystarczającego kodu i dowodu w repozytorium.
- **NOT_PRESENT** — podczas przeglądu nie znaleziono biblioteki, loadera,
  serwera lub integracji.

## Mapa elementów

| Element | Status | Co faktycznie istnieje | Pliki i testy | Brakujący dowód / granica |
|---|---|---|---|---|
| OCAF Runtime i lifecycle | PARTIAL | Rust runtime, manifesty, bounded execution, podpisy i bramki lifecycle | `src/runtime/`, `src/security_gate.rs`, `src/executor/`, `tests/runtime_regressions.rs`, `tests/security_gate.rs` | Referencyjny proces wdrożeniowy i niezależny E2E dla całej ścieżki |
| Agents Builder IR / Code↔NoCode | PARTIAL | Typowany DAG, walidacja cykli i typów oraz ograniczony DSL z konwersją | `src/agent_ir.rs`, `src/builder_conversion.rs`, `tests/agent_ir.rs`, `tests/builder_conversion.rs` | Nie jest to IDE, edytor wizualny, kompilator produktu ani dziesięć platformowych Builderów |
| WASI executor | IMPLEMENTED (zakres lokalny) | Bounded WASI Preview 1 adapter z przypiętym zewnętrznym Wasmtime CLI, manifestem, limitami i receipt | `src/executor/wasi.rs`, `src/executor/wasi_manifest.rs`, `src/executor/wasi_receipt.rs`, `tests/wasi_sandbox.rs`, `tests/scripts/verify_wasmtime_fuel_exhaustion.sh` | Wasmtime nie jest zależnością Cargo; adapter nie jest granicą Firecracker/KVM |
| DGM / RSI | PARTIAL | Kontrakty proof-carrying RSI i małe kontrakty dowodów/receipts | `src/proof_carrying_rsi.rs`, `src/omni_verify.rs`, `tests/proof_carrying_rsi.rs`, `tests/omni_verify.rs` | Brak pełnej Darwinian Gödel Machine, autonomicznego research loop, bezpiecznej mutacji i produkcyjnego hot swap |
| RQGM | PLANNED / SOURCE TARGET MISSING | Roadmapa i workspace odwołują się do `crates/ocaf-cognition-rqgm`, lecz lokalny symlink `crates -> /tmp/ocaf-crates` nie ma obecnie celu | `Cargo.toml`, `docs/ROADMAP_FULL_ECOSYSTEM_2026.md` | Brak dostępnego crate, silnika RQGM, trwałego grafu poznawczego, orkiestracji i dowodu wielowęzłowego |
| AST / typed IR | PARTIAL | Walidowany graf typowany i ograniczone kontrakty parserowe | `src/agent_ir.rs`, `src/patch_guard.rs`, `tests/agent_ir.rs`, `tests/patch_guard.rs` | Brak ogólnego parsera AST repozytorium i pełnej analizy semantycznej Rust |
| OmniMath / Lean / SMT / ImandraX | PARTIAL contract / NOT_PRESENT backend | `omni_math` przyjmuje niezależnie zweryfikowany artefakt i odrzuca sam konsensus LLM; `omni_verify` sprawdza skończony ślad | `src/omni_math.rs`, `src/omni_verify.rs`, `tests/omni_math.rs`, `tests/omni_verify.rs`, `docs/roadmap/missing_capabilities_v11.json` (INV-020) | Brak Lean kernel, Z3, cvc5, ImandraX i realnego solver E2E. Enum backendu nie oznacza obecności solvera |
| Firecracker / KVM | PARTIAL contract-only | Walidacja requestu i fail-closed gate wymaga faktów KVM, guest boot, teardown i digestów | `src/firecracker_ebpf_contract.rs`, `tests/firecracker_ebpf_contract.rs`, `docs/evidence/stage219/firecracker_ebpf_contract.md` | Moduł nie uruchamia VM ani Firecracker; brak hosta KVM, obrazu guest, jailera i produkcyjnego dowodu |
| eBPF / Aya | PARTIAL contract-only | Typowany kontrakt dowodu programu i mały `ProbeEvidence` | `src/firecracker_ebpf_contract.rs`, `crates/ocaf-telemetry-ebpf/src/lib.rs`, `tests/firecracker_ebpf_contract.rs` | Brak Aya/aya-ebpf w Cargo.lock, loadera, programu kernel eBPF i attach/teardown E2E |
| Zenoh transport | PARTIAL | Zenoh 1.10 z lokalnymi security backportami, konfiguracja TLS/mTLS, RPC i testy transportowe; lokalny SHM/CRDT jest bounded | `Cargo.toml`, `vendor/zenoh-*`, `src/transport_policy.rs`, `src/rpc/`, `src/zenoh_crdt.rs`, `tests/transport_policy.rs`, `tests/zenoh_*` | `zenoh_crdt` nie jest dowodem sesji SHM; potrzebny powtarzalny multi-node test z awarią, duplikatem i opóźnieniem |
| MCP | IMPLEMENTED (stateless admission) / PARTIAL (system) | Wersjonowany, tenant-scoped control plane waliduje snapshot, schematy i uprawnienia | `src/mcp_control.rs` (unit tests), `src/registry/mod.rs` | Moduł nie przechowuje ani nie wykonuje narzędzi; brak pełnego zewnętrznego MCP server, OAuth consent i connector E2E (INV-018) |
| L402 / market | PARTIAL | Trwały SQLite ledger, podpisane akcje, replay/limity/manual approval oraz bounded macaroon/ZKP | `src/market_l402.rs`, `src/l402_zkp.rs`, `tests/market_l402.rs`, `tests/l402_zkp.rs` | Kod jawnie nie jest payment processor; brak Lightning invoice/preimage, zewnętrznego settlement i audytu operatora. ZKP wymaga audytowanej biblioteki/krzywej |
| OMAs — serwer agentów | PARTIAL | Namespace/tenant admission, podpisane operacje, replay DB, RPC i tożsamość | `src/omas_policy.rs`, `src/omas_signed.rs`, `src/rpc/`, `tests/omas_dispatch.rs`, `tests/persistent_replay.rs` | Brak odrębnego binarium/procesu nazwanego OMAs z rejestrem, sesją, izolacją tenantów i pełnym E2E |
| OMAt — serwer narzędzi | PARTIAL | Registry manifestów, JSON Schema, typed tool DAG i WASI executor | `src/registry/mod.rs`, `src/typed_tool_dag.rs` (unit tests), `src/executor/`, `tests/wasi_sandbox.rs` | Brak osobnego katalogu/procesu OMAt, lifecycle narzędzia i testu zdalnego nieuprawnionego wywołania |
| OSTc — dostęp do źródeł | PLANNED / DISABLED | Agent reference wymusza `ostc_enabled=false`, `source_access=disabled`, a `ACTIVE` jest odrzucane także przy fałszywym readiness | `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/src/lifecycle.rs`, `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/src/security.rs`, `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/tests/security.rs`, `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/config/agent.toml`, `docs/evidence/AGENT_PACKAGE_E2E_2026-10-21.md` | Brak specyfikacji OSTc, szyfrowania, envelope/key rotation, recovery i niezależnych testów kryptograficznych; nie wolno twierdzić, że źródła są obecnie zaszyfrowane |

## Biblioteki i zależności

### Obecne w Cargo

- `zenoh = 1.10` (opcjonalne feature TCP/TLS) oraz lokalne patche
  `vendor/zenoh-transport`, `vendor/zenoh-config`,
  `vendor/zenoh-link-commons`, `vendor/zenoh-link-tls`;
- `tokio`, `tokio-postgres`, `native-tls`, opcjonalne `rustls` i
  `tokio-rustls`;
- `rusqlite` z feature `bundled`, `openssl`, `serde`, `serde_json`, `jsonschema`,
  `json5`, `hmac`, `sha2`, `base64`, `uuid`, `fs2`, `libc`;
- `wasmtime` nie występuje jako pakiet Cargo. Adapter WASI przyjmuje ścieżkę
  do osobno zainstalowanego, przypiętego digestem executable.

### Nieobecne albo wymagające osobnego wdrożenia

W `Cargo.toml`/`Cargo.lock` nie znaleziono `aya`, `aya-ebpf`, Firecracker,
Tree-sitter, Z3, cvc5, Lean, ImandraX, Automerge/Yrs ani eksportera OTLP.
Enumy i typy opisujące te backendy są kontraktami wejścia/wyjścia; nie są
zależnościami ani dowodem ich uruchomienia.

## 33 roje i 111 osobnych agentów

`docs/agents/swarm_groups_v1.json` opisuje 33 roje operacyjne. Rejestr 111 osobnych
pakietów (`docs/agents/agent_registry_111.json`) wskazuje własny katalog, manifest
i polityki każdego agenta. Pakiety są strukturalnie odrębne, ale nie tworzą jeszcze
111 procesów runtime. W repozytorium znajduje się zweryfikowana paczka
referencyjna `OCAF-AGENT-001`, która kończy ścieżkę na `STAGED`; nie ma dowodu
aktywnego agenta ani 111 gotowych binariów. Każdy wpis można podnieść do
`IMPLEMENTED` dopiero po własnym manifeście, buildzie, digestach, testach i
artefakcie instalacyjnym.

## Minimalne kryteria zamknięcia braków

1. Dla każdego backendu zapisać wersję, źródło, digest i proces uruchomieniowy.
2. Dodać test pozytywny i negatywny dla granicy tenant/auth/replay oraz test
   E2E w środowisku docelowym.
3. Dla Firecracker/eBPF dostarczyć host KVM, guest image, loader Aya, attach,
   teardown i podpisany receipt.
4. Dla Lean/SMT/ImandraX dostarczyć niezależny checker i dowód jego wykonania;
   `CertifiedProof` bez takiego checkera pozostaje wejściem kontraktu.
5. Dla OMAs i OMAt zbudować osobne procesy, manifesty usług, health checks,
   rotację kluczy i test izolacji tenantów.
6. OSTc wdrażać dopiero po pełnej specyfikacji: szyfrowanie źródła,
   envelope keys, rotacja, recovery, revocation i testy fałszywej gotowości.

## Zakres dowodów testowych

Zestaw testów kontraktowych obejmuje wymienione pliki `tests/`. Wcześniejszy
raport w `docs/evidence/AGENT_PACKAGE_E2E_2026-10-21.md` potwierdza paczkę
OCAF-AGENT-001 i odmowę `ACTIVE` z `OstcDisabled`. Wynik workspace
`cargo test --workspace --no-default-features --lib --tests` (559 testów,
0 błędów) jest dowodem testów Rust dla tego zakresu, a nie dowodem obecności
zewnętrznych backendów, sprzętu ani produkcyjnego E2E.
