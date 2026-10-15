# OCAF — audyt dokumentacyjny 20 PDF (15.10.2026)

Repo: `mojealterego/ocaf-runtime`; gałąź `main`.

## Metodyka i zakres

Przeanalizowano tekst wszystkich 20 przesłanych PDF (307 stron łącznie z powtórzeniami), wyodrębniono 17 rodzin wymagań, wykryto 3 grupy plików o identycznej znormalizowanej treści (2, 3 i 2 pliki). Porównano wymagania z aktualnymi plikami `Cargo.toml`, `src/lib.rs`, `ROADMAP.md`, `docs/MASTER_ARCHITECTURE_26_PDFS.md`, `docs/roadmap/missing_capabilities_v11.json` i `docs/ROADMAP_TWO_PDF_RUST_MODULES_2026-10-15.md`.

**Granica weryfikacji:** to audyt dokumentacyjny i przegląd dostępnych dowodów, nie kompletny audyt każdego symbolu w repozytorium. Kontener nie miał dostępu sieciowego do klonowania GitHub. Nie uruchomiono Cargo CI, KVM ani testów trzech hostów. Statusy poniżej nie stanowią certyfikacji produkcyjnej.

## Macierz wymagań PDF → repo → status

| Domena | Dowód / istniejący punkt | Werdykt |
|---|---|---|
| Rust/Tokio runtime | `Cargo.toml`, `src/lib.rs`, `src/runtime` | Implementacja rdzenia istnieje; pełne E2E niezweryfikowane |
| Zenoh TLS/TCP | `Cargo.toml` Zenoh 1.10, `src/transport` | Częściowo; SHM zero-copy i benchmarki niepotwierdzone |
| MCP dynamic tool loading | `src/registry`, INV-018, 0.43–0.44 | Pełne remote discovery/OAuth niepotwierdzone |
| JSON Schema / Tree-sitter | `jsonschema=0.33`; brak Tree-sitter w Cargo | Walidacja częściowo; AST niepotwierdzone |
| OAuth2.1/PKCE | `src/auth`, INV-018 | Niepotwierdzone |
| Firecracker/WASI | 0.39, INV-015 | OTWARTE; brak rzeczywistego guest WASI receipt |
| CRDT Automerge/Yrs | INV-019 | Niepotwierdzone; brak zależności |
| OTLP OpenTelemetry | `tracing`, INV-019 | Tracing częściowo; realny OTLP niepotwierdzony |
| PostgreSQL HA | 0.40, INV-016 | OTWARTE; brak live inter-kernel WAL |
| L402/x402 settlement | dokumentacja 26 PDF | Planowane; brak potwierdzonego ledgeru |
| ZKP/PQC | dokumentacja 26 PDF | Planowane; brak potwierdzonego silnika |
| RAG / wektory | dokumentacja 26 PDF | Niepotwierdzone E2E |
| AGI/neurosymbolic | źródła PDF; backlog badań | Koncepcje, bez dowodów AGI |
| 10 Agents Builder Code/NoCode | 0.50–0.55, INV-001–013 | Brak produktów i platformowego E2E wg rejestru |
| Omni swarms | 0.56–0.89, INV-014 | Planowane/niepotwierdzone |
| OWASP ASI | `src/auth`, `src/rpc` | Częściowe zabezpieczenia; brak pełnej macierzy ataków |
| CI/benchmark | ROADMAP.md | Historyczne dowody nie zastępują aktualnego testu |

## Wykryte sprzeczności i ryzyka

