# OCAF — audyt naprawy agent artifact contract / 2026-10-30

## Źródło awarii

Repozytorium: `mojealterego/ocaf-runtime`, gałąź `main`. GitHub Actions [41058312131](https://github.com/mojealterego/ocaf-runtime/actions/runs/41058312131) wykazał 32 błędy `MISSING dedicated source folder` dla G02–G33. Warunki `rustc --test` dla ówczesnego walidatora przeszły 5/5, ale produkcyjny skan katalogu odrzucił brak źródeł. To nie był błąd kompilacji Rust.

`ocaf-components.tsv` zawierał 33 rekordy bez rozróżnienia `IMPLEMENTED` i `PLANNED`. Rejestr 111 agentów i 33 manifestów członkostwa (oddzielny `scripts/ocaf_agent_fleet_supervisor.py`) nie jest dowodem implementacji kodu ani uruchomienia agentów; supervisor deklaruje `active_runtime_agent_count=0` i `ostc_enabled=false`.

## Zmiana kontraktu

W `ocaf-components.tsv` wprowadzono pięć pól: ID, ścieżka, rodzaj, wymagany Zenoh, `implementation_state`. G01 = `IMPLEMENTED`; G02–G33 = `PLANNED`.

Rust walidator `scripts/ocaf_agent_artifact_gate.rs` zachowuje wymagania dotyczące Cargo, rzeczywistej struktury Rust, testów, dokumentacji oraz deklaracji transportu Zenoh, ale stosuje je do pozycji oznaczonych `IMPLEMENTED`. Planowana pozycja może nie mieć źródeł; jeśli katalog źródeł niespodziewanie pojawi się bez aktualizacji rejestru, gate kończy się FAIL. Nieznane stany, nieprawidłowe wpisy, duplikaty i traversal są odrzucane.

`--release` nie akceptuje planowanych komponentów i nadal odmawia certyfikacji G01 bez dowodu akceptacji. CI wymusza negatywny test tego zachowania; build nie uzyskuje fikcyjnej zgody produkcyjnej. To rozdziela zielony stan *spójności manifestu i gotowych artefaktów* od stanu *gotowości wszystkich agentów*.

## Potwierdzona weryfikacja

| Kryterium | Wynik | Dowód |
|---|---|---|
| RED: nowe testy przed implementacją | FAILED zgodnie z oczekiwaniem | [CI 41094841701](https://github.com/mojealterego/ocaf-runtime/actions/runs/41094841701) |
| Testy Rust walidatora | 9 PASS, 0 FAIL | [CI 41095114134](https://github.com/mojealterego/ocaf-runtime/actions/runs/41095114134) |
| Strukturalna walidacja zadeklarowanych implementacji | PASS, 33 rekordy | [CI 41095114134](https://github.com/mojealterego/ocaf-runtime/actions/runs/41095114134) |
| Próba przyjęcia brakujących agentów przez `--release` | Zablokowana; test negatywny PASS | [CI 41095114134](https://github.com/mojealterego/ocaf-runtime/actions/runs/41095114134) |
| Kod zaakceptowany przez CI | `0e4e806ee4a4b929f14db3d66773883467e1f09b` | [commit](https://github.com/mojealterego/ocaf-runtime/commit/0e4e806ee4a4b929f14db3d66773883467e1f09b) |

## Otwarta praca

G02–G33 wymagają własnego, kompletnego kodu, testów, kontraktów transportu i dokumentacji przed promocją na `IMPLEMENTED`. G01 nie ma dowodów certyfikacji release i nie jest automatycznie gotowy do produkcyjnego uruchomienia Zenoh. Nie dopisywać szablonowych funkcji wyłącznie dla zaliczenia walidatora.

Oznaczenie: **STRUCTURAL_CI_PASS**, **FLEET_IMPLEMENTATION_PARTIAL**, **RELEASE_DENIED**. Żadnego nowego `DEPLOYED` ani `OPERATOR_ACCEPTED`.
