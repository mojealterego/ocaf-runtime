# OCAF Agents Builder — KaiOS 2.5

Status: **source implementation committed; packaging CI and real device/simulator execution unverified**.

A side-effect-free canonical typed DAG validator is included in `canonical-graph.js` and tested separately. The v1 editor still uses its restricted node-only schema; canonical project editing and execution are not integrated.\n\nThis is a small, keypad-friendly, offline **subset** of OCAF Agents Builder, not a full agent compiler or runtime. It targets the KaiOS 2.5 packaged web-app model (`manifest.webapp`). KaiOS 3.x compatibility has not been established and must be tested separately.

## Implemented

- Code: edit a constrained JSON project (version 1, project_id, up to 32 unique node IDs).
- NoCode: add and list typed-identifier nodes; the same JSON document is displayed in Code mode.
- Local: persist and reopen validated projects in browser localStorage.
- Cloud: **disabled and fail-closed**. There is no authentication, API key handling, model inference, GGUF, tool execution, cross-device synchronization, or deployment. A separate static typed DAG validator is bundled, but it is not yet wired into the v1 node-only editor and is not a signed compiler.
- Keypad shortcuts: SoftLeft save, SoftRight open; controls are standard focusable HTML elements.

## Limitations and security

The JSON validator rejects unsupported project/node properties rather than silently stripping unknown graph semantics. This is a deliberately strict schema boundary.

The simplified KaiOS schema is **not** the canonical OCAF typed DAG and is not interoperable with the desktop project's complete schema. The UI must not claim semantic Code/NoCode round-trip with the canonical compiler. The app requests no privileged KaiOS permissions and does not evaluate project JSON as code. Local storage is not encrypted; do not enter secrets.

## Build and acceptance

`.github/workflows/agents-builder-kaios.yml` runs source contract tests and JavaScript syntax checks and creates a ZIP of `manifest.webapp`, `index.html`, `app.js`, `style.css`, `canonical-graph.js`. This is an **unsigned web-app package**, not proof of installation.

Acceptance still requires: pinned KaiOS SDK/device target, install and launch, keypad navigation E2E, persistence after restart, storage quota handling, accessibility, and independent negative tests. For KaiOS 3.x, separately verify manifest and runtime requirements before claiming compatibility.

## Kanoniczna walidacja DAG w interfejsie

Sekcja **Kanoniczny DAG OCAF (walidacja)** przyjmuje ścisły JSON projektu `schema_version=1` i jawny identyfikator tenanta. Przycisk **Sprawdź DAG** wywołuje lokalny, bezefektowy walidator `canonical-graph.js` i wyświetla uporządkowany plan lub komunikat błędu. Identyfikator tenanta jest tu wpisywany przez użytkownika, **nie jest to uwierzytelniona tożsamość**. Wynik nie jest podpisanym manifestem, nie wykonuje narzędzi i nie jest dowodem autoryzacji serwerowej.

Weryfikacja: `node apps/agents-builder-kaios/tests/canonical-ui.cjs` oraz `node apps/agents-builder-kaios/tests/canonical-graph.cjs`. Tryb Code/NoCode v1 nadal zachowuje własny, uproszczony format; pełna konwersja semantyczna do kanonicznego DAG nie została wdrożona.

## Lokalny zapis kanonicznego DAG

Formularz kanonicznego DAG ma osobne pola `Tenant` i `ID projektu DAG`. Zapis i odczyt używają klucza `ocaf-kaios-canonical-v1:<tenant>:<project>`; przed zapisem oraz po odczycie wykonywana jest walidacja grafu i zgodności identyfikatorów. Błędy walidacji nie nadpisują poprzednio zapisanego projektu. Pamięć `localStorage` nie jest szyfrowana i nie zapewnia izolacji bezpieczeństwa między uwierzytelnionymi tenantami; jest to jedynie lokalna przestrzeń nazw. Nie przechowuj sekretów ani danych poufnych.

## Kanoniczny NoCode — operacje grafowe