1. Starszy dokument `MASTER_ARCHITECTURE_26_PDFS.md` zawiera superseded numerację etapów 0.39–0.50; obowiązuje v11 `ROADMAP.md`.
2. Fragmenty Rust w PDF nie są gotowymi implementacjami: występują przestarzałe API Zenoh/OTLP i niebezpieczne demonstracyjne skróty.
3. Twierdzenia o 35 μs, 40/67 Gbps lub <1 ms wymagają własnych benchmarków i topologii.
4. CRDT nie może być autorytatywnym mechanizmem fencing, uprawnień ani płatności.
5. Firecracker wymaga Linux/KVM; Android/iOS potrzebują osobnych backendów.
6. Deklaracje ZKP/PQC i „AGI” nie są dowodami.
7. `Cargo.toml` deklaruje MIT; komunikat „wszystkie prawa zastrzeżone” w README wymaga spójnej decyzji licencyjnej.
8. Etapy 0.00–0.38 oznaczone administracyjnie jako ukończone nie mają przez to automatycznie potwierdzonego odbioru produkcyjnego.

## Deduplikacja i decyzja Roadmapy

Nie stwierdzono jeszcze nowego, unikalnego wymagania, które można bezpiecznie dodać jako 1.95 bez powtórzenia pozycji 0.39–1.94 lub INV-001–024. **Nie dodawano sztucznych etapów**. W kolejnych partiach dopisywać wyłącznie nowe kontrakty z `PDF + strona + acceptance gate + dedupe proof`; istniejące numery i 10 Builders zachować.

## Otwarte bramy audytu wykonawczego

