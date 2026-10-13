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

## Podpisany manifest i izolacja tożsamości — aktualizacja

`src/executor/wasi_manifest.rs` wdraża **Ed25519**: niezależny
zaufany klucz publiczny wydawcy wprowadzany przez operatora,
podpis odłączony nad dosłownymi bajtami
`OCAF-WASI-MANIFEST-V1\\0 || signed-json`. Powiązane pola to
`schema_version=1`, `tenant_id`, `agent_id`, `tool`
(łącznie z JSON Schema, ceną, ACL i limitem czasu),
`module_sha256_hex`, `expires_unix_secs` i pełna
`SandboxPolicy`. Parser JSON odrzuca nieznane pola manifestu;
podpis, czas ważności, SHA-256 modułu i polityka
są sprawdzane *przed* startem procesu oraz ponownie
przed każdym wykonaniem.

Podpisane manifesty wykluczają `cost_sats > 0`,
niepuste `permissions` i `enabled=false`; są przeznaczone
wyłącznie do narzędzi nieuprzywilejowanych. Nadal istnieje
jawny konstruktor `WasmToolExecutor::new` dla ręcznie
zatwierdzonych lokalnych uruchomień; tylko ścieżka
`OcafRuntime::register_signed_wasi` dopuszcza import z
podpisanego manifestu w usługach sieciowych.

`OcafRuntime::with_tenant_namespace` ustala przestrzeń
dzierżawcy wyłącznie z zaufanej konfiguracji. Po sprawdzeniu
certyfikatu sesji TLS, PoP, HMAC i replay handler RPC przekazuje
uwierzytelniony sender do
`execute_authenticated_peer`. Executor dopuszcza tylko
dosłowną zgodność `tenant:agent` z podpisanym manifestem.
Nieznany tenant lub anonimowe `OcafRuntime::execute`
zwracają odmowę w podpisanej ścieżce.
Dla workerów zdalnych tożsamość wykonawcy wyprowadzana
jest z identyfikatora lokalnego po weryfikacji dzierżawy.

## Podłączanie podpisanego narzędzia do sieci

Wspólny, operatorowy loader:
`src/ops/wasi_config.rs`; komponenty korzystające:
`ocaf-coordinator` oraz `ocaf-rpc-node` w trybie
`OCAF_ROLE=worker`. Konfiguracja wymaga:

| Zmienna środowiskowa | Zawartość |
|---|---|
| `OCAF_RPC_INGRESS` | dokładnie `tls-bound` |
| `OCAF_REQUIRE_PEER_CERT_POP` | `1` |
| `OCAF_TENANT_NAMESPACE` | zatwierdzona nazwa dzierżawcy |
| `OCAF_WASI_MANIFEST_FILE` | ścieżka absolutna do JSON z podpisaną polityką |
| `OCAF_WASI_SIGNATURE_FILE` | ścieżka absolutna do 64-bajtowego podpisu Ed25519 |
| `OCAF_WASI_PUBLISHER_KEY_FILE` | ścieżka absolutna do PEM klucza publicznego wydawcy |
| `OCAF_WASI_MODULE_FILE` | ścieżka absolutna do zatwierdzonego modułu .wasm |
| `OCAF_WASMTIME_EXECUTABLE` | ścieżka absolutna do binarnego Wasmtime |
| `OCAF_WASMTIME_SHA256` | 64 znaki lowercase SHA-256 binarium |

Wszystkie pliki manifestu, podpisu, modułu i PEM muszą być
własnością operatora, mieć uprawnienia tylko dla właściciela
(np. `0600`), nie mogą być symlinkami ani hardlinkami.
Binarium Wasmtime wymaga niezależnego bezpieczeństwa ścieżki,
ochrony katalogów nadrzędnych i SHA-256. Brak któregokolwiek
warunku powoduje odmowę startu, bez Zenoh fallbacku.
Zdalny worker w wariancie TLS używa osobnej sesji mTLS
do *każdego* `Heartbeat`, `ClaimJob`, `ValidateLease`
i `FinishJob` bez automatycznego retry skutków.

