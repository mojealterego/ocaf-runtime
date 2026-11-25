# OCAF Runtime — źródłowa macierz pozostałych wymagań

Data: 2026-11-26. Bazowy SHA `92df9addd75b323aff92ee37f85f5a7eea01529d`. Tree SHA `4571ce268079f4d5af3720d583f01c34ca7a7a12`. Zakres wyłącznie `mojealterego/ocaf-runtime` i `main`.

## Pokrycie źródeł

Macierz `docs/roadmap/unfinished_requirement_matrix_2026_11_26.json` przenosi bez pomijania rekordów źródłowe opisy, kryteria odbioru, stany i znane ścieżki z czterech istniejących manifestów:

| Rejestr | Wpisy |
|---|---:|
| Etapy główne | 334 |
| Etapy techniczne | 64 |
| Repozytoria referencyjne | 33 |
| Wymagania AB-OM | 20 |
| Warianty Agents Builder | 10 |
| Luki `missing_capabilities_v11.json` | 64 |
| **Łącznie rekordy źródłowe** | **525** |

**525 nie oznacza 525 unikalnych zadań:** wymagania, warianty, luki i adaptacje mogą przypadać na te same etapy. Nazwy referencyjne pozostawiono oryginalne. Macierz zachowuje oryginalne ID etapów; nie zmienia ich numeracji. Pole `implementation_verified_by_this_matrix=false` jest celowe; żadna istniejąca wartość statusu nie została automatycznie awansowana do PASS.

## Dowody i ograniczenia

Zebrano wszystkie 525 rekordów z deklarowanych źródeł (333? nie — dokładnie 334 główne) i porównano nazwy ścieżek występujące w źródłach z drzewem Git (8351 pozycji). `in_git_tree=true` oznacza wyłącznie obecność wskazanego pliku, nie jego poprawność lub aktualny wynik testów. Zachowano oryginalny `acceptance`, `negative`, `repair` i deklarację techniczną `engineering_progress` bez wymyślania kodu, dowodów i statusów. Wykryto 0 odwołań zależności, których nie można odnaleźć w dwóch listach etapów manifestu; wymagają osobnej analizy, nie automatycznej poprawki.

**Nie wykonano** analizy i implementacji każdego niewykonanego kryterium, pełnych testów, buildów, integracji i wdrożeń. Macierz służy do identyfikacji zaległości; nie oznacza zamknięcia Roadmapy. Nie utworzono fikcyjnych adapterów, faktur, certyfikatów ani nowych gałęzi. `production_approved=false`.
