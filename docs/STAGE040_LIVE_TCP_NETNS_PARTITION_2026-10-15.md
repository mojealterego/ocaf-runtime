# OCAF 0.40.6 — bezpośredni PostgreSQL WAL streaming TCP przez trzy realne Linux netns

**Data:** 2026-10-15; **gałąź:** `main`. **Rzeczywisty test:** [GitHub Actions 38752910618 — SUCCESS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38752910618), [artefakt 11981069593](https://github.com/mojealterego/ocaf-runtime/actions/runs/38752910618/artifacts/11981069593), SHA256 ZIP: `31c59e1c42fb3c9d5205aec94368ed50519b81f32913eb8b3345b97e1f9f65dc`.

## Rzeczywisty przebieg

1. Jądro Linux uruchomiło trzy osobne *network namespaces*, trzy pary veth w izolowanym bridge, bez tras domyślnych ani publicznego dostępu do sieci.
2. Każdy netns uruchomił **prawdziwy proces PostgreSQL 16** z oddzielną bazą danych, socket lock dir i adresem `10.247.40.1`, `.2`, `.3`. Wszystkie użyły **tego samego portu TCP** w osobnych stosach sieciowych.
3. Standby B i C wykonały `pg_basebackup --wal-method=stream --write-recovery-conf` po prawdziwym TCP do primary A; oba `pg_stat_wal_receiver.status` były `streaming`; `pg_stat_replication.client_addr` na A wykazał dwa odrębne prywatne IP.
4. W kernel namespace B dodano rzeczywistą trasę `blackhole 10.247.40.1/32`. Primary A wykonał nową transakcję, C odtworzył ją, B nie; brak zdalnej replikacji B przez podmienione dane/syntetyczny raport.
5. Usunięto trasę `blackhole`; po ponownym połączeniu TCP B odtworzył WAL.
6. Primary A zatrzymano `pg_ctl stop -m immediate`, B promowano na primary; C skierowano do `10.247.40.2` i zweryfikowano stan `streaming` oraz nowo zapisany rekord.

**Regresja Python: 3/3 PASS**, obejmuje fałszywą deklarację niezależnych hostów i brak obserwacji. Znacznik CI: `STAGE040_ACTUAL_THREE_POSTGRESQL_KERNEL_NETNS_TCP_WAL_PARTITION_PROMOTION_PROVED`.

## Ograniczenia

- **Wszystkie trzy netns współdzielą JEDNO jądro systemu Linux**, choć używają oddzielnych stosów TCP i faktycznie rozdzielonych adresów IP; nie są oddzielnymi serwerami fizycznymi ani niezależnymi VM-kernel.
- `pg_stat_replication` w ramach jednej fizycznej maszyny nie dowodzi szyfrowanego WAN mTLS, wielostrefowej HA, quorum/witness leader fencing ani awarii całego hosta.
- PostgreSQL w eksperymencie konfigurowano z `trust` tylko wewnątrz efemerycznego prywatnego bridge; **NIE WOLNO** kopiować ustawień HBA `trust` do produkcji.
- Wciąż otwarte `0.40.3` i `0.40.4`: prawdziwe dwa/trzy różne `/proc/sys/kernel/random/boot_id` z **żywą replikacją TCP między nimi**, wiarygodna blokada nieaktualnego leadera i automatyczne przełączenie po utracie węzła.

**0.40.6 LAB PASSED. 0.40 INFRASTRUCTURE NOT COMPLETE, production_approved=false.**
