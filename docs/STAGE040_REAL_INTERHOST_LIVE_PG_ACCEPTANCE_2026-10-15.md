# OCAF 0.40.3 — live PostgreSQL WAL replication między trzema niezależnymi hostami

**Status:** implementacja modułu testowego **GOTOWA**, prawdziwy test **NOT RUN** bez trzech dostępnych hostów z uprawnieniami. `stage_040_complete=false`. Nie należy traktować [wcześniejszego netns TCP](STAGE040_LIVE_TCP_NETNS_PARTITION_2026-10-15.md) ani trzech osobnych host-local labów jako bezpośredniej replikacji międzysystemowej.

## Odtwarzalny mechanizm

Moduł `scripts/stage040_interhost_live.py` po autoryzacji łączy się do **A/B/C przez SSH z kontrolą kluczy serwera**. Sprawdza rzeczywiste `/proc/sys/kernel/random/boot_id` każdej maszyny, źródłowe `pg_is_in_recovery=false`, oba standby w recovery, oraz rzeczywiste PostgreSQL `pg_stat_replication` wskazujące **dwa inne adresy klientów TCP**. Odbiorcy B/C muszą zgłaszać `pg_stat_wal_receiver.status=streaming`. Dopiero potem na pierwotnym A tworzony jest tymczasowy testowy rekord z nowym kryptograficznym nonce (w **dedykowanej** bazie `ocaf_stage040_lab`) i mierzony rzeczywisty WAL LSN. B/C muszą odtworzyć dokładny nonce, source boot ID i LSN przez PostgreSQL streaming, a nie z GitHub artifact.

Weryfikator zabrania localhost, pokrywających się kernel `boot_id`, niespójnych adresów, nieprzypiętych SSH kluczy, nierównych nonce, braku replikacji i nieuprawnionych stwierdzeń o świadku/quorum lub produkcji. Kontrakt zawiera także `consensus_fencing_verified=false` i `physical_host_power_off_tested=false`, nawet po sukcesie transmisji.

## Uruchomienie na trzech rzeczywistych hostach (bez tworzenia płatnej infrastruktury)

Warunki konieczne:
- Trzy istniejące, **odrębne Linux kernels** z rzeczywistym TCP między serwerami PostgreSQL A→B/C i uprawnieniami do odczytu i zapisu w **wyizolowanej** bazie `ocaf_stage040_lab`.
- Dedykowany operator SSH na hostach, jawnie zatwierdzony dostęp `sudo -n -u postgres psql`, uprzednio weryfikowane i **przypięte** publiczne klucze SSH serwerów w pliku `known_hosts`, uprawnienia `0600`.
- B/C są skonfigurowane jako streaming standby PostgreSQL i ich **adresy źródłowe replikacji** są widoczne z A bez translacji NAT.
- Zabezpieczona przed nieuprawnioną modyfikacją konfiguracja JSON:

```json
{
  "schema_version":1,
  "database":"ocaf_stage040_lab",
  "postgres_port":5432,
  "postgres_socket_dir":"/var/run/postgresql",
  "nodes":{
    "a":{"role":"a","host":"10.60.0.1","ssh_user":"pgops","ssh_port":22,"replication_address":"10.60.0.1"},
    "b":{"role":"b","host":"10.60.0.2","ssh_user":"pgops","ssh_port":22,"replication_address":"10.60.0.2"},
    "c":{"role":"c","host":"10.60.0.3","ssh_user":"pgops","ssh_port":22,"replication_address":"10.60.0.3"}
  }
}
```

Adresy powyżej to przykłady RFC1918; **nie** oznaczają istniejących maszyn ani uwierzytelnionej infrastruktury.

Test w CI ma dwa joby: automatyczny **fail-closed unit gate** na zwykłym bezpłatnym GitHub runnerze oraz **rzeczywisty job `workflow_dispatch`** kierowany wyłącznie do autoryzowanego, zarejestrowanego runnera `[self-hosted, linux, ocaf-pg-operator]`. Job ten nie rozpocznie się, dopóki operator nie posiada takiej infrastruktury i plików `OCAF_PG_INTERHOST_INVENTORY`, `OCAF_PG_INTERHOST_KNOWN_HOSTS` na tym runnerze.

**Brak operatorowego skoku na nieznane hosty.** Nigdzie nie jest przechowywany prywatny klucz SSH ani hasło PostgreSQL. Test nie przeprowadza promocji: do ukończenia rzeczywistego HA pozostają trzeci świadek, odmowa zapisu na starym primary podczas podziału sieci, quorum/fencing i wielogodzinny soak.