Do podpisu manifestu służy
`scripts/sign_wasi_manifest.py --private-key <absolute-key> --manifest <absolute-json> --signature <absolute-output>`.
Klucz prywatny Ed25519 musi pozostawać poza repozytorium
i procesami agentów. Skrypt zapisuje wynik `0600`,
odrzuca duplikaty nazw JSON i nie nadpisuje poprzedniego podpisu.
Klucz wydawcy wymaga odrębnej procedury rotacji
i audytu operatora.

## Kryterium dowodu i podpisany ExecutionReceipt

`src/executor/wasi_receipt.rs` implementuje oddzielny HMAC-SHA256
dla typu `ExecutionReceipt`, zawierającego tenant/agent/tool,
correlation ID, digest modułu, digest wejścia i wyjścia,
czas zakończenia i czas trwania. Podpis przy pomocy
`ReceiptSigner` wystawiany jest wyłącznie po pomyślnym
`execute_with_receipt`; jego weryfikacja odrzuca zmiany pól
i nieprawidłowy MAC. Ta metoda jest **opcjonalnym API
evidence**, nie dowodem semantycznej poprawności wyniku ani
zapisem do trwałego dziennika PostgreSQL. Ostateczny odbiór
wymaga osobnego audytu i kontroli klucza MAC.

Programowe testy obejmują podpis Ed25519 (w tym inne
klucze/tenants, podmienione dane, wygasłe manifesty),
powiązanie tożsamości RPC, rejestrację wyłącznie zatwierdzonych
narzędzi, MAC receipt, limity concurrency, timeout/reaping
oraz prawdziwy silnik Wasmtime; niezależny test TLS worker
weryfikuje heartbeat i pustą kolejkę na tym samym protokole
cert-bound co coordinator.

## Otwarte granice bezpieczeństwa

1. Izolacja WASIp1 nie zastępuje jądrowego sandboxa:
   cgroups/namespace/seccomp i broker Firecracker/eBPF
   pozostają częścią osobnego etapu **0.39**.
2. Wymagane są niezależne testy crash/soak, opóźnień i
   ataków na host, rotacji kluczy oraz ocen
   podatności Wasmtime i dostawców.
3. CLI Wasmtime musi być chroniony przed modyfikacją
   podczas wywołania; sprawdzenie SHA-256 i ACL
   nie jest sprzętową atestacją wykonania.
4. `ExecutionReceipt` jeszcze nie stanowi trwałego
   audytu commitów pracy rozproszonej.
5. Tryb transportu mTLS ma odrębne otwarte blokery
   CRL/OCSP i automatycznej rotacji ze stadium 0.37.
6. Produkcja i rzeczywista niezależna wielohostowa HA
   mają osobne twarde gates etapów 0.39–0.42.

**Status: implementacja 0.38 rozszerzona o podpisy,
tenant identity, TLS worker i receipts. Decyzja o
technicznym zamknięciu zależy od wyników CI na
końcowym commicie. Produkcja: NO-GO.**

## Utwardzenie dostępu do artefaktów operatora — 0.38

Loader `src/ops/wasi_config.rs` otwiera manifest, podpis, PEM wydawcy
i moduł przez deskryptory `O_NOFOLLOW | O_CLOEXEC`, zamiast wykonywać
oddzielne `symlink_metadata(path)` i `read(path)` (co pozwalało na
zamianę pliku pomiędzy sprawdzeniem a odczytem). Waliduje `fstat`
rzeczywiście otwartego inode: zwykły plik, pojedyncze dowiązanie,
właściciel równy effective UID procesu, zakaz uprawnień grupy/innych
oraz ograniczenie rozmiaru przed i podczas odczytu. Testy modułowe
obejmują poprawny plik, symlink, hardlink, tryb 0644, pusty plik i
przekroczenie limitu.

Ta zmiana **nie** blokuje podmiany komponentu katalogu nadrzędnego
ani uruchomienia podmienionego pliku Wasmtime między weryfikacją
SHA-256 a `execve`. Operator musi chronić katalogi instalacyjne,
zasób zaufanych kluczy i samą binarkę silnika. Niezależny audyt
oraz próby izolacyjne nadal są wymagane przed odbiorem etapu 0.38.
