# OCAF — kontrakt wykonawczy etapu 0.38 (przed implementacją)

**Stan na 2026-10-13:** PLAN / NOT IMPLEMENTED. Gałąź: `main` wyłącznie.
**Cel:** bezpieczne uruchamianie niezaufanych narzędzi WebAssembly/WASI
przez osobny adapter, bez bezpośredniego dostępu do zaufanego jądra
OCAF, danych innych tenantów i uprawnień systemu hosta.

## Zweryfikowana baza i zależności

- HEAD obserwowany przy planowaniu: `6bfc77714e6ecb3a5bda65246bbc69641e1ea8f9`.
- Dla tego commitu przebiegi Rust CI `38398928258` i Supply Chain
  `38398928287` mają status `success`; **nie są** testem sandboxa.
- W `src/executor/mod.rs` występuje interfejs `ToolExecutor` i
  `tokio::time::timeout`. Timeout przyszłości Rust nie stanowi izolacji
  wykonywania obcego kodu. `src/registry/mod.rs` waliduje schematy
  wejścia/wyjścia, ale nie izoluje hosta.
- `Cargo.toml` ma wersję `0.26.0` i nie deklaruje obecnie silnika Wasm/WASI.
- Po 0.37 dostępny jest **opt-in** listener TCP/TLS-bound w obrębie
  wspieranych ról; CRL/OCSP, rotacja certyfikatów, pełna migracja
  remote-worker i odbiór multi-host pozostają otwarte.
- Etap 0.39 rezerwuje niezależny broker Firecracker/KVM + eBPF;
  **nie** stanowi ukrytej części 0.38. Mobilne platformy nie
  dziedziczą izolacji KVM.

## Model zagrożeń: wrogie narzędzie jako przeciwnik

Niezaufany moduł próbuje odczytać pliki/secrety środowiska, uzyskać
sieć, uruchomić proces hosta, wykonać nieautoryzowany syscall,
zużyć nieograniczony CPU/RAM, zablokować runtime, zgłosić fałszywy
wynik, przepełnić stdout/stderr, ominąć uwierzytelnienie lub
przekroczyć autoryzowane uprawnienia. Komunikaty i metadane
modułu traktowane są jako dane, nie instrukcje polityki.

**Granica zaufania:** core OCAF pozostaje autorytatywny dla
tożsamości/ACL, kosztów, podpisu manifestów, replay, fencing,
efektów zewnętrznych i dzienników. Silnik Wasm egzekwuje tylko
*ograniczony* profil wykonania modułu. Nie ma prawa sam wystawiać
uprawnień ani bezpośrednio pisać do sinków.

## Kolejność realizacji 0.38

| Podetap | Praca | Kryterium zakończenia |
|---|---|---|
| 0.38.1 | ADR: wybór konkretnej przypiętej wersji Wasmtime/Wasmer, modelu WASI, importów, licencji i powierzchni ataku | Jawne decyzje i zablokowane zależności; audyt licencji/RustSec |
| 0.38.2 | Typowane `SandboxPolicy`, `ModuleIdentity`, `ExecutionReceipt`; oddzielny `WasmToolExecutor` | Brak eskalacji na podstawie opisu JSON/LLM; domyślne deny |
| 0.38.3 | Walidacja i wersjonowanie modułów, digest SHA-256 i podpisane manifesty, ograniczenia imports/exports | Nieznany/zmieniony artefakt oraz nieautoryzowany import są odrzucane |
| 0.38.4 | Brak preopen hostowego FS i brak sieci; nieprzekazywanie env/secrets; jawny katalog dozwolonych host calls | Pozytywne testy na minimalnym dozwolonym module; odmowa przypadkom zabronionym |
| 0.38.5 | Budżety `fuel`/epoch-interruption, zegar deadline, pamięć/stack/tabele, liczba instancji i limit rozmiaru wejścia/wyjścia | Nieskończona pętla, memory bomb i output bomb kończą się ograniczonym błędem bez blokady hosta |
| 0.38.6 | Deterministyczny profil wejścia: zakaz/nadpisanie losowości, czasu, sieci i niedeterministycznych host calls | Powtórzenie identycznego modułu i wejścia ma identyczny wynik w **zadeklarowanym** profilu |
| 0.38.7 | Integracja rejestru i RPC: atrybucja tenant/agent, koszt, scope, replay, fencing i odbiór wyniku | Nieautoryzowane żądanie odrzucone przed startem Wasm; nie ma pominięcia existing guards |
| 0.38.8 | Anulowanie, cleanup, limit współbieżności, odseparowane logi/metryki bez sekretów | Brak przecieków zasobów w testach przerwań, restartów i błędów |
| 0.38.9 | Harness CI: fixtures Wasm, property tests, Linux hardening, negatywne e2e | Uruchamialne testy, nie tylko symulacja; osobny raport ograniczeń |

