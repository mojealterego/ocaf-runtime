# OCAF — konsolidacja 26 dokumentów i plan 0.36–0.50

Stan analizy: 2026-10-13. Źródła: trzy partie, 26 załączonych plików PDF,
łącznie **556 stron liczonych z powtórzeniami** (162 + 219 + 175).
To liczba stron dostarczonych plików, **nie** liczba unikalnych stron ani
niezależnych wymagań. Przynajmniej dwie nazwy plików w kolejnych partiach
powtarzają się, a duże części kolejnych opracowań zawierają
powielone rozdziały. Starsze i nowsze teksty są traktowane jako
wersjonowane opracowania, a nie 26 równorzędnych specyfikacji.

Repozytorium: `mojealterego/ocaf-runtime`, branch **main**.
Bazowy zweryfikowany stan analizy: `3c447e972796430be5321884d14757e1a71da0f4`.
Workflow Rust CI `38389948598` i Supply Chain `38389948609` zakończyły
się powodzeniem. `Cargo.toml` nadal deklaruje wersję **0.26.0**:
numer priorytetu 0.35 **nie oznacza** wydanej wersji 0.35.0.

## 1. Zasady konsolidacji

- Pierwszeństwo mają testowane niezmienniki i faktyczny kod, a nie
  deklaracje opisowe lub demonstracyjne wywołania `println!`.
- Powtórzone rozdziały nie generują ponownych zadań.
- Każdy komponent otrzyma status: **istnieje/testowany**, **częściowy**,
  **nieobecny**, **koncepcja/badania**.
- Zależności i twierdzenia wydajnościowe z PDF nie są dowodem skuteczności.
- Zakaz automatycznego samodzielnego wdrażania zmian w produkcji.
  Wszelkie zmiany źródeł wyłącznie w `main` i z testami.

## 2. Architektura docelowa

1. **OCAF Kernel (Rust/Tokio):** harmonogram, API zadań, uprawnienia,
   limity, audyt, replay/fencing, ścisłe przetwarzanie skutków ubocznych.
2. **Identity & Security:** zaufana identyfikacja transportu mTLS,
   powiązanie jej z tożsamością RPC, zarządzane OAuth/PKCE, rotacja
   certyfikatów/HMAC, izolacja sekretów.
3. **Execution Boundary:** broker do Firecracker MicroVM / izolacji
   Wasm z limitami CPU/RAM/IO, sieciowym default-deny, TTL, audit trail.
   MicroVM dotyczy serwerów Linux/KVM; platformy mobilne używają
   osobnego, ograniczonego mechanizmu wykonawczego.
4. **Transport & Persistence:** Zenoh TLS/TCP, PostgreSQL jako
   autorytatywny stan lidera, leasingu i efektów; CRDT **wyłącznie**
   dla nieautorytatywnej pamięci i współedytowanych grafów.
   SHM nie jest automatycznie zero-copy pomiędzy różnymi hostami.
5. **MCP Tool Plane:** rejestr i dynamiczne odkrywanie, walidacja
   JSON Schema, polityki narzędzi, podpisane manifesty,
   ograniczenia kosztów oraz dopuszczanie narzędzi na podstawie
   uprawnień (nie wyłącznie ich opisów tekstowych).
6. **Context & Reasoning:** OmniCog/OmniGraph,
   OmniSleep/OmniPrune, OmniArbitrage, OmniContract.
   Wybór modelu i pamięć semantyczna są osobne od decyzji
   o uprawnieniu do wykonania skutków.
7. **Agent Development Platform:** wspólne `ocaf-builder-core`
   z kanoniczną reprezentacją grafu IR i wersjonowanym manifestem;
   adaptery Cloud/Axum, Desktop/Tauri, Mobile/UniFFI.
   Dwukierunkowa edycja Code/No-Code wymaga jawnej semantyki
   zachowania i rozpoznawania fragmentów nieodwracalnych.
8. **Optional Domain Packs:** OmniDev, OmniUX, OmniE2E, OmniWeb,
   OmniForge, OmniStudio, OmniDoc, OmniSec, OmniGov, OmniFin,
   OmniPhys i osobny moduł OmniMath. Działają przez te same
   bramki uprawnień i efektów, a nie wewnątrz zaufanego jądra.

## 3. Ustalenia o aktualnym kodzie

