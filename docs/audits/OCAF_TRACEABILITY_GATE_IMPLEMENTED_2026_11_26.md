# OCAF v11 — wykonywalny gate macierzy 525 rekordów

Data: 2026-11-26. Gałąź: `main`. Macierz: `docs/roadmap/unfinished_requirement_matrix_2026_11_26.json`, blob SHA `11eb9e9c2c6b4030fe616f4dd5e7de9e26fbaf5c`.

Dodano `scripts/roadmap_traceability_gate.py`, 12 testów `scripts/tests/test_roadmap_traceability_gate.py` oraz workflow `.github/workflows/ocaf-roadmap-traceability.yml`. Gate weryfikuje SHA czterech manifestów, kolejność 525 rekordów, zachowanie ID, statusów, kryteriów odbioru, zależności i niepodnoszenie flag gotowości. Odrzuca JSON duplicate keys, symlinkowany plik danych, drift, usunięte etapy, podmianę statusu i fałszywe PASS.

**Testy lokalne:** `python3 -B -m unittest discover -s scripts/tests -p test_roadmap_traceability_gate.py -v` — 12/12 PASS, Python 3.13.5. `python3 -m py_compile scripts/roadmap_traceability_gate.py` — PASS. Testy obejmują uruchomienie `-O`. Bajty kodu i testów przesłane do GitHub mają identyczne Git blob SHA z lokalnie testowanymi plikami: `820fe5b5ceea0dd2fdc69335efed3722a5427dfd` / `5df194b206937598992f8d7c7d9176dec3fb29d8`.

**Weryfikacja źródeł:** porównanie dokładnie 525 rekordów do czterech manifestów GitHub — 0 niezgodności, zgodność wszystkich czterech blob SHA. Nie uruchomiono pełnych testów projektu, Rust E2E ani nie potwierdzono zielonego CI tego commita. To bramka spójności **źródeł**, a nie ukończenia Roadmapy. Wartość `production_approved=false` niezmieniona.

## Uzupełnienie testu bezpieczeństwa 2026-11-26

Nowy test odmowy wejścia przez symlinkowany katalog rodzica początkowo FAIL (13 testów, 1 FAIL), co ujawniło obejście samego `Path.is_symlink()` na pliku końcowym. Po sprawdzaniu każdego komponentu ścieżki 13/13 PASS w trybie zwykłym i `python3 -O`; `py_compile` PASS, Python 3.13.5. Ograniczenie: sprawdzenie komponentów nie gwarantuje odporności na wrogą podmianę TOCTOU; do pełnej izolacji wymagane są odczyty deskryptorowe na obsługiwanych hostach. Nowy SHA kodu `9ccbc01dd2ba111390cdd9dfb40641cc97391e11`, testów `38087129bf4a1ed12586bcb2af1f2fbc24c34983`.
