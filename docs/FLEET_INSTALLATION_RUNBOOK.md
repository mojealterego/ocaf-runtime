# Instalacja floty agentów i rojów

## Zakres

- `OCAF-FLEET-111`: 111 osobnych pakietów agentów AI.
- `SWARM-FLEET-33`: 33 osobne manifesty rojów i ich handoff.
- OMAs: osobny serwer agentów.
- OMAt: osobny serwer narzędzi.
- Zenoh: transport.
- OSTc: przyszłe szyfrowanie źródeł; w bieżącym wydaniu `DISABLED`.

## Budowa 111 agentów

```sh
OCAF_AGENT_FLEET_ROOT="OCAF Agents MOJEALTEREGO/AGENT-FLEET-111" \
  OCAF_AGENT_FLEET_OUT="$PWD/dist/agent-fleet-111" \
  ./scripts/build_agent_fleet_111.sh
```

Skrypt wymaga realnego `cargo`, buduje każdy manifest osobno, zapisuje log
każdego agenta i tworzy SHA-256. Przerwanie jednego builda kończy cały przebieg;
nie ma częściowego oznaczenia floty jako gotowej. Bez `cargo` nie powstają
artefakty.

## Instalacja po weryfikacji

```sh
OCAF_AGENT_PREFIX="$HOME/.local" ./scripts/install_agent_fleet_111.sh
```

Instalator najpierw sprawdza wszystkie digesty, następnie wykonuje instalację
transakcyjną. Poprzedni pakiet jest zachowywany jako
`ocaf-agent-fleet-111.rollback`.

## Instalacja 33 rojów

```sh
OCAF_SWARM_FLEET_ROOT="OCAF Agents MOJEALTEREGO/SWARM-FLEET-33" \
  OCAF_AGENT_PREFIX="$HOME/.local" ./scripts/install_swarm_fleet_33.sh
```

Skrypt wymaga dokładnie 33 manifestów rojów, kopiuje je do osobnego katalogu i
zachowuje poprzednią wersję do rollbacku. Manifest nie uruchamia procesu i nie
nadaje uprawnień.

## Warunki aktywacji

Każdy agent i rój musi mieć własny manifest, build, testy, digest, podpis,
rejestrację OMAs, allowlistę OMAt, dowód rollbacku i test negatywny. Przy
`OSTc=DISABLED` dostęp do źródeł jest wyłączony, a przejście `ACTIVE` jest
odrzucane także przy fałszywych potwierdzeniach gotowości.

Nie wolno tworzyć zastępczych MSI, DMG, PKG, APK, AAB ani IPA. Produkty
platformowe wymagają właściwych runnerów, podpisów i testów E2E.
