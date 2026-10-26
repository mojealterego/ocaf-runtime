# 0.40.11 — żywy WAL a przekazanie artefaktu

`scripts/stage040_wal_evidence_classifier.py` rozdziela trzy typy raportów:
trzy lokalne failovery PostgreSQL na niezależnych runnerach połączone przez
artefakt, bezpośredni A→B TCP WAL oraz bezpośredni A→B/C TCP WAL. Pierwszy
typ **nigdy** nie daje wyniku `direct_tcp_wal_claim_in_validated_receipt=true`.

```bash
python3 scripts/stage040_wal_evidence_classifier.py \
  --input /PRIVATE/postgres-receipt.json \
  --report /PRIVATE/wal-classification.json
```

Walidator wymaga spójnych boot ID, ról, obserwacji `pg_stat_wal_receiver`,
nonce i porządku LSN odbioru oraz odtworzenia. Odrzuca podmianę samego statusu
raportu z transferu artefaktu na deklarację żywego TCP. Wynik jest **kontrolą
kontraktu offline**: nie uwierzytelnia źródła raportu ani hostów. Przebieg
realnego operatora wymaga nadal przypiętych kluczy SSH i bezpośredniego testu
SQL na odrębnych kernelach według
[instrukcji 0.40.3](STAGE040_REAL_INTERHOST_LIVE_PG_ACCEPTANCE_2026-10-15.md).

Lokalne testy negatywne: `python3 -m unittest -v
scripts.tests.test_stage040_wal_evidence_classifier`. Status całego 0.40 i
`stage_040_complete` pozostają otwarte.
