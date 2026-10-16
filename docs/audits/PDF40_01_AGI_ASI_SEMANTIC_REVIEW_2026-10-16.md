# PDF 01/40 — Analiza AGI I Ludzkich AI (1).PDF — analiza stron 1–14

**Rodzaj źródła:** raport porównawczy / przegląd literatury, nie bezpośrednia specyfikacja wdrożeniowa OCAF. Każda wzmianka o obcym projekcie wymaga osobnej decyzji architektonicznej; nie tworzy automatycznie zobowiązania implementacji.

| Strona | Treść faktycznie omawiana | Powiązanie z istniejącą Roadmapą | Rozstrzygnięcie |
|---|---|---|---|
| 1 | AGI/ASI, modularne architektury poznawcze, pamięć, empatia, executive control | 0.67, 0.83, 1.97 | TŁO / cel badawczy, brak odrębnego kontraktu wykonawczego |
| 2 | OpenCog Hyperon, Distributed Atomspace, hipergraf wiedzy | 0.67, 1.97 | REFERENCJA; brak obowiązku integracji Hyperon bez ADR |
| 3 | Symbolic guardrails, LIDA/GWT, EideticEngine, meta-cognition | 0.67, 0.68, 0.83 | KANDYDAT R&D: kontrola planów i selekcja uwagi, nie dowód implementacji |
| 4 | temporal memory, Cognee, Graphiti, Zep, entity linking, BM25 | 0.67, 1.97 | POKRYTE TEMATYCZNIE; osobne testy temporalnej proweniencji pozostają wymagane |
| 5 | Porównanie pamięci; symulacje społeczne generative agents | 0.67, 0.77 | REFERENCJA; nie traktować symulacji emocji jako dowodu ludzkiej świadomości |
| 6 | EmoLLM, teoria umysłu, wnioskowanie o intencjach | 0.83, 2.03 | POKRYTE JAKO R&D; opt-in, niepewność i zakaz ukrytej inferencji cech wrażliwych |
| 7 | DSPy, BootstrapFewShot, MIPRO, GEPA, optymalizacja i kompresja kontekstu | 0.68, 0.98, 1.99 | KANDYDAT EWALUACYJNY; walidacja na holdout, ochrona przed data leakage |
| 8 | AI Scientist: hipotezy, sandbox eksperymentów, LaTeX | 2.01 | POKRYTE TEMATYCZNIE; reprodukowalność i aprobata człowieka |
| 9 | BabyAGI/taskweaving, GodMode, OpenHands | 2.00, 2.02, 2.06 | POKRYTE TEMATYCZNIE; DAG, approval gates, izolacja narzędzi |
| 10 | SWE-bench Verified i deklarowane wyniki modeli | 1.99 | REFERENCJA; liczby benchmarkowe niezweryfikowane, wymagają zewnętrznych źródeł i protokołu testowego |
| 11 | mini-SWE-agent, GLM i metryki; ograniczenia benchmarków | 1.99 | REFERENCJA; nie przypisywać benchmarków OCAF |
| 12 | Podsumowanie pamięci, ToM, DSPy i AI Scientist | 0.67, 2.01, 2.03 | PODSUMOWANIE wcześniejszych tematów; bez nowego obowiązku |
| 13 | Bibliografia, źródła 19–52 | 1.97 | BIBLIOGRAFIA; weryfikacja linków osobno, nie tworzyć etapów z samych nazw |
| 14 | Bibliografia, źródła 53+ | 1.97, 2.02 | BIBLIOGRAFIA; brak nowego wymagania normatywnego |

## Wniosek dla PDF 01
Nie wykryto jednoznacznego obowiązku nowego etapu ponad istniejące 0.67, 0.68, 0.77, 0.83, 0.98, 1.97, 1.99, 2.00–2.03, 2.06. **To jest klasyfikacja zawartości konkretnego pliku, nie dowód implementacji tych etapów.** Rozdzielono zewnętrzne wyniki benchmarków od dowodów projektu OCAF. Dalszy audyt musi stosować tę samą metodę do dokumentów 02–40.