Przycisk **Nowy graf** tworzy poprawny, dwuwęzłowy projekt kanoniczny `Input → Output` z portami `text`. Przycisk **Dodaj Transform** wstawia nowy węzeł w jedyne połączenie prowadzące do `Output` (wymagane porty `out` i `in`, typ `text`). Po operacji następuje pełna walidacja kanonicznego DAG. Błędne identyfikatory, duplikaty i nieobsługiwane grafy są odrzucane bez modyfikacji edytora. Jest to ograniczony edytor liniowych transformacji, **nie** kompletny wizualny edytor dowolnych grafów, narzędzi ani uprawnień.

## Odwracalne operacje NoCode

**Usuń Transform** usuwa wskazany identyfikatorem węzeł typu `transform` z dokładnie jednym wejściem i jednym wyjściem `text`, a następnie łączy jego poprzednika z następnikiem. Zmiana jest atomowa na poziomie stanu formularza: edytor aktualizuje JSON dopiero po pomyślnej walidacji całego grafu. Odmowa następuje dla innych typów węzłów, nieobsługiwanych portów i nieprawidłowej topologii. To nie jest mechanizm cofania dowolnych zmian ani pełny edytor DAG.

## Typowane porty w NoCode

Formularz umożliwia wybór typu portu `text`, `json` albo `bytes` dla nowego grafu. Wstawienie transformacji dziedziczy typ z połączenia prowadzącego do `Output`; usuwanie transformacji zachowuje typ istniejących portów. Wybór nieznanej wartości jest odrzucany bez modyfikacji projektu. To statyczne sprawdzanie kontraktów, a nie uruchamianie transformacji.

## Import i eksport kanonicznego DAG

**Eksportuj DAG** kopiuje zwalidowany projekt do pola tekstowego, skąd użytkownik może skopiować JSON do innego środowiska. **Importuj DAG** weryfikuje rozmiar (maks. 128 KiB), identyfikator projektu, tenant i cały kanoniczny graf przed aktualizacją edytora. Nie wykonuje kodu, nie wysyła danych do sieci i nie zapisuje importu automatycznie. To transfer JSON przez schowek użytkownika, nie eksport podpisanego manifestu OCAF ani pakietu wykonywalnego.

## Węzły Tool w NoCode

**Dodaj Tool** tworzy typowany węzeł narzędzia pomiędzy poprzednikiem a `Output`, deklaruje `tool_name`, przydziela uprawnienie w `grants`, zwiększa budżet o zadeklarowany koszt oraz inkrementuje `revision`. **Usuń Tool** usuwa wskazany węzeł, przywraca połączenie, odejmuje jego koszt i usuwa grant tylko wtedy, gdy żaden inny węzeł nie żąda tego narzędzia. Każda operacja przechodzi walidację kanonicznego DAG przed aktualizacją edytora. To wyłącznie projektowanie statycznego kontraktu — aplikacja nie uruchamia narzędzi, nie wydaje uprawnień w backendzie i nie zarządza sekretami.

## Zmiana typu portów grafu

Przycisk **Zmień typ grafu** migruje wszystkie porty jednokanałowego grafu do wybranego typu `text`, `json` lub `bytes`. Zmiana jest atomowa, wymaga poprawnego kanonicznego DAG, zwiększa rewizję i ponownie uruchamia walidator. Grafy z wieloma portami nie są obsługiwane przez tę operację NoCode. Migracja zmienia kontrakt statyczny, **nie konwertuje danych ani nie wykonuje narzędzi**.

## Edycja kosztu Tool

Przycisk **Zmień koszt Tool** aktualizuje `max_cost_minor` wskazanego węzła `Tool` oraz budżet całego DAG o różnicę między nową i starą wartością. Operacja jest atomowa, ograniczona do wartości całkowitych dodatnich nieprzekraczających 10¹², ponownie waliduje projekt i zwiększa rewizję. Jest to edycja deklaratywnego kosztu, **nie naliczenie płatności ani wykonanie narzędzia**.