- Pełny checkout Git i śledzenie wymagań do symboli oraz testów.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features`, `cargo test --all-features`, SAST/SBOM na aktualnym SHA.
- Live WASI/Firecracker na KVM, PostgreSQL HA na odrębnych kernelach, SLO/DR.
- Realne OTLP, CRDT convergence, OAuth PKCE, MCP interoperability i testy ZKP/PQC.
- Weryfikacja workflow na aktualnym commicie.

**Werdykt: brak podstaw do produkcyjnego GO; pełna weryfikacja techniczna pozostaje otwarta.**


## Aneks weryfikacyjny 20/20 źródeł i kodu (15.10.2026)

**Weryfikacja plików PDF:** 20 plików, 307 stron, 16 unikalnych znormalizowanych tekstów. SHA-256 treści tekstowej wykazał trzy grupy identycznych źródeł: (a) `Analiza Architektury Systemu OCAF (1).PDF` i `Analiza Architektury Systemu OCAF.PDF`; (b) `Architektura OCAF w Języku Rust (1).PDF`, `Architektura OCAF w Języku Rust.PDF`, `Architektura OCAF w Języku RustPDF_261012_042941.pdf`; (c) `Projekt i Mapa Drogowa Rewolucyjnej Architektury S.. (1).PDF` i `Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(1) (1).PDF`. Pozostałe 13 źródeł mają różny tekst. Pełny indeks stron/tematów dostępny w oddzielnym lokalnym raporcie.

**Rzeczywista inspekcja kodu na `main`:**
- `src/auth/mod.rs`: `AuthContext` przechowuje `macaroon`, `preimage`, `invoice`, `nonce`, ale `DenyByDefaultVerifier` zawsze zwraca `VerificationUnavailable`. To nie jest działający L402/payment settlement ani OAuth PKCE.
- `src/transport/mod.rs`: komentarz źródłowy jednoznacznie zastrzega, że otwarcie sesji Zenoh nie zapewnia automatycznie tool discovery/pub-sub/remote execution. Transport RPC istnieje w osobnych modułach, więc nie należy utożsamiać tej uwagi z brakiem wszelkiej komunikacji.
- `src/registry/mod.rs`: lokalny rejestr z kompilacją input/output JSON Schema; blokuje zewnętrzne `$ref`, `$dynamicRef`, `$recursiveRef`. Nie potwierdza federacji MCP.
- `src/rpc/mod.rs`: uwierzytelnianie HMAC-SHA256, ograniczenia ramek i replay; komentarz źródłowy stwierdza, że ramki same w sobie **nie są szyfrowane** — wymagany autoryzowany TLS.
- `src/executor/mod.rs`: obecny Linux-only moduł `wasi`; nie dowodzi ukończonego broker→Firecracker guest WASI.
- `src/cluster/mod.rs`: kod opisuje eksperymentalne PostgreSQL primitives i wyraźnie odróżnia je od replikacji katalogu SQLite. Nie jest to dowód działającej repliki PostgreSQL HA.
- `Cargo.toml`: Zenoh 1.10, Tokio, `tracing`, `jsonschema`; nie deklaruje `automerge`, `yrs`, `opentelemetry-otlp`, `tree-sitter`, `halo2` w głównym crate. Brak zależności sam w sobie nie dowodzi nieistnienia każdego adaptera w całym repo.
- `.github/workflows/rust.yml`: rzeczywiste kroki `cargo fmt`, core-only Clippy i testy, Wasmtime acceptance, PostgreSQL loopback chaos. Definicja workflow nie jest dowodem zakończenia bieżącego run ani prawdziwego KVM i multi-host failover.
- `README.md` utrzymuje jedną tabelę 269 pozycji i wyraźnie rozróżnia administracyjne `ETAP UKOŃCZONY` od odbioru produkcyjnego.

**Werdykt:** audyt 20 dokumentów źródłowych i wybranych modułów wykonano; **pełny audyt wszystkich plików kodu, uruchomienie testów oraz certyfikacja bezpieczeństwa i infrastruktury NIE zostały wykonane**. Produkcyjne GO = **NO-GO**. Nie nadano sztucznych ID 1.95+, gdyż opisane wymagania mają już przypisania w v11 i INV-001–024.


## Kontrola kompletności Roadmapy — wynik korekty 20 PDF

**Wynik porównania z kanoniczną Roadmapą v11 przed korektą:** NIE, nie wszystkie wymagania były jawnie ujęte. Dokument `Analiza AGI I Ludzkich AI (1).PDF` (s. 2–6, 11–13) wymienia specyficzne technologie i benchmarki, których nie było w `ROADMAP.md` pod nazwami: OpenCog Hyperon/MeTTa/PLN/ECAN; ACT-R/Soar/LIDA; Letta/MemGPT, Mem0, Cognee, Zep i ewaluacje LoCoMo/LongMemEval. Samo ogólne hasło „cognition/memory” nie wystarczało do potwierdzenia kompletności.

**Korekta na `main`:** dopisano trzy odrębne, weryfikowalne etapy:
- `1.95`: eksperymentalny Hyperon/MeTTa/PLN/ECAN z kontrolą uprawnień i oceną niepewności;
- `1.96`: porównanie ACT-R, Soar, LIDA względem istniejących mechanizmów;
- `1.97`: Letta/MemGPT, Mem0, Cognee, Zep/Graphiti, LoCoMo/LongMemEval, metryki pamięci i izolacja tenantów.

Zaktualizowano `ROADMAP.md`, `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, `docs/roadmap/canonical_v11_inventory.json` oraz `README.md`. Manifest ma teraz **198 etapów głównych + 64 techniczne + 10 Agents Builder = 272 pozycje**. Nowy numer dla unikalnego przyszłego wymagania: `1.98`.

### Mapa 20 plików → zakres istniejącej Roadmapy lub korekta

