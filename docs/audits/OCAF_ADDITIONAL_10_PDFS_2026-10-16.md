# OCAF — druga paczka dokumentacji, analiza pokrycia (2026-10-16)

## Zakres i dowody

Przetworzono **11 nowych plików PDF, 252 strony** w rodzinie `Architektura OmniContext Agent Framework (OCAF) w ..`. To kolejne rewizje jednej rozwijanej architektury: nie należy liczyć powtórzeń rozdziałów jako niezależnych funkcji. Ostatnia wersja `..(10).PDF` liczy 33 strony i zawiera rozdziały 1–28. Weryfikacja została przeprowadzona na ekstrakcji tekstowej `pdftotext -layout`, nie na kompilacji przykładowych fragmentów kodu.

| Plik źródłowy | Strony |
|---|---:|
| `Architektura OmniContext Agent Framework (OCAF) w .. (1).PDF` | 24 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(1) (1).PDF` | 26 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(2).PDF` | 19 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(3).PDF` | 21 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(4).PDF` | 21 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(5).PDF` | 21 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(6).PDF` | 21 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(7).PDF` | 16 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(8).PDF` | 22 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(9).PDF` | 28 |
| `Architektura OmniContext Agent Framework (OCAF) w ..(10).PDF` | 33 |

## Macierz wymagań (weryfikacja jawnej obecności w Roadmapie)

Wszystkie 30 nazw kontrolnych (Zenoh, Automerge, OTLP, Tree-sitter, Firecracker, L402, ML-KEM, Loihi, ZKP oraz 21 rodzin Omni*) zostały znalezione w aktualnym `ROADMAP.md`. **Wystąpienie nazwy nie jest dowodem implementacji ani kompletności wymagań szczegółowych.** Poniższe strony odnoszą się do `..(10).PDF`.

| Wymaganie | Strony PDF | Słowa kontrolne w Roadmapie | Status implementacji | Minimalna walidacja |
|---|---|---|---|---|
| Zenoh SHM typed zero-copy | 1–3 | Zenoh; SHM | unverified | Benchmark 64B and 128KB; memory-safety and tenant boundaries |
| Automerge/Yrs offline-first CRDT | 2–4 | Automerge | unverified | Partition, merge convergence, provenance, tombstone and tenant tests |
| OTLP and Tokio trace propagation | 3–5 | OTLP | unverified | Trace/span continuity across spawn and RPC, exporter backpressure |
| MCP schema fusion and Tree-sitter CST | 4–6 | Tree-sitter | unverified | Adversarial metadata, schema equivalence, parser-version tests |
| OAuth 2.1 PKCE, Macaroons, ZKP | 4–7 | ZKP | unverified | Fail-closed verifier, key rotation, no token exposure, real proof verification |
| Firecracker microVM with eBPF | 5–7 | Firecracker | unverified | Escape and syscall-policy tests; never equate WASI with microVM |
| L402 offline economy | 6 | L402 | unverified | Real invoice verification, replay prevention, budget cap and settlement audit |
| PQC and neuromorphic edge | 6–7 | ML-KEM; Loihi | unverified | Interoperability and crypto agility; hardware availability gates |
| OmniDev/OmniUX/OmniE2E/OmniForge | 7–10 | OmniDev; OmniUX; OmniE2E; OmniForge | unverified | Real repo AST, sandbox compilation, visual/E2E/game build evidence |
| OmniWeb/OmniStudio/OmniDoc | 9–11 | OmniWeb; OmniStudio; OmniDoc | unverified | Consent/robots and SSRF controls; media/document deterministic outputs |
| OmniGov/OmniFin/OmniPhys/OmniSec | 11–12 | OmniGov; OmniFin; OmniPhys; OmniSec | unverified | Authorization, ROI ledger, hardware safety and approved purple-team scopes |
| OmniCog/OmniEvolve/OmniSense | 12–14 | OmniCog; OmniEvolve; OmniSense | unverified | Memory deletion, bounded self-change, reproducible reasoning benchmarks |
| OmniArbitrage/OmniStrategy/OmniGraph | 14–20 | OmniArbitrage; OmniStrategy; OmniGraph | unverified | Decision calibration, DAG cycles, cancellation and retries |
| OmniCausal and neurosymbolic verification | 20–23 | OmniCausal | unverified | SMT proof obligations, evidence hashes, invalid proof rejection |
| OmniSleep/OmniPrune and latent messaging | 23–25 | OmniSleep; OmniPrune | unverified | Non-destructive consolidation, audit retention, codec compatibility |
| Active inference, topology, HDC, OmniFractal/OmniNARS | 26–31 | Active Inference; HDC; OmniFractal; OmniNARS | unverified | Toy baselines vs measured results, topology sanity, logical consistency |

## Krytyczne rozbieżności w przykładach kodu źródłowego

1. **OmniDev:** metoda `weryfikuj_w_sandboxie` w `..(10).PDF` s. 7 zwraca bezwarunkowo `true`; `analizuj_drzewo_skladni` zwraca `AST_MAP`, a `generuj_latke_kodu` tekst `git diff`. To **stub demonstracyjny**, nie działający silnik kodujący. Nie wolno przepisać go do repo jako ukończonego modułu.
2. **OmniUX:** analogiczny kod zwraca stałe `FIGMA_AST` i deklaruje sukces przy `ocena_ux > 0.98` bez udokumentowanego pomiaru. Wymagane testy wizualne na rzeczywistych renderach.
3. **L402:** w `..(10).PDF` s. 6 występują `mock_paid_preimage` i `mock_macaroon`, bez weryfikacji zapłaty. Należy zabronić fałszywego oznaczania płatności jako rozliczonych.
4. **ZKP:** przykład obwodu R1CS na s. 4–5 narzuca jedynie równość zmiennej świadka z publicznym wejściem, a nie dowodzi weryfikacji podpisanego Macaroona. Nie stanowi dowodu bezpieczeństwa.
5. **eBPF:** przykład na s. 5 dołącza tracepoint `openat`, lecz nie implementuje deklarowanej blokady ransomware ani wyłączenia microVM. Wymagana rzeczywista egzekucja polityki i testy.
6. **PQC:** samo wywołanie domyślnego providera `aws_lc_rs` nie dowodzi, że negocjowany TLS faktycznie używa ML-KEM. Wymagana kontrola uzgodnionej grupy i zgodności wersji bibliotek.
7. **Tree-sitter:** detekcja jednego literalnego tokenu `<|override_system_prompt|>` nie wystarcza do ochrony przed prompt injection. Wymagana kontrola pochodzenia metadanych i polityki uprawnień niezależna od heurystyki ciągów.
8. **Samodoskonalenie:** żaden pokazowy hot-swap nie może ominąć review, testów regresyjnych, podpisów artefaktów ani możliwości rollback.

## Decyzja roadmapowa

**Nie dodano kolejnych etapów wyłącznie na podstawie nazw.** Wszystkie rozpoznane rodziny funkcji mają istniejące punkty Roadmapy; nowe nazwy etapów bez deduplikacji wprowadziłyby chaos. Dalsze rozliczanie musi dotyczyć **atomowych kryteriów akceptacji i testów negatywnych** dla każdej rodziny, nie liczby dopisanych wierszy. Etapy 0.38–0.41 nadal nie mogą zostać oznaczone jako zatwierdzone bez dowodów infrastrukturalnych i podpisu operatora.

## Ograniczenia

To kontrola 11 plików, ich rozdziałów i 16 grup wymagań. **Nie jest to pełna weryfikacja każdej linijki wszystkich 252 stron względem każdego pliku źródłowego repo**. Statusy implementacji pozostają `unverified` do czasu udokumentowanego przeglądu kodu i uruchomienia testów. Liczba pozycji kanonicznej Roadmapy pozostaje **281**.

## Uzupełnienie po kontroli wszystkich stron (2026-10-16)

- [Szczegółowa macierz 29 grup rozdziałów ↔ konkretne etapy Roadmapy](OCAF_BATCH2_CHAPTER_TO_ROADMAP_MATRIX_2026-10-16.md) — zastępuje ocenę opartą wyłącznie na obecności 30 nazw.
- Lokalny artefakt dowodowy `OCAF_BATCH2_252_PAGE_TRACEABILITY.md` obejmuje wszystkie 252 strony, z identyfikatorem pliku, numerem strony, SHA-256 tekstu, nagłówkami i wykrytymi domenami. Pełny rejestr lokalny nie jest automatycznie tożsamy z plikiem na GitHub.
- W `ROADMAP.md` i jej lustrzanej kopii skorygowano błędne historyczne podsumowanie 195/259 do aktualnego 207/281.
- **Werdykt:** pokrycie zakresu w Roadmapie dla 29 grup rozdziałów; testy funkcjonalne, przegląd atomowych wymagań i produkcyjny odbiór niepotwierdzone.
