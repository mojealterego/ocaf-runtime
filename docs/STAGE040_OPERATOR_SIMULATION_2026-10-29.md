# 0.40 — odbiór projektowy na podstawie symulacji

Właściciel 2026-10-29 polecił oprzeć etap na symulacji ze względu na brak
trzech dostępnych hostów. Jest to zgoda na zastępczą metodę odbioru,
nie dowód wykonania testów ani podpis kryptograficzny właściciela.

Status: SIMULATION_ACCEPTANCE_PENDING_RESULTS.

Próba zastępcza obejmuje:

1. Trzy rzeczywiste procesy PostgreSQL w oddzielnych przestrzeniach sieciowych
   jednego kernela: TCP WAL, odcięcie B przy zdrowym C, 3600 sekund pgbench
   z ośmioma klientami i docelową stawką 100 transakcji/s, naprawa partycji,
   zgodność danych, awaria A, promocja B i przepięcie C.
2. Osobne laboratorium świadka Ed25519: wymagane quorum przed SQL,
   odmowa przy mniejszości i przy starej epoce. To kontrola adaptera;
   nie stanowi blokady dowolnych bezpośrednich klientów SQL.
3. Osobne laboratorium PostgreSQL HA z lokalnym świadkiem, pg_rewind
   i powrotem starego primary jako repliki.

Przebiegi są osobnymi laboratoriami, nie jednym rozproszonym klastrem.
Nie dowodzą awarii fizycznych hostów, rozproszonego quorum ani pełnego
fencingu PostgreSQL. Zachować stage_040_complete=false i
production_approved=false w dowodach technicznych. Status administracyjny
można zamknąć dopiero po dołączeniu udanych wyników tych prób.

Workflow stage040-real-postgres-netns.yml przy workflow_dispatch przyjmuje
soak_seconds=3600; zwykłe pushe zachowują krótki test 20 sekund.
Limit joba wynosi 80 minut i obejmuje instalację, obciążenie oraz recovery.
