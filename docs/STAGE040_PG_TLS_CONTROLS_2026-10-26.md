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

[Rzeczywisty przebieg CI 40282896779](https://github.com/mojealterego/ocaf-runtime/actions/runs/40282896779)
na commicie `8e90ff8a03c92f32643746bb2dfa58838b2611a4` zakończył się
sukcesem. Artefakt `stage040-postgres-tls-role-lab-not-interhost` ma GitHub
artifact ID `12633201847` oraz zgłoszony digest archiwum SHA-256
`fbfe0bde939b39eabe81505f563f8763736c60fa4f0df4c65f7c539f3f8b500c`.