| Źródło PDF | Dominujący zakres Roadmapy | Decyzja |
|---|---|---|
| Analiza AGI I Ludzkich AI (1).PDF | 0.48, 0.91, 1.65–1.69; nowe 1.95–1.97 | **LUKI UZUPEŁNIONE** |
| Analiza Architektury Systemu OCAF (1).PDF | 0.43–0.49, 0.63 | ISTNIEJĄCE |
| Analiza Architektury Systemu OCAF.PDF | 0.43–0.49, 0.63 | DUPLIKAT TEKSTU |
| Analiza Architektury Systemu OCAF(1).PDF | 0.43–0.49, 0.63 | ISTNIEJĄCE |
| Analiza Architektury Systemu OCAF(2).PDF | 0.39, 0.43–0.49, 1.90–1.94 | ISTNIEJĄCE |
| Analiza Architektury Systemu OCAF(3).PDF | 0.39, 0.43–0.49, 1.90–1.94 | ISTNIEJĄCE |
| Analiza Architektury Systemu OCAF(4).PDF | 0.39, 0.43–0.49, 1.90–1.94 | ISTNIEJĄCE |
| Analiza Architektury Systemu OCAF(5).PDF | 0.39, 0.43–0.49, 1.90–1.94 | ISTNIEJĄCE |
| Architektura OCAF w Języku Rust (1).PDF | 0.42–0.49, 0.63 | ISTNIEJĄCE |
| Architektura OCAF w Języku Rust.PDF | 0.42–0.49, 0.63 | DUPLIKAT TEKSTU |
| Architektura OCAF w Języku RustPDF_261012_042941.pdf | 0.42–0.49, 0.63 | DUPLIKAT TEKSTU |
| Architektura OCAF w Języku Rust(1).PDF | 0.39, 0.43–0.49, 1.90–1.94 | ISTNIEJĄCE |
| Architektura OmniContext Agent Framework (OCAF) w .. (1).PDF | 0.39–0.49, 1.90–1.94 | ISTNIEJĄCE |
| Dokument bez nazwy (2).PDF | 0.43–0.49 | ISTNIEJĄCE |
| Dokument bez nazwy (6).PDF | 0.43–0.49 | ISTNIEJĄCE |
| Kompleksowa Roadmapa Wdrożenia Ekosystemu OCAF w Rust.PDF | 0.42–0.49 | ISTNIEJĄCE |
| Projekt i Mapa Drogowa Rewolucyjnej Architektury S.. (1).PDF | 0.39–0.49, 0.50–0.55 | ISTNIEJĄCE |
| Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(1) (1).PDF | 0.39–0.49, 0.50–0.55 | DUPLIKAT TEKSTU |
| Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(2) (1).PDF | 0.39–0.49, 0.50–0.55 | ISTNIEJĄCE |
| Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(3).PDF | 0.39–0.49, 0.50–0.55 | ISTNIEJĄCE |

**Zakres zapewnienia:** ta mapa potwierdza obecność głównych obszarów funkcjonalnych w Roadmapie oraz naprawienie zidentyfikowanych braków nazwanych. Nie jest dowodem, że każda pojedyncza rekomendacja, przykładowy snippet i wariant API z 307 stron został odrębnie przepisany na zadanie. Nie jest też potwierdzeniem wykonania kodu ani testów. Wymagania `1.95–1.97` mają status **PLANNED**, a nie DONE.


### Uzupełniająca kontrola źródłowa AGI PDF — dodatkowe trzy luki

Po ponownym odczytaniu `Analiza AGI I Ludzkich AI (1).PDF` (wyciąg tekstowy: sekcje „DSPy: Koniec Epoki Prompterów”, „SWE-bench: Ewaluacja Sprawności Ludzkiej”, opis „taskweavingu”) stwierdzono brak w kanonicznym ROADMAP.md jawnych kryteriów:
- `1.98` DSPy: BootstrapFewShot, MIPRO, GEPA, oddzielny zbiór holdout i kontrola kosztu oraz regresji;
- `1.99` SWE-bench Verified, N-run reliability, testy powtarzalne, artefakty patch/test oraz zakaz fabrykowania wyników;
- `2.00` taskweaving DAG, asynchroniczne zależności, rollback, idempotencja i fault injection.

Po synchronizacji Roadmapy, manifestu, lustra i README: **201 głównych + 64 techniczne + 10 Agents Builder = 275 pozycji**; kolejny numer **2.01**. Wpisy są `PLANNED`, nie wykonane. Przypisanie wszystkich 20 PDF do rodzin etapów nie jest równoważne sprawdzeniu każdego indywidualnego zdania ani uruchomieniu testów źródłowych.
