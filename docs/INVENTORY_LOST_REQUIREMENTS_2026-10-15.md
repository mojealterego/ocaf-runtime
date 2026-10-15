# OCAF — audyt odzyskania zagubionych wymagań (2026-10-15)

**Metoda:** porównanie 195+64 etapów i 33 repozytoriów w kanonicznej Roadmapie v10 z rzeczywistą listą 41 plików `src/`, `Cargo.toml`, architekturą 26 dokumentów PDF i bezpośrednim poleceniem operatora o pierwotnym Agents Builder. **Nie utożsamiać PLAN z kodem.** `MISSING` oznacza brak implementacji produktu w obecnym drzewie źródłowym, nie usunięcie prawa do oryginalnego zakresu.

| Nr | Pierwotna potrzeba / luka | Etap historyczny | Stan źródeł | Ustalone zadanie naprawcze |
|---|---|---|---|---|
| 1 | **Agents Builder Cloud Code** — No Cloud Code builder app sources/build evidence | 0.52 | `MISSING` | Write Cloud Code editor, typed IR/project persistence, browser E2E |
| 2 | **Agents Builder Cloud NoCode** — No browser NoCode UI, compiled graph preview | 0.52 | `MISSING` | Build interactive Cloud canvas and graph roundtrip |
| 3 | **Agents Builder Windows Code** — No Tauri Windows product project, binary or CI build | 0.53 | `MISSING` | Separate signed Windows Code IDE and installer test |
| 4 | **Agents Builder Windows NoCode** — No Windows visual NoCode project or GUI E2E | 0.53 | `MISSING` | Separate native Windows visual builder, offline save/reopen |
| 5 | **Agents Builder macOS Code** — No macOS Code Builder product or codesign | 0.53 | `MISSING` | Build macOS Code IDE with reproducible macOS CI and notarization-ready config |
| 6 | **Agents Builder macOS NoCode** — No macOS visual NoCode frontend or UI CI | 0.53 | `MISSING` | Build native macOS graph editor/preview/roundtrip |
| 7 | **Agents Builder Android Code** — No Android Kotlin/Compose application or ARM64 build | 0.54 | `MISSING` | Build mobile Code DSL project editor with UniFFI core and instrumentation |
| 8 | **Agents Builder Android NoCode** — No Android touch NoCode agent designer | 0.54 | `MISSING` | Build offline Compose graph editor with E2E |
| 9 | **Agents Builder iOS Code** — No SwiftUI iOS Code Builder/Xcode target | 0.54 | `MISSING` | Build iOS Code DSL editor, actual iOS simulator smoke |
| 10 | **Agents Builder iOS NoCode** — No SwiftUI iOS NoCode visual editor target | 0.54 | `MISSING` | Build iOS graph canvas and simulator UI roundtrip |
| 11 | **Shared Agents Builder IR, compiler and debugger** — No ocaf-builder-core crate or typed agent-project schema in source tree | 0.50 | `MISSING` | Typed DAG project IR, compiler, agent preview and versioning |
| 12 | **Shared bidirectional Code↔NoCode editor** — No bidirectional interactive IDE/canvas or true round-trip implementation | 0.51 | `MISSING` | Supported DSL Code/visual conversion with lossiness detection |
| 13 | **Agents Builder sandbox, security and publishing** — Runtime has executor code but no product packaging or builder permission UI | 0.55 | `MISSING` | Per-project tool grants, cost limits, preview, signed export and consent |
| 14 | **Specialized Omni* domain swarms** — Roadmap lists OmniDev, OmniUX, OmniE2E, OmniForge, OmniStudio, OmniMath and other swarms; src contains no dedicated Omni implementation | 0.56–0.77 | `PLANNED_UNVERIFIED` | Per-swarm registry, defined real entrypoint, capability manifest and E2E |
| 15 | **True Firecracker broker→guest WASI execution** — Script/CI exists; production and signed workload execution proofs not complete | 0.39 | `ACTIVE_TECHNICAL_OPEN` | Signed WASI module and broker-bound guest exec, actual completion report and tampered module rejection |
| 16 | **PostgreSQL TCP WAL replication across independent kernels** — Separate-runner local labs and three netns one-kernel tests not live cross-host cluster | 0.40 | `ACTIVE_TECHNICAL_OPEN` | Three durable Linux hosts with secure live TCP, witness fencing and stale-primary write denial |
| 17 | **Reliability prolonged soak, real infrastructure RTO/RPO** — Short CI/netns WAL chaos and restore tests are not multi-hour production DR | 0.41 | `ACTIVE_TECHNICAL_OPEN` | 30/60/120-minute soak, host power loss, trusted offsite restore, loss analysis |
| 18 | **Dynamic third-party MCP and OAuth user consent** — Runtime registry and HMAC transport exist; full authenticated remote MCP discovery and OAuth2.1 scope consent not verified | 0.43–0.44 | `PLANNED_UNVERIFIED` | Strict MCP lifecycle, OAuth PKCE and connector isolation test |
| 19 | **CRDT memory + OpenTelemetry actual exports** — Single runtime crate contains no Automerge/Yrs/OTLP collector integration | 0.46–0.48 | `PLANNED_UNVERIFIED` | CRDT convergence, OTLP exporter real collector and privacy filter |
| 20 | **Domain-specific OmniMath Lean/SMT verified checker** — Roadmap describes formal reasoning modules; src tree does not contain real Lean/SMT verifier | 0.78–0.89 | `PLANNED_UNVERIFIED` | Independent checker, typed proofs and real solver E2E |
| 21 | **Historical milestone specifications 0.00–0.10** — Current repository does not carry original titles/contracts; numbering only | 0.00–0.10 | `UNKNOWN_SOURCE` | Recover old commits/specs before filling metadata or claiming Done |
| 22 | **Original stage details 0.11–0.35** — README historic headings visible; source-era test provenance still needs reconciliation | 0.11–0.35 | `HISTORY_PARTIAL` | Link each historical commit/tests to acceptance contract; mark partial until verified |
| 23 | **Six Sovereign Workspace crates** — v9 documents desired workspace crates; actual root Cargo.toml declares a single ocaf_runtime crate | 1.55–1.94 | `PLANNED_UNVERIFIED` | Create incremental cargo workspace and package CI without dropping current runtime |
| 24 | **Repository 33 reference integrations** — 33 external repository names preserved in inventory but do not prove adapters are implemented | 0.43–1.94 | `REFERENCE_ONLY` | For each source pin license+commit, test actual adapter, add real evidence and status |

## Reguła inwentaryzacji

- **Agents Builder Code i NoCode to 10 osobnych produktów/platform-mode, nie agentów.** Ich odkrycie naprawia rozpisanie zakresu 0.50–0.55, nie jest dodaniem fikcyjnych nowych funkcji.
- **Etapy 0.00–0.10**: nie odtwarzać nazw na podstawie samej liczby; brak starych dokumentów wskazuje `UNKNOWN_SOURCE`.
- **Pozostałe pozycje** z 26 PDF, 33 repo i v7–v9 zachowujemy w kanonicznej Roadmapie. Każde nowe odkrycie dopisuje się do `docs/roadmap/STAGE_CHANGELOG.md` jako `RESTORED_EXISTING`, chyba że jest rzeczywiście nowym wymaganiem: wtedy nowy numer **1.95+**, bez ruszania historycznej kolejności.
- Statusy po zamknięciu **każdego** etapu wynikają z wykonania kodu i rzeczywistych dowodów. Założenia o architekturze są nadal planem.
