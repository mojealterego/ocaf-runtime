# PDF 03/40 — Analiza Architektury Systemu OCAF(1).PDF

**Źródło:** 15 stron, odrębna rewizja względem PDF 02 (13 stron). Ocena zakresu roadmapy, nie potwierdzenie implementacji. Weryfikacja tekstu źródłowego obejmuje strony 1–15. Strony 11–13 zawierają istotny dodatkowy rozdział 8 (Rust/Zenoh/ZKP), którego nie należy pomijać ani mylić z bibliografią.

| Strona | Wymaganie lub charakter materiału | Etapy roadmapy | Rozstrzygnięcie |
|---|---|---|---|
| 1 | Problem przeciążenia kontekstu i dynamiczny routing MCP, izolacja OAuth, MicroVM i płatności | 0.39, 0.43, 0.44, 0.49 | Zakres istnieje; deklaracje autora nie dowodzą implementacji |
| 2 | Lost-in-the-Middle, metryki BFCL/GeoEngine, hypothesize-retrieve-invoke | 0.43, 0.98 | Wymaga testu A/B statycznego i dynamicznego ładowania schematów na tych samych zadaniach; liczb nie przypisywać OCAF |
| 3 | MCP tools/list, inputSchema/outputSchema, usuwanie schematów po użyciu, confused deputy | 0.43, 0.44 | Wymaga kontroli wersji, ACL odkrywania i unieważniania; kontrakt odziedziczony po PDF 02 |
| 4 | Managed OAuth 2.1, PKCE, per-user routing i izolacja tokenów | 0.44 | Tenant/user/session binding i negatywne testy token leak/replay |
| 5 | Code-as-a-Tool, @tool, introspekcja funkcji, RCE i eskalacja uprawnień | 0.43, 0.55, 0.63 | Typowana walidacja schematu, sandbox, zakaz automatycznej autoryzacji na podstawie docstrings |
| 6 | Firecracker, izolacja jądra i egress, ekonomia M2M | 0.39, 0.49 | Rzeczywisty guest i egress broker; nie wystarcza Wasmtime w procesie hosta |
| 7 | Rejestry MCP, L402/x402, budżet, płatność i macaroon | 0.43, 0.49, 1.53–1.54, 1.85–1.87 | Osobne protokoły, realna autoryzacja i settlement; fail closed |
| 8 | Enterprise ingest/RAG, rekurencyjny parser, provenance i OWASP | 0.48, 0.63 | ACL per chunk, delete propagation, izolacja niezaufanych dokumentów |
| 9 | ASI01 goal hijack, ASI02 tool misuse, ASI03 identity/privilege abuse | 0.44, 0.63, 0.83 | Negatywne scenariusze injection, naruszenia scope i cross-tenant |
| 10 | ASI04 supply chain/rug pull, ASI05 unexpected code execution | 0.39, 0.43, 0.63 | Pinning podpisanego manifestu i testy podmiany wersji, sandbox breakout |
| 11 | Podsumowanie i początek **rozdziału 8**: Rust, Zenoh i CRDT | 0.46, 1.15, 1.19 | Rozdział 8 jest nową treścią względem wcześniejszej rewizji; rozdzielić SHM lokalne od transportu sieciowego |
| 12 | Zenoh SHM i Automerge CRDT, OpenTelemetry, Aya/eBPF, ZKP/Arkworks/Halo2, L402 offline, type-sitter | 0.39, 0.43, 0.45–0.47, 0.87, 1.20, 1.33, 1.42, 1.55, 1.57 | Wszystkie obszary mają istniejące etapy; uzupełnić kryteria bezpieczeństwa i testów |
| 13 | PQC FIPS 203/204, X25519MLKEM768, Loihi 2 SNN; początek bibliografii | 0.45, 0.91, 1.38–1.39, 1.41, 1.76–1.77, 1.79 | Nie uznawać hardware ani PQC za zintegrowane bez interoperacyjnych i sprzętowych dowodów |
| 14 | Bibliografia, w tym Zenoh, CRDT, eBPF, ZKP, Tree-sitter i Loihi | etapy wyżej | Źródła referencyjne, brak dodatkowego obowiązku z samego cytowania |
| 15 | Koniec bibliografii, Loihi | 0.91, 1.38 | Źródło referencyjne, brak nowego obowiązku |

## Szczególne braki kontraktów w rozdziale 8 (strony 11–13)

1. **0.46 / 1.15 / 1.55:** zero-copy SHM dotyczy lokalnej domeny pamięci i zgodnej konfiguracji Zenoh. Nie deklarować zero-copy przez niezależne hosty i Firecracker bez odrębnych dowodów. Porównywać rzeczywiste payloady, wersje Zenoh i ścieżki TCP/SHM.
2. **0.46 / 1.19 / 1.57:** CRDT Automerge/Yrs jedynie dla stanu nieautorytatywnego; nie zastępować PostgreSQL jako źródła prawdy dla lease, fencing, płatności lub autoryzacji. Wymagane convergence/partition/tombstone/replay tests.
3. **0.47 / 1.20 / 1.21 / 1.63–1.64:** propagacja trace context i ograniczone kolejki eksportu OTel; mierzyć overhead i gwarantować flush/shutdown; nie obiecywać zerowego narzutu.
4. **0.39 / 1.42 / 1.80 / 1.84:** Aya/eBPF na rzeczywistym jądrze gościa wymaga weryfikacji capability, loadera, map i polityki kwarantanny. SIGKILL procesu hosta nie jest tożsamy z izolacją MicroVM.
5. **0.45 / 0.87 / 1.85–1.87:** eksperyment ZKP dla klauzul uprawnień wymaga niezależnego verifiera, jawnego public input, soundness i testów niepoprawnego świadka; offline settlement wymaga późniejszego rozliczenia i limitów strat, nie można zakładać bezwarunkowego zaufania do CRDT.
6. **0.43 / 1.33 / 1.72:** type-sitter/Tree-sitter służy do typowanej analizy składni, a nie automatycznego dowodu semantycznej zgodności dowolnego API; konieczne wersjonowane gramatyki i walidacja schema.
7. **0.45 / 1.41 / 1.79:** ML-KEM/ML-DSA, hybrydowy TLS, downgrade resistance i transcript binding są osobnymi kontraktami. Sama obecność biblioteki aws-lc-rs nie dowodzi obsługi obu algorytmów.
8. **0.91 / 1.38–1.39 / 1.76–1.77:** Loihi 2 jako eksperymentalny adapter sprzętowy, pomiar na tym samym workload, fallback CPU; podawane w PDF „rzędy wielkości” poprawy nie są zweryfikowanymi benchmarkami OCAF.

## Decyzja względem numeracji

Bieżący manifest v11 zawiera już główne etapy 0.39, 0.43–0.49, 0.63, 0.87, 0.91 i szczegółowe podetapy 1.15, 1.19–1.21, 1.33, 1.38–1.42, 1.55, 1.57, 1.63–1.64, 1.72, 1.76–1.80, 1.84–1.87. **Brak uzasadnienia nowego etapu 2.07** z samego PDF 03; potrzebne jest doprecyzowanie kryteriów istniejących etapów.

**Granica audytu:** ustalono dopasowanie wymagań do zakresu Roadmapy, nie sprawdzono w tym dokumencie kodu, uruchomionych testów wielohostowych ani odbioru operatora. Status PLANNED/UNVERIFIED.
