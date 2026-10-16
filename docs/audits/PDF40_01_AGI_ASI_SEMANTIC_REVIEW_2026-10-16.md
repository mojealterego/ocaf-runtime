# PDF 01/40 — Analiza AGI I Ludzkich AI (1).PDF — analiza stron 1–14

**Rodzaj źródła:** raport porównawczy / przegląd literatury, nie bezpośrednia specyfikacja wdrożeniowa OCAF. Każda wzmianka o obcym projekcie wymaga osobnej decyzji architektonicznej; nie tworzy automatycznie zobowiązania implementacji.

| Strona | Treść faktycznie omawiana | Powiązanie z istniejącą Roadmapą | Rozstrzygnięcie |
|---|---|---|---|
| 1 | AGI/ASI, modularne architektury poznawcze, pamięć, empatia, executive control | 0.67, 0.83, 1.95–1.96, 1.97 | TŁO / cel badawczy, brak odrębnego kontraktu wykonawczego |
| 2 | OpenCog Hyperon, Distributed Atomspace, hipergraf wiedzy | 1.95, 1.97 | REFERENCJA; brak obowiązku integracji Hyperon bez ADR |
| 3 | Symbolic guardrails, LIDA/GWT, EideticEngine, meta-cognition | 1.96, 2.04 | KANDYDAT R&D: kontrola planów i selekcja uwagi, nie dowód implementacji |
| 4 | temporal memory, Cognee, Graphiti, Zep, entity linking, BM25 | 0.67, 1.97 | POKRYTE TEMATYCZNIE; osobne testy temporalnej proweniencji pozostają wymagane |
| 5 | Porównanie pamięci; symulacje społeczne generative agents | 0.67, 0.77 | REFERENCJA; nie traktować symulacji emocji jako dowodu ludzkiej świadomości |
| 6 | EmoLLM, teoria umysłu, wnioskowanie o intencjach | 0.83, 2.03 | POKRYTE JAKO R&D; opt-in, niepewność i zakaz ukrytej inferencji cech wrażliwych |
| 7 | DSPy, BootstrapFewShot, MIPRO, GEPA, optymalizacja i kompresja kontekstu | 1.98, 0.98, 1.99 | KANDYDAT EWALUACYJNY; walidacja na holdout, ochrona przed data leakage |
| 8 | AI Scientist: hipotezy, sandbox eksperymentów, LaTeX | 2.01 | POKRYTE TEMATYCZNIE; reprodukowalność i aprobata człowieka |
| 9 | BabyAGI/taskweaving, GodMode, OpenHands | 2.00, 2.02, 2.06 | POKRYTE TEMATYCZNIE; DAG, approval gates, izolacja narzędzi |
| 10 | SWE-bench Verified i deklarowane wyniki modeli | 1.99 | REFERENCJA; liczby benchmarkowe niezweryfikowane, wymagają zewnętrznych źródeł i protokołu testowego |
| 11 | mini-SWE-agent, GLM i metryki; ograniczenia benchmarków | 1.99 | REFERENCJA; nie przypisywać benchmarków OCAF |
| 12 | Podsumowanie pamięci, ToM, DSPy i AI Scientist | 1.95–1.98, 2.01, 2.03 | PODSUMOWANIE wcześniejszych tematów; bez nowego obowiązku |
| 13 | Bibliografia, źródła 19–52 | 1.97 | BIBLIOGRAFIA; weryfikacja linków osobno, nie tworzyć etapów z samych nazw |
| 14 | Bibliografia, źródła 53+ | 1.97, 2.02 | BIBLIOGRAFIA; brak nowego wymagania normatywnego |

## Wniosek dla PDF 01
Po sprawdzeniu bieżącego manifestu v11 nie wykryto jednoznacznego obowiązku nowego etapu ponad istniejące 0.67, 0.68, 0.77, 0.83, 0.98, 1.95–2.06. Wcześniejsza tabela pomijała istotne etapy 1.95 (Hyperon), 1.96 (ACT-R/Soar/LIDA), 1.98 (DSPy), 2.04 (EideticEngine) i 2.05 (Agent-Zero); ten błąd został skorygowany. Nie ma podstaw do dopisywania 2.07 na podstawie samego PDF 01. **To jest klasyfikacja zawartości konkretnego pliku, nie dowód implementacji tych etapów.** Rozdzielono zewnętrzne wyniki benchmarków od dowodów projektu OCAF. Dalszy audyt musi stosować tę samą metodę do dokumentów 02–40.

## Korekta audytowa: porównanie z manifestem v11

Przeprowadzono kontrolę aktualnych wpisów `main[]` w `docs/roadmap/canonical_v11_inventory.json` na gałęzi `main`. Istnieją już **osobne** etapy 1.95 Hyperon/MeTTa, 1.96 ACT-R/Soar/LIDA, 1.97 pamięć agentowa, 1.98 DSPy, 1.99 SWE-bench, 2.00 taskweaving, 2.01 AI Scientist, 2.02 OpenHands, 2.03 ToM, 2.04 EideticEngine, 2.05 Agent-Zero i 2.06 NuNet/PRIMUS/GodMode. Dlatego dopisanie tych samych funkcji jako nowych etapów stanowiłoby duplikację historycznej Roadmapy.

**Istotne rozróżnienie:** tekst źródłowy jest przeglądem technologii, zawiera ocenne i niezweryfikowane stwierdzenia o AGI/ASI oraz benchmarkach; nie potwierdza implementacji OCAF. Wpisy manifestu są planem, a nie dowodem działania. Zakończenie kontroli zakresu PDF 01 nie oznacza zamknięcia audytu implementacji, testów ani niezależnego odbioru.