| Obszar | Faktyczne świadectwo w main | Ocena |
|---|---|---|
| Rust/Tokio runtime i budżety | `src/runtime`, `src/executor` | Istnieje |
| Walidacja narzędzi | `src/registry` i JSON Schema | Częściowa: brak pełnego zdalnego MCP |
| Autoryzacja | `src/auth` ma `DenyByDefaultVerifier`, RPC HMAC | Częściowa: brak produkcyjnego OAuth2.1/PKCE |
| Secure RPC | `src/rpc`, 0.35 per-peer HMAC cutover | Istnieje; brak transportowego mTLS→RPC identity binding |
| Zenoh/mTLS | `src/transport`, `src/transport_policy.rs` | Istnieje; realna rotacja na hostach niepotwierdzona |
| HA/fencing | `src/cluster`, `src/jobs`, `src/sink` | Istnieje; brak dowodu failover całego środowiska |
| CI, SBOM i atestacje | workflow supply-chain, 0.31–0.34 | Istnieje; nie jest produkcyjną certyfikacją |
| MicroVM/eBPF | brak takiego backendu w `src/executor` i zależnościach | Nieobecne |
| CRDT/OTLP | brak Automerge/Yrs i eksportera OTLP w Cargo | Nieobecne |
| L402/x402 i rozliczenia | `AuthContext` i `cost_sats` nie są portfelem ani settlementem | Nieobecny realny billing |
| PQC/ZKP | brak silników dowodowych/algorytmów w zależnościach | Nieobecne |
| Omni* swarms, Builder, OmniMath | brak modułów oraz E2E wdrożeniowego | Koncepcje |

`Cargo.toml` nadal zawiera `license = "MIT"`, podczas gdy
README zastrzega prawa autorskie. Przed dystrybucją konieczna
jest świadoma decyzja o zakresie licencji i spójnych komunikatach.

## 4. Sprzeczności i niezweryfikowane obietnice z PDF

1. **Deklaracje 5–35 μs i <1 ms dla Zenoh:** zależą od topologii,
   rozmiaru ładunku, hostów i trybu SHM; wymagają własnych benchmarków.
2. **CRDT ≠ konsensus.** Nie wolno stosować eventual consistency do
   uprawnień, ledgera płatniczego ani wyłączności lidera.
3. **„Formalne dowody bezpieczeństwa dowolnego kodu”:** Z3/SMT i
   reguły tekstowe nie dowodzą ogólnego zakończenia wszystkich programów.
   Dowodzi się zdefiniowanych własności w konkretnym modelu.
4. **„Automatyczny hot-swap i samoulepszanie”:** przykłady w PDF
   często sygnalizują tylko powodzenie; produkcyjna mutacja
   bez podpisanej promocji, odwracalności i kontroli człowieka
   pozostaje zabroniona.
5. **Builder Code↔No-Code:** pełna bezstratna odwracalność
   dowolnego Rusta nie wynika z użycia Tree-sitter; należy
   ograniczyć gwarancję do kanonicznego DSL/grafu.
6. **Firecracker na telefonie:** nie należy przenosić wymagań
   Linux/KVM bezpośrednio na Android/iOS. Oddzielić backendy.
7. **Latent messaging:** wektory wymagają uzgodnionego modelu,
   wymiarów, polityki dostępu i wersji osadzeń; nie są uniwersalnym
   protokołem semantycznym.
8. **OmniMath:** poprawnie sprawdzony dowód jest mocny tylko
   w założonym systemie aksjomatów; heurystyczna redukcja
   AST nie jest sama certyfikowanym e-graph lub dowodem Lean.
   „Konsensus modeli” nie zastępuje niezależnego checkera.
9. **OmniWeb:** automatyzacja musi respektować zgodę,
   uprawnienia, limity, prywatność i warunki dostępu; nie
   traktować omijania zabezpieczeń serwisu jako funkcji domyślnej.
10. **L402/x402 i zakupy M2M:** żadnych realnych wydatków
   bez jawnych limitów, potwierdzeń i bezpiecznego ledgeru.
11. **OmniPhys i OmniSec:** efekty fizyczne i operacje
   bezpieczeństwa wymagają osobnych bramek autoryzacji,
   zakresów uprawnień i środowisk testowych.
12. **OmniAlign/PQC/„quantum-ready”:** hashe w przykładach i
   nazwy modułów nie stanowią kryptograficznego dowodu zgodności
   ani niezależnej certyfikacji.

## 5. Zamknięta propozycja etapów 0.36–0.50

**Strumień A: produkcyjny rdzeń — 0.36–0.40**

