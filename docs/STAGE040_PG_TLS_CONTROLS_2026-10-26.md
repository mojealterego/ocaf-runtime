# 0.40.10 — oddzielne role PostgreSQL i TLS 1.3

`scripts/stage040_pg_tls_lab.py` uruchamia izolowany klaster PostgreSQL 16
na jednym runnerze Linux. Tworzy osobne role `ocaf_admin` oraz `ocaf_repl`,
osobne certyfikaty klientów i prywatną CA laboratoryjną. TCP wymaga TLS 1.3;
`pg_hba.conf` nie zawiera reguły TCP `trust`, hasła ani `md5`.

Brama pozytywna wymaga połączenia administracyjnego z TLS 1.3 i weryfikacją
certyfikatu serwera oraz rzeczywistego `pg_basebackup` uwierzytelnionego
certyfikatem roli replikacyjnej. Role nie mają `SUPERUSER`, `CREATEDB` ani
`CREATEROLE`.

Brama negatywna odrzuca połączenie bez TLS, obcą CA, certyfikat klienta z
inną nazwą, administracyjne `CREATE ROLE`, zapytanie SQL roli replikacyjnej
oraz próbę wykonania `pg_basebackup` przez rolę administracyjną. Skrypt
zatrzymuje serwer i usuwa prywatne klucze po próbie. Workflow
`.github/workflows/stage040-postgres-tls-controls.yml` publikuje raport z
wynikami, bez kluczy prywatnych.

To test laboratoryjny na **jednym jądrze** z efemerycznymi certyfikatami.
Nie dowodzi TLS między niezależnymi hostami ani hybrydy PQC. Raport zachowuje
`independent_host_tls_tested=false`, `pqc_hybrid_tested=false`,
`stage_040_complete=false` i `production_approved=false`.
