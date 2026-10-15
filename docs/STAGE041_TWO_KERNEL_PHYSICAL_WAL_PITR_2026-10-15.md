# OCAF 0.41.4 — rzeczywisty physical PostgreSQL PITR na dwóch kernelach Linux

**Data:** 2026-10-15. **Gałąź:** `main`. **Zakres:** dwie oddzielne efemeryczne instancje Linux / PostgreSQL 16 na runnerach GitHub Actions. **Wynik CI:** PASS obu jobów; **nie** produkcyjne DR.

## Potwierdzenie testu

[**GitHub Actions 38738198412 — SUCCESS, 2/2 niezależne Linux jobs**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412).

**Host źródłowy A:**
1. Prawdziwy PostgreSQL z danymi 25 rekordów.
2. **Fizyczny `pg_basebackup`** wykonany przed kolejnymi 10 zatwierdzonymi transakcjami; zarchiwizowany prawdziwy segment WAL i zapisany jego docelowy LSN po stanie 35 rekordów.
3. Zatwierdzono dodatkowe 5 transakcji po docelowym LSN, następnie przeprowadzono `pg_ctl stop -m immediate` pierwotnego serwera. Lokalna rekonstrukcja PostgreSQL z base backup i WAL potwierdziła odtworzenie stanu z 35 rekordami, bez 5 późniejszych rekordów.
4. Fizyczna kopia bazowa i katalog zarchiwizowanego WAL zostały spakowane oddzielnie do `basebackup.tar.gz` i `archived-wal.tar.gz`.
5. SHA-256 obu archiwów, SHA-256 źródłowego raportu, kernel `boot_id`, `recovery_target_lsn`, SHA-256 oczekiwanych danych i jednorazowy `run_nonce` zapisano w ścisłym, limitowanym `manifest.json`.

**Host odtworzeniowy B:**
1. W osobnym Linux Actions jobie pobrano niezmienne artefakty źródłowe, zweryfikowano pełne SHA-256, oryginalny raport i odmienny `/proc/sys/kernel/random/boot_id`.
2. Wykluczono ścieżki `../`, linki symboliczne i specjalne pliki TAR, duplikaty ścieżek, nieprawidłowe typy plików, przekroczenia limitów i inne niebezpieczne archiwa.
3. **Negatywne testy na rzeczywistym pobranym WAL:** modyfikacja bajtu tarballa spowodowała odmowę; również podszywanie się pod źródłowy `boot_id` odrzucono.
4. Wypakowano poprawną fizyczną kopię PostgreSQL i prawdziwe zarchiwizowane WAL do prywatnej ścieżki, utworzono `recovery.signal` i podano `restore_command`, `recovery_target_lsn`, `recovery_target_action=promote`.
5. Uruchomiono **nowy, rzeczywisty postmaster PostgreSQL** i odtworzono WAL aż do oryginalnego LSN: dokładnie **35/35 rekordów**, **0 rekordów 36–40**, **SHA-256 danych taki sam** jak na źródle; `pg_is_in_recovery()=false`.
6. Zmierzono `pitr_duration_ms` i zachowano JSON dowodowy wraz z sumami SHA-256 i rozdzielnymi kernel `boot_id`.

## Dowody CI i SHA-256 ZIP

- **Run:** [38738198412](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412), oba zadania `success`.
- **Source artifact 11973973900:** [pobierz z GitHub](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412/artifacts/11973973900), SHA-256 ZIP `2f333dfabb982ed21578cec55ac0de8bdcf250768b9f1eb07c1e5534037d2ed9`.
- **Recovery artifact 11974568641:** [pobierz z GitHub](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412/artifacts/11974568641), SHA-256 ZIP `c799f58b716bf29fa23ce9fb61e53c665ca8a62a82a4473a6c690e9920b37ce6`.
- **5/5 Python regression tests PASS**, w tym fałszywe tożsamości kernela, nieautoryzowane metadane, zmiana bajtów archiwum, symlink w TAR i błąd odtwarzania wierszy.
- Zweryfikowane znaczniki z logów: `STAGE041_REAL_SOURCE_PHYSICAL_PGBASEBACKUP_AND_WAL_ARCHIVE_SHA256_EXPORTED`; `STAGE041_REAL_CROSS_KERNEL_WAL_ARCHIVE_TAMPER_AND_ID_REPLAY_REJECTED`; `STAGE041_REAL_SECOND_LINUX_KERNEL_POSTGRES_ARCHIVED_WAL_PITR_35_RECORDS_PROVED`.

## Granice tego wyniku

- **Dwa Linux kernele są odrębnymi efemerycznymi środowiskami GitHub Actions**; to nie jest symulacja odcięcia energii fizycznego serwera ani rzeczywista awaria regionalna.
- Zapis przesłano poprzez **prywatne artefakty workflow Github**, a nie niezależny szyfrowany/offsite magazyn z obsługą retencji, automatyczną rotacją kluczy i autentycznymi podpisami PKI.
- SHA-256 wiąże treść raportów i archiwów w obrębie workflow, ale sam **nie poświadcza niezależnej autentyczności nadawcy**. Tożsamość kerneli jest porównywana przez źródło `/proc/sys/kernel/random/boot_id`, bez TPM/TEE remote attestation.
- Brak testu ciągłej utraty danych przy awarii źródła przed fizycznym backupem, brak określonego SLO RTO/RPO dla dużej produkcyjnej bazy danych, brak testów długotrwałego obciążenia i automatycznego międzyhostowego HA.
- Operator upoważnił kontynuację rozwoju bez blokady niezależnymi audytami/podpisami; nie jest to potwierdzenie produkcyjnego bezpieczeństwa.

**Ocenione kryterium 0.41.4 — physical WAL PITR / dwa odrębne Linux kernele: PASS. Pełny etap 0.41: IN PROGRESS. Release: NO-GO.**

## Kod

- `scripts/stage041_pitr.py` — prawdziwy source PG i eksport `basebackup.tar.gz`, `archived-wal.tar.gz` oraz SHA256 manifestu po IMMEDIATE source crash.
- `scripts/stage041_pitr_cross_kernel.py` — stricte walidowany transfer, bezpieczny unpack, rzeczywiste odtworzenie PostgreSQL do docelowego LSN na nowym kernelu.
- `scripts/tests/test_stage041_pitr_cross_kernel.py` — testy pozytywne i negatywne.
- `.github/workflows/stage041-physical-pitr-cross-kernel.yml` — 2 faktyczne oddzielne GitHub-hosted Linux CI kernel jobs.
