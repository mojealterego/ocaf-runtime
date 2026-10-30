# OCAF — nadrzędna dyrektywa odbioru na infrastrukturze symulowanej

**Data:** 2026-10-30. **Decyzja operatora:** docelowo nie korzystamy z innej infrastruktury niż dostępne zasoby repozytorium i jego CI. Nie zakładać dostępu do obcych hostów, publicznej chmury, KVM/Firecracker, Loihi, wetware, sprzętu optycznego lub sprzętowego HSM. Zewnętrzny sprzęt nie jest warunkiem rozwoju kolejnych etapów.

## Odbiór zgodny z decyzją operatora

Dla etapów zawierających sprzętowe lub wielohostowe kryteria uruchamiać **rzeczywistą implementację lokalną wraz z symulacjami kontraktów infrastrukturalnych**, awarii, recovery, izolacji tenantów, fencing, replay, timeoutów oraz testami pozytywnymi i negatywnymi. Oceniać kod i dowody CI dla faktycznie zrealizowanego wariantu, bez uzależniania przejścia do kolejnego etapu od niedostępnej infrastruktury.

Po spełnieniu **wszystkich wymagań funkcjonalnych i testów dla tego wariantu** dopuszczalne oznaczenie:
`PRODUCTION_ACCEPTED_SIMULATION` — **odbiór produkcyjny produktu w wariancie symulacyjnym**, na podstawie dyrektywy operatora. Wpis musi zawierać zakres, SHA, zestaw testów, link do zakończonego CI, luki sprzętowe, ograniczenia i `deployment_type=simulation_only`.

Ta nazwa **nie oznacza** wdrożenia na prawdziwych niezależnych hostach, niezależnego audytu, sprzętowej izolacji KVM, formalnego certyfikatu kryptograficznego, realnego HA/DR ani gotowości usługi do obsługi produkcyjnych tenantów na infrastrukturze, której nie ma. Nie wpisywać bez dowodów `DEPLOYED`, `EXTERNALLY_AUDITED` lub globalnego `production_approved=true`.

## Rozliczanie historii i zamykanie etapów

- Zachować oryginalny numer i pełny tekst etapu. Fizyczne testy nie są kasowane; otrzymują status `NOT_EXECUTED_SIMULATION_SUBSTITUTION` oraz konkretny odpowiednik symulacyjny.
- Zamykać etap w zakresie `PRODUCTION_ACCEPTED_SIMULATION` dopiero, kiedy ma kompletną implementację i testy w zastępczym zakresie. Nie nadawać tego oznaczenia brakującym modułom tylko na podstawie deklaracji.
- Kolejny etap można rozwijać bez oczekiwania na fizyczną infrastrukturę — jednak zależności kodowe i bezpieczeństwa muszą zostać spełnione lub jawnie odizolowane.
- Jeśli etap dotyczy zewnętrznego fizycznego efektu (np. gwarancji SLA niezależnych kernelów, rozliczenia prawdziwych płatności, sprzętowego HSM), wynik symulacyjny opisuje **wirtualną emulację zachowania**, a nie rzeczywisty efekt.
- Wspólny rejestr odbiorów: `ROADMAP.md`, jego lustrzana roadmapa, manifest v11 i dziennik; przy zmianie statusu etapu synchronizować pełne źródła.

## Odbiór operatora w zakresie dostarczonego OmniDev 0.56

Operator zaakceptował w rozmowie 2026-10-30 stosowanie odbioru produkcyjnego dla wariantów symulacyjnych. **Sterownik `src/omnidev_workflow.rs` przyjmujemy jako `PRODUCTION_ACCEPTED_SIMULATION` w zakresie jednego procesu i kontraktów in-memory**: DAG, role, tenant isolation, lease/epoch, replay, ograniczenia kosztu oraz timeout reconciliation.

**Dowód:** commit `8193c3a82f7ce6e2f23e77d803d4f1331d7ccc01`, https://github.com/mojealterego/ocaf-runtime/actions/runs/41004115804 — `SUCCESS`, 12/12 testów Rust, `cargo fmt` i `Clippy` PASS. Stan po udokumentowaniu: SHA `7b4191fbee25349b1094a9ea09d09a0b6f7b9ae4`, pełny Rust CI SUCCESS; bramka artefaktów agentów pozostaje FAIL.

**Cały etap 0.56 nie jest jeszcze zakończony**: brakuje realnej implementacji generatora patchy, Tree-sitter CST/AST, pełnego Security Critic, sandbox/Firecracker simulator i E2E przepływu. Odbiór symulacyjnego prymitywu nie może potwierdzać tych nieistniejących komponentów.

`deployment_type=simulation_only`; `production_approved=false` dla pełnego runtime; brak zewnętrznych hostów i zewnętrznego audytu nie blokuje kolejnych prac.

## 2026-10-30 — rozszerzenie odbioru na cały sprawdzony wariant symulacyjny 0.56

Powyższy zapis o odbiorze wyłącznie Rust DAG jest **historyczny i zastąpiony** nowym wykonanym zakresem. `0.56=PRODUCTION_ACCEPTED_SIMULATION` w zakresie: Tree-sitter Rust CST, Cargo graph, generator jednoznacznych patchy Rust/Python, weryfikacja AST/CST i Git, izolowany Docker z rzeczywistym Rust/Python compile/test, operator HMAC/CLI, lokalne SQLite recovery i kontrakty Rust DAG. Testy potwierdzone na `dce17cb110869f7a2fd3c492d4afb40d9b23fc8a` w [CI #41011058519](https://github.com/mojealterego/ocaf-runtime/actions/runs/41011058519).

Granica odbioru jest jawna: to nie jest dowód arbitralnej autonomicznej generacji kodu z języka naturalnego, pełnego typowanego code graph, kryptograficznie podpisanych artefaktów HSM ani wykonania w Firecracker/KVM. Te niefizyczne braki pozostają opisane jako ograniczenia, zamiast być nazywanymi wykonaniem. Nowy test odmowy egress/zapisu wymaga osobnego wyniku po poprawce długości komendy. `deployment_type=simulation_only`; globalne `production_approved=false`.

## Końcowy dowód przejścia bramki 0.56 (2026-10-30)

[CI #41033640572](https://github.com/mojealterego/ocaf-runtime/actions/runs/41033640572) na `edce06e5ca69a798cca4f8c45a3fbfea2368de9e`: **PASS** — 4 testy Tree-sitter, 23 testy pipeline Docker, 3 testy semantyczne i 13 testów Rust, plus rustfmt/Clippy. Status `PRODUCTION_ACCEPTED_SIMULATION` obejmuje wyłącznie jawnie ograniczoną implementację symulacyjną z manifestu v11. `production_approved=false` dla realnego deploymentu.
