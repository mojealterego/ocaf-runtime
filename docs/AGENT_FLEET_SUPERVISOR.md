# Lokalny supervisor floty agentów

`scripts/ocaf_agent_fleet_supervisor.py` ładuje osobne manifesty 111 agentów i
33 rojów, sprawdza członkostwo, kompletność katalogów, rozdzielenie OMAs/OMAt
oraz bramki `OSTc=DISABLED` i `ACTIVE=DENY`. Zapisuje każdy przebieg jako
append-only JSONL.

```sh
./scripts/ocaf_agent_fleet_supervisor.py \
  --log /tmp/ocaf-agent-fleet-supervisor.jsonl
```

Supervisor nie uruchamia modeli, nie przyznaje dostępu do źródeł i nie udaje
aktywacji. Wynik `PASS` oznacza poprawność kontraktów i manifestów. Aktualny
przebieg raportuje `111` agentów, `33` roje i `0` aktywnych procesów.
