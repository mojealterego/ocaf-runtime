# Pipeline dowodowy etapów, agentów i rojów

Źródło prawdy dla statusu wykonania stanowi
[`scripts/evidence_pipeline.py`](../scripts/evidence_pipeline.py) oraz
[`FLEET_EVIDENCE_BASELINE_2026-10-21.json`](evidence/FLEET_EVIDENCE_BASELINE_2026-10-21.json).

## Reguła

`COMPLETE_VERIFIED` wymaga jednocześnie:

- commita źródłowego,
- dokładnej komendy build i jej wyniku,
- dokładnej komendy testów i jej wyniku,
- SHA-256 artefaktu,
- identyfikatora receiptu.

Brak jednego pola utrzymuje wpis w `PENDING_EVIDENCE`. Pipeline nie uruchamia
procesu Make i nie tworzy fikcyjnych dowodów.

## Użycie

```sh
python3 scripts/evidence_pipeline.py collect
python3 scripts/evidence_pipeline.py validate
```

Pipeline sprawdza dokładnie 111 agentów i 33 roje oraz odrzuca wpis, który ma
status ukończenia bez pełnego zestawu dowodów.
