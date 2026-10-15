# OCAF — DZIENNIK KOŃCA ETAPU I KONTROLA ZMIAN ROADMAPY

**Zasada bez wyjątków:** każdy domknięty lub zweryfikowany częściowo etap aktualizuje **tę samą** kanoniczną Roadmapę (`/ROADMAP.md`, lustrzane `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, manifest v11) i dostarcza realny raport CI. Nowe punkty dopisujemy po ostatnim numerze. Historycznie zgubionych wymagań nie przenosimy z ich miejsca.

## 2026-10-15 — RESTORED_EXISTING — 0.50–0.55 — Agents Builder Code i NoCode

- **Co odzyskano:** 10 wymaganych produktów **Agents Builder Cloud Windows macOS Android iOS, każdy Code i NoCode**; odrębne GUI i testy per platforma; wspólne IR, debugger, uprawnienia i publikacja.
- **Poprzedni błąd:** zredukowanie czterech platform desktop/mobile do dwóch zbiorczych opisów oraz błędne utożsamianie platformy *budowania agentów* z samymi *agentami*.
- **ID istniejące:** 0.50–0.55. **ŻADNYCH nowych numerów etapów dla tej naprawy**; nowe pozycje w planie numerujemy dopiero od 1.95.
- **Status produktów:** 10 × `NOT_IMPLEMENTED` na podstawie bieżącego drzewa źródeł; brak rzeczywistych GUI/build APK/IPA/desktop/web IDE. **Stan etapów 0.50–0.55:** `PLANNED_OR_UNVERIFIED`.
- **Rejestr utraconych wymagań:** 24 obszary do weryfikacji, w tym specjalistyczne Omni* swarms, podsystemy bezpieczeństwa, P0 WASI, P0 PG HA i disaster recovery.
- **Zatwierdzenie wdrożenia:** `production_approved=false`. To etap prac dokumentacyjno-inwentaryzacyjnych, nie produkcyjna implementacja aplikacji.
- **Dowody historyczne:** `docs/MASTER_ARCHITECTURE_26_PDFS.md`, pierwotna historyczna Roadmapa v9 i manifest v10, jawna dyrektywa użytkownika z 2026-10-15. Wymagany automatyczny test: v11 and v10 Roadmap guard. **Wynik CI**: pole uzupełnia się po rzeczywistym zakończonym workflow; nie wpisywać z wyprzedzeniem.

## Szablon obowiązkowego wpisu po KAŻDYM etapie

```yaml
event_type: STAGE_VERIFIED | STAGE_PARTIAL | RESTORED_EXISTING | NEW_REQUIREMENT
stage_id: "<existing ID or next free e.g. 1.95>"
date_utc: "<timestamp>"
title: "<literal scope>"
status_before: "<...>"
status_after: "<...>"
completed_requirements: []
remaining_requirements: []
source_files: []
commit_sha: "<40 lowercase hex>"
ci_run_url: "<finished actual github actions URL>"
ci_conclusion: "success|failure"
artifact_sha256: "<verified digest, or explicit NOT_AVAILABLE>"
real_backend: false
kernel_boot_ids: []
independent_hosts: 0
positive_tests: []
negative_tests: []
production_approved: false
```

**Wpis `STAGE_VERIFIED` nie może posiadać pustego zestawu testów i pustego potwierdzenia CI. `RESTORED_EXISTING` zawsze wskazuje pierwotny milestone i zachowuje stary zakres. `NEW_REQUIREMENT` musi używać kolejnego niezajętego numeru po 1.94.**


## 2026-10-15 — README_ROADMAP_RENDERED — numery 1–269 na samej górze repo

- Zgodnie z poleceniem operatora: pełna kolejno numerowana Roadmapa (195 etapów głównych + 64 podetapy + 10 pierwotnych produktów Agents Builder Code/NoCode) zostaje **w całości wypisana na samym początku README.md**, nie tylko linkowana.
- Każda pozycja posiada dosłowną etykietę `ZROBIONE — TEST W OGRANICZONYM ZAKRESIE` lub `DO ZROBIENIA`, jawny oryginalny numer ID, tytuł, zakres, kryterium i źródłowy status. `PASS` ograniczonego testu nie oznacza produkcyjnego ukończenia całości.
- Generator: `scripts/render_readme_roadmap.py`, blok chroniony znacznikami HTML, testy anty-utrata/anty-fikcyjny-status, CI sprawdza zgodność z dwoma manifestami. **Bez nowych numerów etapów**: produkty pierwotne są wyłącznie wydzielonymi pozycjami kontrolnymi.
- Przy zmianie statusu lub dopisaniu etapu generator jest obowiązkowym krokiem w tym samym commicie. Weryfikacja po wykonaniu przez GitHub Actions; nie deklarować wyniku przed zakończeniem.
