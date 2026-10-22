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

| Agent ID | Nazwa oryginalna | Rój | Specjalizacja | Stan | OSTc | ACTIVE |
|---|---|---|---|---|---|---|
| `OCAF-AGENT-001` | **Asterion** | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-002` | **Mnemosyne** | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-003` | **Helios** | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-004` | **Kestrel** | G01 | `orchestration-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-005` | **Ferrum** | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-006` | **Rustica** | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-007` | **Boron** | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-008` | **Forge** | G02 | `rust-toolchain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-009` | **Kernelis** | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-010` | **Quanta** | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-011` | **Vectra** | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-012` | **Nucleus** | G03 | `core-runtime` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-013` | **Relay** | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-014` | **Packet** | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-015` | **Cipher** | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-016` | **Lattice** | G04 | `rpc-transport` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-017` | **Walden** | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-018` | **Patron** | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-019` | **Quorum** | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-020` | **Bastion** | G05 | `postgres-ha` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-021` | **Zenobia** | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-022` | **Shard** | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-023` | **Tidemark** | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-024` | **Chorus** | G06 | `zenoh-messaging` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-025` | **Sentinel** | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-026` | **Aegis** | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-027` | **Vigil** | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-028` | **Lockstep** | G07 | `security-policy` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-029` | **Sable** | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-030` | **Provenance** | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-031` | **Ledger** | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-032` | **Chainwatch** | G08 | `supply-chain` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-033` | **Sandbox** | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-034` | **Jailer** | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-035` | **Isolator** | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-036` | **Firebreak** | G09 | `sandbox-execution` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-037` | **Intermediate** | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-038` | **Syntax** | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-039` | **Semira** | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-040` | **Graphite** | G10 | `agent-ir` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-041` | **Coderis** | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-042` | **Compiler** | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-043` | **Byteforge** | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-044` | **Polyglot** | G11 | `builder-code` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-045` | **Nocturne** | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-046` | **Canvas** | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-047` | **Flowcraft** | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-048` | **Pixel** | G12 | `builder-nocode` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-049` | **Transmute** | G13 | `builder-conversion` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-050` | **Bridge** | G13 | `builder-conversion` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-051` | **Alchemy** | G13 | `builder-conversion` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-052` | **Cloudsmith** | G14 | `cloud-workspace` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-053` | **Nimbus** | G14 | `cloud-workspace` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-054` | **Workspace** | G14 | `cloud-workspace` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-055` | **Redmond** | G15 | `desktop-windows` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-056` | **Glasswing** | G15 | `desktop-windows` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-057` | **Desktopa** | G15 | `desktop-windows` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-058` | **Quartz** | G16 | `desktop-macos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-059` | **Sequoia** | G16 | `desktop-macos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-060` | **Parallax** | G16 | `desktop-macos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-061` | **Droida** | G17 | `mobile-android` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-062` | **Orbit** | G17 | `mobile-android` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-063` | **Material** | G17 | `mobile-android` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-064` | **Swiftlet** | G18 | `mobile-ios` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-065` | **Touchline** | G18 | `mobile-ios` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-066` | **Cupertino** | G18 | `mobile-ios` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-067` | **Herald** | G19 | `publish-verification` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-068` | **Veritas** | G19 | `publish-verification` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-069` | **Stamp** | G19 | `publish-verification` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-070` | **Odyssey** | G20 | `e2e-platform` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-071` | **Endline** | G20 | `e2e-platform` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-072` | **Pathfinder** | G20 | `e2e-platform` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-073` | **Inclusive** | G21 | `ui-accessibility` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-074` | **Lumen** | G21 | `ui-accessibility` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-075` | **Aria** | G21 | `ui-accessibility` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-076` | **Shader** | G22 | `shader-gpu` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-077` | **Flux** | G22 | `shader-gpu` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-078` | **Raster** | G22 | `shader-gpu` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-079` | **Syntaxa** | G23 | `ast-cst` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-080` | **Parselet** | G23 | `ast-cst` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-081` | **Treefold** | G23 | `ast-cst` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-082` | **Gatekeeper** | G24 | `patch-admission` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-083` | **Diffra** | G24 | `patch-admission` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-084` | **Mergeguard** | G24 | `patch-admission` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-085` | **Specimen** | G25 | `test-harness` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-086` | **Oracle** | G25 | `test-harness` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-087` | **Regressor** | G25 | `test-harness` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-088` | **Telemetry** | G26 | `observability` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-089` | **Tracer** | G26 | `observability` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-090` | **Watchtower** | G26 | `observability` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-091` | **Origin** | G27 | `data-provenance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-092` | **Attestor** | G27 | `data-provenance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-093` | **Lineage** | G27 | `data-provenance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-094` | **Toolsmith** | G28 | `mcp-integrations` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-095` | **Connector** | G28 | `mcp-integrations` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-096` | **Contextor** | G28 | `mcp-integrations` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-097` | **Finops** | G29 | `cost-governance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-098` | **Meter** | G29 | `cost-governance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-099` | **Thrift** | G29 | `cost-governance` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-100` | **Phoenix** | G30 | `recovery-chaos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-101` | **Chaos** | G30 | `recovery-chaos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-102` | **Resilience** | G30 | `recovery-chaos` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-103` | **Semver** | G31 | `release-engineering` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-104` | **Signer** | G31 | `release-engineering` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-105` | **Shipwright** | G31 | `release-engineering` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-106` | **Critic** | G32 | `independent-review` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-107` | **Adjudicator** | G32 | `independent-review` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-108` | **Redteam** | G32 | `independent-review` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-109` | **Verifier** | G33 | `integration-qa` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-110` | **Acceptance** | G33 | `integration-qa` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
| `OCAF-AGENT-111` | **Integrator** | G33 | `integration-qa` | `NOT_DEPLOYED` | `DISABLED` | `DENY` |
