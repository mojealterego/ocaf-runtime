# Rejestr 33 rojów i 111 osobnych agentów AI

**Stan:** `STRUCTURED_NOT_DEPLOYED`. To jest flota 111 odrębnych pakietów agentowych, nie lista ról. Każdy agent ma własny katalog, `agent_id`, kod źródłowy, manifest, konfigurację modelu, namespace pamięci, politykę narzędzi, test, dokumentację, kontrakt instalatora i rekord certyfikacyjny.

## Bramka architektury

- OMAs: osobny serwer agentów.
- OMAt: osobny serwer narzędzi.
- Zenoh: warstwa transportowa.
- OSTc: przyszłe szyfrowanie źródeł; obecnie `DISABLED`.
- Każdy agent ma własną bramkę lifecycle; `ACTIVE=DENY` przy wyłączonym OSTc i fałszywym readiness.

## Liczność

| Wskaźnik | Wartość |
|---|---:|
| Osobne pakiety agentów | **111** |
| Roje | **33** |
| Aktywne procesy runtime | **0** |
| Referencyjny pakiet staged | **1** |

## Struktura pojedynczego agenta

Każdy katalog `AGENT-FLEET-111/OCAF-AGENT-NNN/` zawiera:

- `Cargo.toml`, `src/main.rs`, `src/lib.rs` — własny punkt wejścia i politykę;
- `config/agent.toml`, `config/autonomy.toml`, `config/tool_policy.toml`;
- `docs/README.md`, `docs/ARCHITECTURE.md`, `docs/USER_MANUAL.md`;
- `tests/security.rs`;
- `packaging/install.sh`, `packaging/uninstall.sh`, `packaging/INSTALLER_CONTRACT.md`, `packaging/BUILD_EVIDENCE.md`;
- `certification/CERTIFICATE_RECORD.md` ze statusem `NOT ISSUED`.

## Rejestr agentów

| Agent ID | Rój | Specjalizacja | Stan | OSTc | ACTIVE |
|---|---|---|---|---|---|
| `OCAF-AGENT-001` | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-002` | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-003` | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-004` | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-005` | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-006` | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-007` | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-008` | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-009` | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-010` | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-011` | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-012` | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-013` | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-014` | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-015` | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-016` | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-017` | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-018` | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-019` | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-020` | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-021` | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-022` | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-023` | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-024` | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-025` | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-026` | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-027` | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-028` | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-029` | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-030` | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-031` | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-032` | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-033` | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-034` | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-035` | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-036` | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-037` | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-038` | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-039` | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-040` | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-041` | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-042` | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-043` | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-044` | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-045` | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-046` | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-047` | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-048` | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-049` | G13 | `builder-conversion` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-050` | G13 | `builder-conversion` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-051` | G13 | `builder-conversion` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-052` | G14 | `cloud-workspace` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-053` | G14 | `cloud-workspace` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-054` | G14 | `cloud-workspace` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-055` | G15 | `desktop-windows` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-056` | G15 | `desktop-windows` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-057` | G15 | `desktop-windows` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-058` | G16 | `desktop-macos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-059` | G16 | `desktop-macos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-060` | G16 | `desktop-macos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-061` | G17 | `mobile-android` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-062` | G17 | `mobile-android` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-063` | G17 | `mobile-android` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-064` | G18 | `mobile-ios` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-065` | G18 | `mobile-ios` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-066` | G18 | `mobile-ios` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-067` | G19 | `publish-verification` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-068` | G19 | `publish-verification` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-069` | G19 | `publish-verification` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-070` | G20 | `e2e-platform` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-071` | G20 | `e2e-platform` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-072` | G20 | `e2e-platform` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-073` | G21 | `ui-accessibility` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-074` | G21 | `ui-accessibility` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-075` | G21 | `ui-accessibility` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-076` | G22 | `shader-gpu` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-077` | G22 | `shader-gpu` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-078` | G22 | `shader-gpu` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-079` | G23 | `ast-cst` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-080` | G23 | `ast-cst` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-081` | G23 | `ast-cst` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-082` | G24 | `patch-admission` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-083` | G24 | `patch-admission` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-084` | G24 | `patch-admission` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-085` | G25 | `test-harness` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-086` | G25 | `test-harness` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-087` | G25 | `test-harness` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-088` | G26 | `observability` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-089` | G26 | `observability` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-090` | G26 | `observability` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-091` | G27 | `data-provenance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-092` | G27 | `data-provenance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-093` | G27 | `data-provenance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-094` | G28 | `mcp-integrations` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-095` | G28 | `mcp-integrations` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-096` | G28 | `mcp-integrations` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-097` | G29 | `cost-governance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-098` | G29 | `cost-governance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-099` | G29 | `cost-governance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-100` | G30 | `recovery-chaos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-101` | G30 | `recovery-chaos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-102` | G30 | `recovery-chaos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-103` | G31 | `release-engineering` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-104` | G31 | `release-engineering` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-105` | G31 | `release-engineering` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-106` | G32 | `independent-review` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-107` | G32 | `independent-review` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-108` | G32 | `independent-review` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-109` | G33 | `integration-qa` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-110` | G33 | `integration-qa` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-111` | G33 | `integration-qa` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |

Pełny rejestr maszynowy znajduje się w [`agent_registry_111.json`](agent_registry_111.json), a fizyczne pakiety w [`AGENT-FLEET-111`](<../../OCAF Agents MOJEALTEREGO/AGENT-FLEET-111/>). Status `NOT_DEPLOYED` oznacza brak uruchomionego procesu, a nie brak struktury pakietu.
