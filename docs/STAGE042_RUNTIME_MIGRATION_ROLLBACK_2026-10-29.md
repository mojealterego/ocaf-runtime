# 0.42 — rzeczywista migracja i odtworzenie stanu kolejki runtime

Workflow `stage042-stateful-migration-rollback.yml` używa osobnego,
jednorazowego PostgreSQL 16 i wywołuje rzeczywiste API `PgJobQueue`.

1. Tworzy historyczny schemat kolejki bez `cluster_id` i `leader_epoch`,
   zapisuje zadanie pending i robi `pg_dump -Fc` przed migracją.
2. `PgJobQueue::connect` wykonuje migracje z kodu runtime. Test sprawdza
   domyślne wartości obu nowych kolumn, właściciela i stan zadania.
3. Runtime przydziela zadanie, zapisuje wynik, zamyka połączenie i odczytuje
   wynik po ponownym połączeniu. Obcy właściciel nie widzi zadania, a
   zakończone zadanie nie jest ponownie przydzielane.
4. `pg_restore` odtwarza stary schemat i dane. Test odrzuca pozorne
   odtworzenie pozostawiające nowe kolumny lub ukończony stan zadania.
   Ponownie uruchamia rzeczywistą migrację i wykonanie przez runtime.
5. Po zmianie hasła stary sekret nie pozwala połączyć się przez
   `PgJobQueue`; nowy pozwala odczytać utrwalony wynik.

Workflow wymaga ustawionej fazy i dedykowanej nazwy bazy. Zwykłe uruchomienie
całego zestawu testów bez środowiska PostgreSQL pomija tę próbę i nie jest
jej dowodem. Baza oraz hasła w workflow są jednorazowymi danymi testowymi.

Raport wiąże wynik z SHA źródła, numerem przebiegu i podejścia. Ten test nie
uruchamia poprzedniej podpisanej binarki; odpowiada za to odrębny workflow
`stage042-rollback-drill.yml`, który wymaga teraz zgodności SHA checkoutu
z SHA kandydata. Test nie dowodzi rollbacku całego rozproszonego runtime,
zewnętrznych efektów ani innych magazynów sekretów/KMS. Odtworzenie
snapshotu celowo cofa późniejszy wynik zadania i wymaga zatrzymania ruchu
w rzeczywistym wdrożeniu. `stage_042_complete=false` i
`production_approved=false` pozostają w raporcie.

Walidacja przygotowania: test Rust skompilowany z `--locked` i
`--no-default-features`; formatowanie i `git diff --check` zaliczone.
Lokalna próba pobrania obrazu PostgreSQL została zatrzymana przez brak
miejsca w magazynie demona Docker. Nie jest to wynik testu PostgreSQL;
wynik wykonania należy odczytać z CI po integracji zmiany.