| Etap | Rezultat w repozytorium | Warunek akceptacji |
|---|---|---|
| 0.36 | mTLS certificate identity→RPC peer binding | Podszyty i niezgodny SAN nie uzyskuje praw przy poprawnym HMAC |
| 0.37 | izolowany broker wykonania Wasm/MicroVM | Sieć/FS default-deny, limity, cleanup, negatywne testy eksfiltracji |
| 0.38 | pełna HA i odbiór na trzech niezależnych hostach | Utrata węzła, split-brain, stale effect rejection, trwały journal |
| 0.39 | DR + odtwarzanie + chaos + soak | RTO/RPO, rollback, restore, podziały sieci, obserwowalność |
| 0.40 | hardening wydania runtime / decyzja GO-NO-GO | Supply-chain PASS, bezpieczeństwo, niezależna opinia, podpisany odbiór |

**Strumień B: rozszerzony ekosystem — 0.41–0.50**

| Etap | Zakres | Minimalne kryterium odbioru |
|---|---|---|
| 0.41 | MCP tool-plane: discover/routing/manifest | Nieznane/niezweryfikowane narzędzia nie mogą się wykonać |
| 0.42 | Delegowane OAuth2.1/PKCE i adaptery danych | Ścisła izolacja sesji i testy confused deputy |
| 0.43 | OTLP i CRDT dla kontekstu współdzielonego | Ostateczna spójność nie wpływa na fencing i uprawnienia |
| 0.44 | OmniCog / OmniGraph / OmniSleep / OmniPrune | Pamięć, lineage, TTL, replay, polityka retencji |
| 0.45 | OmniArbitrage / OmniContract / OmniGov / OmniFin | Budżety i SLA egzekwowane fail-closed, HITL |
| 0.46 | Opcjonalny L402/x402 adapter | Najpierw testnet/symulacja, ledger idempotentny, brak auto-spend |
| 0.47 | OmniDev / OmniSec / OmniVerify w ograniczonej domenie | Sandbox, testy, podpisane artefakty, ludzka aprobata |
| 0.48 | Agents Builder: kanoniczny graf IR i DSL | Graph validation, wersjonowanie i round-trip w dopuszczonym DSL |
| 0.49 | Cloud/Desktop/Mobile adapters + domain packs | Oddzielne builds i testy platform; ograniczone capability |
| 0.50 | OmniMath i końcowy odbiór platformy | Niezależny checker Lean/SMT dla wybranych twierdzeń, E2E i SLO |

**0.40** jest punktem ukończenia *bezpiecznego runtime*;
**0.50** jest punktem przeglądu *pierwszej platformy OCAF*.
Nie oznacza to, że 0.50 zawiera pełne produkcyjne wersje
wszystkich rojów i kierunków badawczych — to wymaga osobnych,
wersjonowanych pakietów `1.x`.

## 6. Osobny backlog badawczy (bez deklaracji produkcyjnej)

OmniEvolve/OmniCausal autonomiczne RSI, OmniInference/OmniTDA,
OmniHDC / Sparse SAE, OmniFractal/OmniNARS,
OmniCategory/OmniLogic, OmniTranspile/PCC,
OmniThermo oraz OmniAlign pozostają badaniami,
dopóki nie zostaną zdefiniowane mierzalne hipotezy,
dane testowe, modele zagrożeń i niezależne kryteria sukcesu.

`Jarvis.PDF` jest dokumentem porównawczym i koncepcją
**OmniMath**; nie dowodzi implementacji mostu Lean, e-graphs
ani solvera SMT w obecnym OCAF. Integracja z JARVIS
powinna przebiegać przez ograniczony, wersjonowany interfejs,
a nie przez import prywatnego stanu do jądra OCAF.

## 7. Reguła uruchamiania kolejnych prac

Dla każdego etapu: problem i zakres → test negatywny (RED)
→ implementacja (GREEN) → format/Clippy/testy → GitHub
Supply Chain → commit w **main** → sprawdzony status CI →
aktualizacja README i dokumentacji. Żadnych dodatkowych gałęzi,
żadnego deklarowania wykonania bez artefaktów i testów.
**Aktualna decyzja produkcyjna: NO-GO.**


## Superseded by full ecosystem scope (26 PDFs)

The provisional **0.36–0.50** planning boundary above is superseded by
[ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md).
The full roadmap includes **all identified specialized swarms and
research tracks**, not just the initial 0.40/0.50 runtime/platform
targets. Each item retains an implementation/test/acceptance gate.
The stage numbers are roadmap IDs, not published Cargo versions.
