# Lokalny supervisor floty agentów

`scripts/ocaf_agent_fleet_supervisor.py` ładuje osobne manifesty 111 agentów i
33 rojów, sprawdza członkostwo, nazwy rzeczywistych katalogów pakietów,
kompletność plików kontraktu oraz bramki `OSTc=DISABLED` i `ACTIVE=DENY`.
Konfiguracje nadal mają niewyznaczone końcówki OMAs i OMAt; supervisor nie
testuje ich rzeczywistego rozdzielenia sieciowego. Zapisuje każdy przebieg jako
append-only JSONL.

```sh
./scripts/ocaf_agent_fleet_supervisor.py \
  --log /tmp/ocaf-agent-fleet-supervisor.jsonl
```

Supervisor nie uruchamia modeli, nie przyznaje dostępu do źródeł i nie udaje
aktywacji. Wynik `INVENTORY_VALIDATED` oznacza wyłącznie zgodność 111 wpisów
agentów i 33 manifestów rojów z plikami pakietów oraz bezpiecznymi deklaracjami.
Nie oznacza kompilacji, autonomii AI, transportu Zenoh ani E2E. Aktualny
przebieg raportuje `111` pakietów w stanie DRAFT, `33` roje opisane manifestami
i `0` aktywnych procesów. Osobna bramka `ocaf-agent-artifact-gate` nadal
odrzuca brak wykonawczego kodu dla 33 grup inżynierskich.
