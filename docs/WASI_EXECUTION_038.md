# OCAF — etap 0.38: ograniczony adapter WASI Preview 1 (Linux)

**Stan techniczny: częściowa implementacja, wymagany końcowy CI i ocena
zewnętrzna. Nie jest to zatwierdzony sandbox produkcyjny.**
Kod: `src/executor/wasi.rs`; testy: `tests/wasi_sandbox.rs`;
realny harness CI: `scripts/wasmtime_acceptance.sh`.

## Decyzja architektoniczna (ADR-038)

Zamiast automatycznie zmieniać `Cargo.lock` i ładować niezaufane
moduły do przestrzeni adresowej `ocaf-coordinator`, oddzielny
`WasmToolExecutor` uruchamia **w osobnym procesie** operatorowi znany
`wasmtime run`. Plik wykonywalny wskazuje operator jako bezwzględną
ścieżkę wraz z dokładnym SHA-256; sprawdzamy regularny plik
(nie symlink), flagi execute i digest przed każdą operacją.
W CI pobierany jest wyłącznie artefakt upstream Wasmtime **49.0.2
Linux x86_64** z przypiętym SHA-256 archiwum
`a4d6e9e3a5a60f527cf7793d674c48930c80c2e8977995b8a275cad3254b9322`
z oficjalnego wydania. Po walidacji archiwum uruchamiany jest
rzeczywisty test Wasm/WASI (a nie sam shell mock).

To nie ustanawia niezmiennego binarium bez ochrony uprawnień
do ścieżki i jej katalogów nadrzędnych. Operator musi przechowywać
Wasmtime i jego zależności w katalogu chronionym przed
niezaufanym zapisem. Wymagana jest też okresowa ocena CVE/SA,
zamiana digestu po zatwierdzonej aktualizacji oraz kontrola
sygnatur i SBOM upstream — SHA-256 nie uwierzytelnia *wydawcy*
bez wcześniejszego zaufania do wzorcowego digestu.

## Polityka i granica możliwości

- Akceptowane są tylko *core wasm binary* (`\0asm\x01\0\0\0`),
  o wielkości co najwyżej 1 MiB, operatorowo zatwierdzonym SHA-256.
  Bezpieczne parsowanie import section odrzuca obce moduły i importy
  niebędące funkcjami; pełne sprawdzenie składni następuje dopiero
  w zewnętrznym Wasmtime.
- Biała lista importów: `wasi_snapshot_preview1`:
  `fd_read`, `fd_write`, `fd_fdstat_get`, `proc_exit`.
  Brak `path_open`, pozostałych `path_*`, `sock_*`,
  `random_get`, `clock_time_get`, WASIp2, dowolnych host extensions.
- Każde wywołanie tworzy nieprzewidywalny, prywatny
  katalog `/tmp/ocaf-wasi-<uuid>` z trybem `0700`,
  zapisuje moduł `0600` i usuwa go po zakończeniu;
  dane wejściowe nie są zapisywane na dysk.
- Wasmtime startuje z `env_clear`, bez `--dir`, `--env`,
  `--tcplisten`, `serve`, socket capability i hostowego FS.
  Żaden parametr CLI nie pochodzi z niezaufanego opisu agenta.
- Interfejs: JSON na stdin, JSON na stdout. Limit domyślny
  16 KiB/strumień; sztywne maksimum 64 KiB; błędy nie zwracają
  stderr gościa do wywołującego.
- Domyślnie: 2 mln `fuel`, maksymalnie 50 mln, 32 MiB
  pamięci liniowej (sztywny limit 64 MiB),
  stos Wasm 1 MiB (sztywny 2 MiB),
  2 sekundy wall-clock (sztywny limit 10 s),
  4 równoczesne instancje (sztywny limit 32).
  Na timeout lub błąd limitu wynik jest odrzucany,
  proces potomny ma `kill_on_drop`.
- `ToolExecutor` jest adapterem warstwy istniejącej:
  `OcafRuntime` zachowuje wcześniejszą walidację JSON Schema,
  płatności/ACL w `AuthPolicy` i budżet. Nie zastępuje
  *autorytatywnego* PostgreSQL RPC fencing, leader lease ani
  idempotencji efektów zewnętrznych.
- Profil deterministyczny jest ograniczony do wejścia/wyjścia
  i braku importów zegara/losowości, dla stabilnej wersji silnika.
  Nie stanowi formalnego dowodu deterministycznej inferencji.

## Testy implementacji

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features --test wasi_sandbox
bash scripts/wasmtime_acceptance.sh
```

`tests/wasi_sandbox.rs` zawiera pozytywny scenariusz
rejestracji narzędzia w `OcafRuntime`, test odmowy
niebezpiecznych importów, zmian digestu binarium/modułu,
błędów JSON, output bomb, nadmiaru wejścia i timeout.
Realny Wasmtime CI uruchamia WebAssembly z importem
`fd_write`; testuje też powtórzenie wyniku, pętlę
bez zakończenia (paliwo) i moduł z nadmierną pamięcią.
Test z użyciem shell mock służy tylko kontroli
*adaptera subprocess*, a nie za dowód izolacji WASI.
Wszystkie wyniki liczą się jako PASS dopiero na
**ostatecznym commicie** i po sprawdzeniu GitHub CI.

## Przykład integracji (API)

Operator dostarcza zatwierdzony plik `wasmtime` oraz
oczekiwane `[u8; 32]` digests. Aplikacja:

1. Odczytuje zatwierdzony moduł Wasm (limit rozmiaru).
2. Tworzy `PinnedWasmtime::new(absolute_path, engine_sha)`.
3. Tworzy `WasmToolExecutor::new(engine, module, module_sha,
   SandboxPolicy::default())`.
4. Rejestruje wynik w `OcafRuntime::register` z
   **wyłącznie operatorowo zatwierdzonym** `ToolManifest`;
   w szczególności `permissions`, `cost_sats`,
   `timeout_ms` i `enabled` nie pochodzą z promptu.
5. Uruchamia wywołanie przez `OcafRuntime::execute`.
   Żadne niezaufane API nie powinno konstruować `SandboxPolicy`
   ani `PinnedWasmtime`.

Wystąpienie błędu modułu nie może skutkować fallbackiem do
lokalnego uprzywilejowanego `ToolExecutor`.

## Brakujące dowody i bloker produkcyjny

Ten etap dostarcza **ograniczony Linux/WASIp1 execution adapter**,
ale nie wdraża jeszcze:

1. Podpisanych przez zaufanego wydawcę manifestów/łańcucha
   autoryzacji do automatycznego wyboru modułu.
2. Polityki `tenant -> agent -> digest -> capability` w
   zdalnym rejestrze MCP i migracji workerów przez nową granicę
   wykonania (0.38.7).
3. Cgroups v2 / namespace / seccomp / MAC, twardego
   OS-owego limitu RSS ani ochrony przed błędami silnika
   Wasmtime (plan 0.39: broker Firecracker/eBPF).
4. Load/soak, destrukcyjnego chaos, zewnętrznych testów
   eksfiltracji hosta, rotacji sygnatur, threat-review na
   wielu hostach, potwierdzenia odporności supply-chain.
5. Podpisanego `ExecutionReceipt`, bezpośredniego
   propagowania pełnej tożsamości po TLS-bound RPC oraz
   globalnych rozliczeń tenantów.

**Decyzja: 0.38 = IMPLEMENTED PARTIALLY / REVIEW REQUIRED;
production = NO-GO.** Nie awansować do 0.39 jako „0.38 DONE”
bez podpisania brakujących bramek i wyników CI.
