# OCAF 111-agent fleet — structural gate no longer disabled by Python -O

Data 2026-11-26; repozytorium `mojealterego/ocaf-runtime`, gałąź `main`, bazowy commit `5f0a39808e3fc67b7491b98b0d194944f0184597`.

## Defekt i zmiana

Dotychczasowy `scripts/validate_agent_fleet_111.py` opierał wszystkie bramki kontraktu o instrukcje `assert`, które Python całkowicie usuwa po uruchomieniu z `python -O`. Nie stanowią one kontroli bezpieczeństwa. Nowa implementacja używa `require()` i typowanego `FleetValidationError`, porównuje identyfikatory i kolejność 111 pakietów, grupy 33 rojów, kontrole konfiguracji i ścieżek, sprawdza odrzucenie symlinków, brakujące pliki, input length, unikalność nazw i spójność Rust/Cargo/TOML. Nie może zaakceptować `ACTIVE`, `DEPLOYED`, `BUILT`, `ostc_enabled=true` ani `source_access=enabled` na podstawie samego rejestru. Nie modyfikuje plików agentów.

## Dowody

Python 3.13.5, lokalne syntetyczne środowisko zawierające 111 odrębnych kompletów katalogów i plików strukturalnych: `python -B -m unittest discover -s scripts/tests -p test_validate_agent_fleet_111.py -v` — **9/9 PASS, 0 FAIL**. W tym rzeczywiste wywołanie podprocesów `python -O` dla falsyfikacji aktywnego stanu i pozytywnego przebiegu, usunięcie pliku i symlink do zewnętrznego katalogu. `python -m py_compile` — PASS. Sprawdzenie nazw wszystkich 111 wpisów rzeczywistego `docs/agents/agent_registry_111.json` względem nowych reguł wykazało 0 konfliktów. SHA bloba lokalnie przetestowanego kodu: `34b321e4fda4db968ab24b2434c49b33408a4285`; testów: `d553711a0ef2826eb41093bed792fd12e624e743`.

## Ograniczenia

Ten gate jest wyłącznie **strukturalny**. Nie sprawdza rzeczywistej kompilacji 111 agentów, interfejsów OMAs/OMAt, kryptografii, izolacji pamięci, Zenoh, sandbox, modelu ani aktywacji. Deskryptorowa ochrona przed wrogim TOCTOU jest odrębną pracą. Nie uruchomiono pełnej regresji całego OCAF na tym SHA; wyniki GitHub Actions należy sprawdzić osobno. Żaden agent nie otrzymał certyfikatu, licencji ani produkcyjnego uprawnienia. `production_approved=false`.
