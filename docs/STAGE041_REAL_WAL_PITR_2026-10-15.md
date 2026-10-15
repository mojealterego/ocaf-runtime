# OCAF 0.41.3 — rzeczywisty PostgreSQL physical backup, archiwizacja WAL i PITR do LSN

**Data:** 2026-10-15. **Gałąź:** `main`. **Zakres:** rzeczywisty PostgreSQL 16 na jednym, efemerycznym Linux kernel GitHub Actions. **Stan:** lab PASS; pełna certyfikacja DR i produkcja NO-GO.

## Dowody testu

- [GitHub Actions 38737008767 — SUCCESS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38737008767).
- [Artefakt 11974052262](https://github.com/mojealterego/ocaf-runtime/actions/runs/38737008767/artifacts/11974052262), uploader ZIP SHA-256: `536f5aa33af7cb16908d1210f36fea467d2dadcd47f7a8ea76084ec383fc6db2`.
- Znacznik logów z rzeczywistego wykonania: `STAGE041_REAL_PGBASEBACKUP_ARCHIVED_WAL_LSN_PITR_POST_TARGET_EXCLUSION_PROVED`.
- Testy jednostkowe: **3 PASS**, sprawdzają brak wymaganych obserwacji oraz odrzucanie fałszywych deklaracji odtworzenia fizycznego hosta.
- Kod: `scripts/stage041_pitr.py`, `scripts/tests/test_stage041_pitr.py`, `.github/workflows/stage041-real-pitr.yml`.

## Co faktycznie zrobiono

1. Na tymczasowym, nieuprzywilejowanym serwerze PostgreSQL 16 utworzono tabelę i zapisano **25 rekordów**.
2. Wykonano prawdziwy `pg_basebackup -Fp -Xs`, ze skonfigurowanym `wal_level=replica`, `archive_mode=on`, `archive_command` i `fsync=on`.
3. Po base backupie zatwierdzono kolejne **10 rekordów** (stan docelowy **35**); odczytano faktyczne `pg_current_wal_lsn()`, nazwę segmentu oraz SHA-256 uporządkowanych danych.
4. **Po docelowym LSN** zatwierdzono dalsze **5 rekordów** — baza źródłowa posiadała **40**, ale ten późniejszy zapis musi zostać odrzucony podczas odtwarzania.
5. Wymuszono `pg_switch_wal()`; sprawdzono rzeczywisty zapis docelowego segmentu w osobnym katalogu archiwum WAL.
6. Wykonano **`pg_ctl stop -m immediate`**, sprawdzono niedostępność dawnego serwera TCP.
7. Utworzono **nowy** klaster z kopii fizycznej, dodano `recovery.signal`, `restore_command`, `recovery_target_lsn`, `recovery_target_inclusive=on` i `recovery_target_action=promote`.
8. Uruchomiono nowy proces PostgreSQL, zaobserwowano opuszczenie recovery, przywrócenie **dokładnie 35 rekordów**, **0 rekordów 36–40** oraz identyczne SHA-256 treści przy docelowym LSN.

Zapisano czas fizycznego backupu i ponownego startu do LSN, LSN, nazwę zarchiwizowanego WAL, liczbę segmentów WAL, boot ID i SHA-256 danych.

## Istotne ograniczenia

- Oba rzeczywiste procesy PostgreSQL uruchomiono **na jednym efemerycznym kernelu Linux**; IMMEDIATE crash to awaria *procesu*, nie twarda awaria całego hosta.
- Archiwum WAL było lokalne w prywatnym katalogu, a nie w zewnętrznym, szyfrowanym, niezmiennym magazynie.
- Dowiedziono **odtworzenia do precyzyjnego punktu WAL** oraz *wykluczenia nowszych transakcji*. Nie dowiedziono produkcyjnego HA, maksymalnego RTO, ciągłego RPO=0 ani disaster recovery poza hostem.
- Operator zrezygnował z niezależnych podpisów jako *blokady prac rozwojowych*, ale wymaganie rzeczywistej walidacji technicznej pozostaje.

**Wynik techniczny części PITR: PASS. Etap 0.41: nadal IN PROGRESS. `production_approved=false`.**
