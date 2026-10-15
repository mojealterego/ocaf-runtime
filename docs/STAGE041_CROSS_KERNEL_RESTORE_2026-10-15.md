# OCAF 0.41.2 — rzeczywisty backup PostgreSQL przeniesiony między dwoma kernelami Linux

Data: **2026-10-15**. Gałąź: `main`.

## Co wykonano — dwa prawdziwe joby PostgreSQL na osobnych efemerycznych hostach GitHub

### Host źródłowy A

1. PostgreSQL 16, **rzeczywisty proces serwera** pod kontem `postgres`, utworzono 25 unikalnych rekordów trwałych.
2. Wykonano prawdziwy `pg_dump --format=custom`; lokalnie wykonano także `pg_restore` i potwierdzono tę samą liczbę oraz SHA-256 uporządkowanej zawartości.
3. Wymuszono negatywny test naruszenia integralności zmodyfikowanej kopii (odrzucony przez porównanie SHA-256).
4. Zatrzymano testowy proces serwera i **dopiero potem** wystawiono `snapshot.pgcustom`, `source.json` oraz `manifest.json` w artefakcie GitHub Actions. Manifest wiąże sumę SHA-256 snapshotu i raportu, 16-znakowy nonce oraz rzeczywiste `/proc/sys/kernel/random/boot_id`.

### Host odtworzeniowy B

1. Uruchomiono **inny** job `ubuntu-24.04`; sprawdzono, że jego rzeczywisty kernel `boot_id` jest **różny** od źródłowego.
2. Pobierano artefakt z zakończonego jobu A, kopiowano do prywatnego katalogu konta `postgres`, sprawdzano rzeczywiste `PGDMP` custom archive magic, oba SHA-256, rozmiar i spójność raportu.
3. **Rzeczywisty negatywny test na B:** celowo zmodyfikowany bajt pobranej kopii `pgcustom` musi być odrzucony; próba ponownego użycia identyfikatora źródłowego kernela jako odtworzeniowego również musi zostać odrzucona.
4. Utworzono **nowy** klaster PostgreSQL i nową bazę na B, wykonano rzeczywisty `pg_restore --exit-on-error`.
5. Zweryfikowano liczbę 25 rekordów i identyczny SHA-256 uporządkowanego odczytu przed/po.
6. Zapisano niezależny raport o rzeczywistym `pg_restore_duration_ms`, identyfikatorach kerneli, hashach i fladze `production_approved=false`.

## Potwierdzenie z GitHub Actions

- **Run 38734668015: SUCCESS**, 2/2 rzeczywiste joby Linux/PG zakończone bez błędu — [otwórz](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015).
- [Źródłowy artefakt 11972505183](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015/artifacts/11972505183), uploader ZIP SHA-256: `5e51105993d1c2cfa0cf8ebfca09ae856d18f822320970488acabac335ae61d2`.
- [Odtworzony artefakt 11972585218](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015/artifacts/11972585218), ZIP SHA-256: `6aaaa1bcec67d5018be97a6c8222c7f33b4ba4e26ab344be113a481c5d18f145`.
- Źródłowy kernel boot ID: `b85d3ae5-2c48-4368-ba5e-081ac7c975e4`.
- Weryfikowalne znaczniki w logach CI: `STAGE041_REAL_DESTINATION_REJECTED_TAMPERED_ARCHIVE_AND_REUSED_KERNEL_ID` i `STAGE041_REAL_DIFFERENT_LINUX_KERNEL_POSTGRESQL_SHA256_RECOVERY_PROVED`.
- Testy jednostkowe walidacji A/B: **6/6 PASS**, obejmują błędną sumę, taki sam boot ID, uszkodzony JSON, zmianę liczby rekordów, symlink i fałszywą deklarację gotowości produkcyjnej.
- Poprzedni przebieg [38734340050](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734340050) również miał **SUCCESS** dla rzeczywistego cross-kernel restore bez dodatkowej negatywnej próby na docelowym pliku. Poprawiono także rzeczywisty błąd importu sterownika w rozszerzonej próbie [38734554107](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734554107) (`ModuleNotFoundError`, wynik FAILED); końcowy run **38734668015 PASS**.

## Granice wyniku

- To **dwa odrębne efemeryczne systemy GitHub-hosted** (oddzielne Linux `boot_id`); nie wolno twierdzić, że był to wstrzyknięty power loss trwałego produkcyjnego serwera lub regionalna awaria centrum danych.
- Dane trafiają przez artefakt GitHub Actions, **nie** przez zaszyfrowany, kluczem produkcyjnym zabezpieczony zewnętrzny backup store. SHA-256 koreluje integralność, ale **nie** jest sam w sobie niezależnym podpisem PKI ani dowodem autentyczności administratora backupu.
- `rpo_at_backup_boundary_rows_lost=0` mierzy **dokładny stan na chwilę snapshotu**, a nie gwarancję ciągłego zerowego RPO podczas awarii; `pg_restore_duration_ms` mierzy tylko czas przywrócenia bazy, nie pełny RTO infrastruktury.
- Odbudowa dwóch instancji opiera się na świeżym `initdb`, nie na automatycznej replikacji online. Ochrona przed split-brain/leader fencing i odporność pod długotrwałym obciążeniem nadal wymagają testów z docelową topologią.
- **Operator zatwierdził kontynuowanie rozwoju bez zewnętrznych audytów/podpisów jako warunku postępu.** Nie zmienia to faktu braku dowodu produkcyjnego HA/DR.

**Status 0.41: wykonano i zweryfikowano lab cross-kernel restore; pełny etap nadal IN PROGRESS. Produkcja NO-GO.**

## Kod i odtwarzalność

- `scripts/stage041_backup_restore.py` — prawdziwy PostgreSQL, zapis/backup lokalny, integralność i przywrócenie.
- `scripts/stage041_cross_host_restore.py` — niezależna walidacja źródła, nowe `initdb`, rzeczywisty `pg_restore` i porównanie danych na innym `boot_id`.
- `scripts/tests/test_stage041_cross_host_restore.py` — przypadki negatywne.
- `.github/workflows/stage041-cross-host-restore.yml` — dwa szeregowo uruchamiane joby na niezależnych efemerycznych runnerach Linux.
