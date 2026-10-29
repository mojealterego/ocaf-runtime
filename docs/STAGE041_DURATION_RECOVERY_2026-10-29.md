# 0.41 — pomiary długotrwałego odtwarzania PostgreSQL

Workflow `stage041-pitr-duration-soak.yml` uruchamia trzy niezależne
laboratoria trwające co najmniej 30, 60 i 120 minut. W każdym kolejne cykle
wykonują rzeczywisty pg_basebackup, archiwizację WAL, natychmiastowe
zatrzymanie primary i odtworzenie do wskazanego LSN. Nie ma sztucznego
zaliczania czasu ani skracania minut do sekund. Ostatni cykl może wydłużyć
próbę o czas niezbędny do jego ukończenia.

RTO jest mierzone zegarem monotonicznym od żądania zatrzymania primary
przez kopiowanie backupu i start recovery do sprawdzenia liczby i skrótu
odtworzonych danych. Raport zawiera p50/p95/p99 i maksimum. Dotychczasowe
`pitr_restart_duration_ms` nadal mierzy węższy czas startu recovery.

Odtwarzamy wszystkie 35 rekordów wybranego punktu; pięć później zapisanych
rekordów zostaje celowo pominiętych. Nie oznacza to RPO=0 względem awarii.
`rpo_seconds=null`: czasowego RPO wobec ostatniego potwierdzonego zapisu
przed awarią ta próba nie wyznacza.

To kolejne odtwarzania małej bazy, a nie ciągłe obciążenie klientów ani
zintegrowana awaria brokera, Firecracker i PostgreSQL. Nie obejmuje
przeciążenia telemetrii, kwarantanny gościa, niezależnego KMS lub offsite
Object Lock. Każdy raport utrzymuje `stage_041_complete=false` oraz
`production_approved=false`. Testy jednostkowe nie są dowodem ukończenia
przebiegu 30/60/120 minut. Wyniki wykonania należy pobrać z artefaktów CI.

## Decyzja właściciela — 2026-10-29

Właściciel zaakceptował laboratoria i symulację jako podstawę odbioru
projektowego 0.41 z jawnymi ograniczeniami. Status odbioru pozostaje
SIMULATION_ACCEPTANCE_PENDING_RESULTS. Zgoda nie jest wynikiem testu
ani potwierdzeniem rzeczywistego S3/KMS lub zintegrowanego DR.
Pierwszy przebieg 40827723998 zakończył się błędem uprawnień przed
uruchomieniem PostgreSQL; nie stanowi wykonanej próby czasowej.
Poprawka przenosi prywatny katalog użytkownika postgres do /tmp.