## Kryteria RED / GREEN / NO-GO

**Negatywne testy wymagane przed deklaracją 0.38:**
1. `fd_read`, preopen `/etc`, `/proc` lub plików sekretów — odmowa.
2. Próba TCP/UDP, DNS i obejścia przez pośrednie hostcall — odmowa.
3. Import API poza allowlist — odmowa instancjonowania.
4. Nieskończona pętla, duża rekurencja, memory.grow i masowe alokacje
   — zakończenie w budżecie, bez destabilizacji koordynatora.
5. Próba nadmiarowego stdout/stderr, wejścia JSON oraz wyniku — bounded.
6. Nieznany digest, podmieniony manifest, inne tenant/agent ID,
   permission escalation i fałszywy token — odmowa przed wykonaniem.
7. Wygaśnięcie deadline, anulowanie po awarii i przerwanie na
   granicy effect-fencing — brak automatycznej powtórki skutku
   niepewnego; audyt zgodny z istniejącym modelem.
8. Moduł wolny od uprawnień wykonuje wyłącznie zatwierdzoną
   transformację i przechodzi walidację output_schema.

**Gates:** `cargo fmt --all -- --check`, `cargo clippy --locked
--all-targets --all-features -- -D warnings`, adekwatne `cargo test
--locked` (w tym nowe izolacyjne), skany supply-chain oraz GitHub CI
dla *końcowego* HEAD po commicie `main`.
Każdy wyjątek dokumentować z reprodukcją, bez deklarowania PASS
na podstawie wcześniejszych commitów.

## Granice gwarancji i otwarte blokery

- WASI sandbox nie jest równoważny izolacji KVM ani systemowi
  bezpieczeństwa dla dowolnego exploita runtime'u Wasm; wymaga
  aktualizacji, hardeningu hosta, audytu i ew. osobnego procesu.
- `fuel` jest licznikiem wykonania Wasm, nie twardym limitem
  procesorowych mikrosekund ani energii. Deadline nie zatrzymuje
  automatycznie kodu natywnego wywołanego przez host function.
- Determinizm dotyczy ściśle zdefiniowanego zestawu importów,
  engine/config i danych; nie obejmuje dowolnego modelu AI.
- Wpływ na hostowe zasoby i granice izolacji należy przetestować
  również przy równoczesnej pracy wielu tenantów.
- Bez ukończenia 0.39–0.42, dowodu fizycznej HA,
  cyklu życia certyfikatów i niezależnego audytu stan to **NO-GO**.

## Pochodzenie zakresu

- `docs/ROADMAP_FULL_ECOSYSTEM_2026.md` (konsolidacja 26 PDF):
  etap 0.38 = WASI, 0.39 = Firecracker/eBPF, 0.40–0.42 = HA/release.
- Trzy dodatkowe rewizje raportu „Kompleksowy Raport
  Badawczo-Architektoniczny: Koncepcja i Mapa Drogowa Systemu
  OCAF oraz JARVIS OMEGA INFINITY” (załączone 2026-10-13)
  wzmacniają wymagania izolacji, lecz nie potwierdzają implementacji.
- „Dokument bez nazwy” (2026-10-13) zawiera demonstracyjne
  koncepcje Frontier; pseudokodu nie wolno uznać za implementację.

**Warunek przejścia do implementacji:** potwierdzony stan HEAD i
zaakceptowany ADR `0.38.1`. Ten dokument jest planem, nie
ukończonym etapem 0.38.
