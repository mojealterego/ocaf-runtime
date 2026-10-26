# 0.40.3 — bezpośredni WAL A→B

`scripts/stage040_live_twohost_probe.py` wykonuje minimalną próbę na **dwóch
już uruchomionych hostach Linux**: primary A i standby B. Odczytuje dwa różne
`/proc/sys/kernel/random/boot_id` przez SSH z przypiętymi kluczami hostów,
sprawdza TCP `pg_stat_replication` i `pg_stat_wal_receiver`, zapisuje świeży
nonce SQL na A i czeka, aż B odtworzy dokładnie ten rekord i LSN. Ponownie
sprawdza jądra, role i stan strumieniowania przed utworzeniem raportu.
Raport zawiera również `transport_health`: LSN świeżego zapisu primary,
LSN odebrany i odtworzony przez standby oraz obliczone opóźnienie w bajtach.
Próba odrzuca raport, gdy B twierdzi, że odtworzył WAL, którego jeszcze nie
odebrał, albo jego odbiór nie osiągnął LSN świeżego zapisu. To jest bramka
telemetrii 0.40.7 uruchamiana tylko przy rzeczywistym TCP między hostami.

Wymaga osobnej, jednorazowej bazy `ocaf_stage040_lab`, uprzednio zestawionej
replikacji PostgreSQL, dostępu `sudo -n -u postgres psql` na obu hostach oraz
prywatnego pliku SSH `known_hosts` ze zweryfikowanymi kluczami obu hostów.
Operator uruchamia:

```bash
python3 scripts/stage040_live_twohost_probe.py \
  --inventory /PRIVATE/pg-a-b.json \
  --known-hosts /PRIVATE/known_hosts \
  --report /PRIVATE/stage040-a-b.json
```

Plik inventory ma pola `schema_version=1`, `database=ocaf_stage040_lab`,
`postgres_port`, `postgres_socket_dir` oraz `nodes.a` i `nodes.b` z `role`,
`host`, `ssh_user`, `ssh_port` i `replication_address`. Format jest podzbiorem
[pełnej konfiguracji trzech hostów](STAGE040_REAL_INTERHOST_LIVE_PG_ACCEPTANCE_2026-10-15.md).
Adresy `host` i `replication_address` muszą być różnymi literalnymi IPv4.

Raport `TWO_KERNEL_PG_WAL_STREAMING_OBSERVED_LAB_ONLY` potwierdza tylko tę
konkretną próbę A→B, gdy została uruchomiona na prawdziwych hostach. Testy
jednostkowe sprawdzają odmowę przy tym samym kernelu, brakującym strumieniu,
braku replay, sprzecznym LSN odbioru/replay i nieprzypiętym SSH. Żaden taki test jednostkowy nie stanowi
dowodu istniejącej infrastruktury. Raport zachowuje
`third_host_witness_tested=false`, `fencing_tested=false`,
`stage_040_complete=false` i `production_approved=false`.
