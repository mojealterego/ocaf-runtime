# OCAF-SWARM-20 — kontrakt handoff

## Specjalizacja

`e2e-platform`: cross-platform E2E fixtures and run evidence

## Zasada

claim the next eligible stage after verified completion. Rój może pobrać kolejny etap dopiero po zapisaniu
zweryfikowanego receiptu poprzedniego etapu. Nie wolno przeskakiwać zależności.

## Wymagany receipt

`swarm_id`, `stage_id`, source commit, polecenie testu, wynik testu, digest
artefaktu, `receipt_id`, znane ograniczenia i rollback. Brak pola oznacza
`BLOCKED`.

## Granice

OMAs jest osobnym serwerem agentów, OMAt osobnym serwerem narzędzi, Zenoh
transportem. OSTc jest wyłączone; `ACTIVE` pozostaje odrzucone.
