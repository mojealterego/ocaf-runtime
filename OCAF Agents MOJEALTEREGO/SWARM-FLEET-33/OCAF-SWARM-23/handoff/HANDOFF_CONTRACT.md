# OCAF-SWARM-23 — kontrakt handoff

## Specjalizacja

`ast-cst`: AST/CST repository analysis and patch structure

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
