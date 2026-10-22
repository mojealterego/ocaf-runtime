# Rejestr 33 kompletnych rojów agentowych AI

**Stan:** `STRUCTURED_NOT_DEPLOYED`. Każdy rój ma własny manifest, skład z osobnych agentów, kontrakt handoff, regułę pobierania następnego etapu, healthcheck, test kontraktu i rekord certyfikacyjny.

## Zasady wspólne

- Rój kończy przypisany etap i dopiero po zweryfikowanym receipcie pobiera następny kwalifikowalny etap.
- Handoff wymaga `stage_id`, commita, testu, wyniku, digestu, receiptu i listy ograniczeń.
- OMAs jest osobnym serwerem agentów; OMAt osobnym serwerem narzędzi; Zenoh transportem.
- OSTc pozostaje wyłączone, a `ACTIVE` jest odrzucane także przy fałszywym readiness.
- Status `STRUCTURED_NOT_DEPLOYED` nie oznacza uruchomionego procesu.

## Roje

| Rój | Specjalizacja | Członkowie | Stan | Healthcheck |
|---|---|---:|---|---|
| `OCAF-SWARM-01` / G01 | `orchestration-runtime` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-02` / G02 | `rust-toolchain` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-03` / G03 | `core-runtime` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-04` / G04 | `rpc-transport` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-05` / G05 | `postgres-ha` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-06` / G06 | `zenoh-messaging` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-07` / G07 | `security-policy` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-08` / G08 | `supply-chain` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-09` / G09 | `sandbox-execution` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-10` / G10 | `agent-ir` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-11` / G11 | `builder-code` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-12` / G12 | `builder-nocode` | 4 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-13` / G13 | `builder-conversion` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-14` / G14 | `cloud-workspace` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-15` / G15 | `desktop-windows` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-16` / G16 | `desktop-macos` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-17` / G17 | `mobile-android` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-18` / G18 | `mobile-ios` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-19` / G19 | `publish-verification` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-20` / G20 | `e2e-platform` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-21` / G21 | `ui-accessibility` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-22` / G22 | `shader-gpu` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-23` / G23 | `ast-cst` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-24` / G24 | `patch-admission` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-25` / G25 | `test-harness` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-26` / G26 | `observability` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-27` / G27 | `data-provenance` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-28` / G28 | `mcp-integrations` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-29` / G29 | `cost-governance` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-30` / G30 | `recovery-chaos` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-31` / G31 | `release-engineering` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-32` / G32 | `independent-review` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |
| `OCAF-SWARM-33` / G33 | `integration-qa` | 3 | `STRUCTURED_NOT_DEPLOYED` | `health/healthcheck.sh` |

Fizyczne pakiety znajdują się w [`SWARM-FLEET-33`](<../../OCAF Agents MOJEALTEREGO/SWARM-FLEET-33/>). Manifest maszynowy: [`swarm_registry_33.json`](swarm_registry_33.json). Dowód strukturalny: [`SWARM_BUILD_EVIDENCE_2026-10-21.md`](SWARM_BUILD_EVIDENCE_2026-10-21.md).
